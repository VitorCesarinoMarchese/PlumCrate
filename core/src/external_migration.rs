use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ErrorCode, ManagerError};
use crate::managed_storage::{create_managed_mod_transaction, ImportRequest};
use crate::mod_scan::{scan_mods, ModSource};
use crate::toggle::IssueEvent;
use crate::{fs_scope, link_ownership, operation};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MigrateResult {
    pub managed_mod_id: String,
    pub issues: Vec<IssueEvent>,
}

pub fn migrate_external_mod(
    managed_root: &Path,
    game_mods_dir: &Path,
    external_mod_key: &str,
) -> Result<MigrateResult, ManagerError> {
    let _guard = operation::acquire(managed_root)?;
    fs_scope::component(external_mod_key)?;
    let scanned = scan_mods(game_mods_dir, managed_root)?;
    let mut matching = scanned.into_iter().filter(|entry| entry.key == external_mod_key).peekable();
    if matching.peek().is_none() {
        return Err(ManagerError::new(ErrorCode::NotFound, format!("External mod not found: {external_mod_key}")));
    }
    let external = matching.find(|entry| entry.source == ModSource::External).ok_or_else(|| {
        ManagerError::new(ErrorCode::ExternalLink, format!("Mod is not external: {external_mod_key}"))
    })?;

    let temporary = managed_root.join("tmp");
    fs_scope::create_dir_all(&temporary)?;
    let staging = tempfile::Builder::new()
        .prefix("migrate-")
        .tempdir_in(temporary)
        .map_err(fs_scope::io_error)?;

    copy_external_contents(game_mods_dir, staging.path(), &external.files)?;

    let mut transaction = operation::Transaction::new(managed_root, "external_migration")?;
    let result = (|| {
        let imported = create_managed_mod_transaction(
            managed_root,
            ImportRequest {
                name: external.name,
                slug: None,
                source_dir: staging.path().to_owned(),
            },
            &mut transaction,
        )?;
        replace_live_files_with_managed_links(
            game_mods_dir,
            managed_root,
            &imported.mod_id,
            &external.files,
            &mut transaction,
        )?;
        Ok(imported)
    })();
    let imported = match result {
        Ok(imported) => imported,
        Err(error) => {
            transaction.rollback()?;
            return Err(error);
        }
    };
    transaction.commit()?;

    Ok(MigrateResult {
        managed_mod_id: imported.mod_id,
        issues: vec![IssueEvent {
            id: format!("migrated:{}", external_mod_key),
            severity: "info".to_string(),
            message: "External mod migrated to managed storage".to_string(),
            code: None,
        }],
    })
}

fn replace_live_files_with_managed_links(
    game_mods_dir: &Path,
    managed_root: &Path,
    mod_id: &str,
    files: &[String],
    transaction: &mut operation::Transaction,
) -> Result<(), ManagerError> {
    let instance = fs::canonicalize(game_mods_dir).map_err(fs_scope::io_error)?;
    let backup = instance
        .parent()
        .ok_or_else(|| ManagerError::new(ErrorCode::InvalidPath, "Mods root has no parent"))?
        .join(format!(".ts4-originals-{}", Uuid::new_v4()));
    // Backups on the game filesystem preserve the original symlink object through rename.
    fs_scope::create_dir_all(&backup)?;
    let result = (|| {
        for rel in files {
            fs_scope::ensure_no_symlink_parents(&instance, Path::new(rel))?;
            let live = instance.join(rel);
            let preserved = backup.join(rel);
            fs_scope::create_dir_all(preserved.parent().unwrap())?;
            transaction.move_path(&live, &preserved)?;
            let target = link_ownership::target(managed_root, mod_id, rel)?;
            transaction.create_link(&instance, Path::new(rel), &target)?;
        }
        link_ownership::write_transactional(
            managed_root,
            mod_id,
            &instance,
            files.to_vec(),
            transaction,
        )?;
        // An index records where originals remain, without copying or retiring them.
        fs_scope::create_dir_all(&managed_root.join("backups"))?;
        transaction.write_file(
            &managed_root.join("backups").join(format!("{mod_id}.json")),
            &serde_json::to_vec(
                &serde_json::json!({"version":1,"modId":mod_id,"originals":backup}),
            )
            .map_err(|e| ManagerError::new(ErrorCode::InternalError, e.to_string()))?,
        )?;
        Ok(())
    })();
    result
}

fn copy_external_contents(
    game_mods_dir: &Path,
    staging: &Path,
    files: &[String],
) -> Result<(), ManagerError> {
    for rel in files {
        fs_scope::ensure_no_symlink_parents(game_mods_dir, Path::new(rel))?;
        let in_game = game_mods_dir.join(rel);
        let target = resolve_source_file(&in_game)?;
        let out = staging.join(rel);

        if let Some(parent) = out.parent() {
            fs_scope::create_dir_all(parent)?;
        }

        let mut input = fs::File::open(&target).map_err(fs_scope::io_error)?;
        let mut output = fs_scope::create_file(staging, Path::new(rel))?;
        std::io::copy(&mut input, &mut output).map_err(fs_scope::io_error)?;
        output.sync_all().map_err(fs_scope::io_error)?;
    }

    Ok(())
}

fn resolve_source_file(path: &Path) -> Result<PathBuf, ManagerError> {
    let meta = fs::symlink_metadata(path).map_err(|e| {
        ManagerError::new(
            ErrorCode::IoError,
            format!("Metadata failed {}: {e}", path.display()),
        )
    })?;

    if meta.file_type().is_symlink() {
        let link = fs::read_link(path).map_err(|e| {
            ManagerError::new(
                ErrorCode::IoError,
                format!("Read link failed {}: {e}", path.display()),
            )
        })?;

        let resolved = if link.is_absolute() {
            link
        } else {
            path.parent().unwrap_or(path).join(link)
        };

        if !resolved.is_file() {
            return Err(ManagerError::new(
                ErrorCode::NotFound,
                format!("Link target not file: {}", resolved.display()),
            ));
        }

        return Ok(resolved);
    }

    if !meta.is_file() {
        return Err(ManagerError::new(
            ErrorCode::InvalidPath,
            format!("Not file for migration: {}", path.display()),
        ));
    }

    Ok(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::migrate_external_mod;

    #[test]
    fn migrates_external_symlink_mod_to_managed_storage() {
        let tmp = TempDir::new().expect("tmp");
        let managed = tmp.path().join("managed");
        let game_mods = tmp.path().join("Game/Mods");
        let external_root = tmp.path().join("outside");

        fs::create_dir_all(&managed).expect("managed");
        fs::create_dir_all(&game_mods).expect("mods");
        fs::create_dir_all(&external_root).expect("outside");

        fs::write(external_root.join("ExtMod_main.package"), b"PKG").expect("pkg");

        #[cfg(unix)]
        std::os::unix::fs::symlink(
            external_root.join("ExtMod_main.package"),
            game_mods.join("ExtMod_main.package"),
        )
        .expect("link");

        let migrated = migrate_external_mod(&managed, &game_mods, "ExtMod").expect("migrate");

        assert!(managed
            .join("mods")
            .join(&migrated.managed_mod_id)
            .join("files/ExtMod_main.package")
            .is_file());
    }

    #[test]
    fn migration_replaces_external_links_with_managed_links() {
        let tmp = TempDir::new().expect("tmp");
        let managed = tmp.path().join("managed");
        let game_mods = tmp.path().join("Game/Mods");
        let external_root = tmp.path().join("outside");

        fs::create_dir_all(&managed).expect("managed");
        fs::create_dir_all(&game_mods).expect("mods");
        fs::create_dir_all(&external_root).expect("outside");

        let src = external_root.join("SkinPack_main.package");
        let link = game_mods.join("SkinPack_main.package");
        fs::write(&src, b"PKG").expect("pkg");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&src, &link).expect("link");

        let migrated = migrate_external_mod(&managed, &game_mods, "SkinPack").expect("migrate");

        let target = fs::read_link(&link).expect("still symlink");
        assert!(target.starts_with(
            managed
                .join("mods")
                .join(migrated.managed_mod_id)
                .join("files")
        ));
    }

    #[test]
    fn rejects_non_external_mod_keys() {
        let tmp = TempDir::new().expect("tmp");
        let managed = tmp.path().join("managed");
        let game_mods = tmp.path().join("Game/Mods");

        let managed_file = managed.join("mods/id/files/Local_main.package");
        fs::create_dir_all(managed_file.parent().expect("parent")).expect("managed");
        fs::create_dir_all(&game_mods).expect("mods");
        fs::write(&managed_file, b"PKG").expect("pkg");

        fs::write(managed.join("mods/id/meta.json"), serde_json::json!({
            "version": 1, "createdBy": "sims4-mod-manager", "modId": "id",
            "name": "Local", "displayName": "Local", "files": ["y.package"],
            "source": "local"
        }).to_string()).unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&managed_file, game_mods.join("Local_main.package"))
            .expect("link");

        let err = migrate_external_mod(&managed, &game_mods, "Local").expect_err("must fail");
        assert_eq!(err.code.as_str(), "EXTERNAL_LINK");
    }
}
