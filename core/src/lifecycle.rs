mod trash_deletion;
pub use trash_deletion::{TrashDeletion, TrashDeletionEntry, delete_reviewed_trash, review_trash_deletion};

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{ErrorCode, ManagerError};
use crate::toggle::IssueEvent;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct UninstallResult {
    pub mod_id: String,
    pub trashed_path: String,
    pub issues: Vec<IssueEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TrashEntry {
    #[serde(default)]
    pub title: Option<String>,
    pub name: String,
    pub path: String,
    pub original_path: Option<String>,
    pub deletion_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RestoreResult {
    pub restored_path: String,
}

pub fn uninstall_managed_mod(
    managed_root: &Path,
    game_mods_dir: &Path,
    trash_files_dir: &Path,
    mod_id: &str,
) -> Result<UninstallResult, ManagerError> {
    let _guard = crate::operation::acquire(managed_root)?;
    crate::managed_storage::read_managed_mod(managed_root, mod_id)?;
    let mod_root = managed_root.join("mods").join(mod_id);
    let mut instances = vec![];
    let records = mod_root.join("links");
    if crate::fs_scope::exists(&records) {
        crate::fs_scope::ensure_no_symlink_parents(
            managed_root,
            &PathBuf::from("mods").join(mod_id).join("links/entry"),
        )?;
        for entry in fs::read_dir(&records).map_err(crate::fs_scope::io_error)? {
            let path = entry.map_err(crate::fs_scope::io_error)?.path();
            if path.extension().and_then(|v| v.to_str()) != Some("json") {
                continue;
            }
            if fs::symlink_metadata(&path)
                .map_err(crate::fs_scope::io_error)?
                .file_type()
                .is_symlink()
            {
                return Err(ManagerError::new(
                    ErrorCode::InvalidPath,
                    "Ownership record is a symlink",
                ));
            }
            let record: crate::link_ownership::LinkRecord =
                serde_json::from_slice(&fs::read(&path).map_err(crate::fs_scope::io_error)?)
                    .map_err(|e| ManagerError::new(ErrorCode::InvalidPath, e.to_string()))?;
            if record.version != 2 || record.mod_id != mod_id {
                return Err(ManagerError::new(
                    ErrorCode::InvalidPath,
                    "Ownership record mismatch",
                ));
            }
            let instance = record.instance_root.ok_or_else(|| {
                ManagerError::new(ErrorCode::InvalidPath, "Missing ownership instance")
            })?;
            instances.push(instance);
        }
    }
    if mod_root.join("links.json").is_file() && !instances.contains(&game_mods_dir.to_path_buf()) {
        instances.push(game_mods_dir.to_path_buf());
    }
    let mut issues = vec![];
    let mut removals = vec![];
    for instance in instances {
        if !instance.is_dir() {
            issues.push(issue_for_skipped_link(
                &instance.to_string_lossy(),
                "Instance unavailable",
            ));
            continue;
        }
        let record = crate::link_ownership::read(managed_root, mod_id, &instance)?;
        for rel in record.links {
            if !crate::fs_scope::exists(&instance.join(&rel)) {
                continue;
            }
            if crate::link_ownership::owned(managed_root, mod_id, &instance, &rel)? {
                let target =
                    fs::read_link(instance.join(&rel)).map_err(crate::fs_scope::io_error)?;
                removals.push((instance.clone(), rel, target));
            } else {
                issues.push(issue_for_skipped_link(&rel, "Target replaced"));
            }
        }
    }
    crate::fs_scope::create_dir_all(trash_files_dir)?;
    crate::fs_scope::Directory::open(trash_files_dir)?;
    let trashed_name = format!("{}-{}", mod_id, uuid::Uuid::new_v4());
    let trashed = trash_files_dir.join(&trashed_name);
    validate_trashinfo_scope(trash_files_dir, &trashed_name)?;
    let mut transaction = crate::operation::Transaction::new(managed_root, "trash")?;
    let result = (|| {
        for (instance, rel, target) in &removals {
            transaction.remove_link(instance, Path::new(rel), target)?;
        }
        transaction.move_path(&mod_root, &trashed)?;
        write_trashinfo(trash_files_dir, &trashed_name, &mod_root, &mut transaction)?;
        Ok(())
    })();
    if let Err(error) = result {
        transaction.rollback()?;
        return Err(error);
    }
    transaction.commit()?;
    Ok(UninstallResult {
        mod_id: mod_id.into(),
        trashed_path: trashed.to_string_lossy().into(),
        issues,
    })
}

pub fn list_trash_entries(trash_files_dir: &Path) -> Result<Vec<TrashEntry>, ManagerError> {
    if !trash_files_dir.exists() {
        return Ok(vec![]);
    }

    let mut entries = fs::read_dir(trash_files_dir)
        .map_err(|e| {
            ManagerError::new(
                ErrorCode::IoError,
                format!("Read trash failed {}: {e}", trash_files_dir.display()),
            )
        })?
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let name = path.file_name()?.to_string_lossy().to_string();
            if name.starts_with(".ts4-") {
                return None;
            }
            let info = read_trashinfo(trash_files_dir, &name);
            if !is_mod_trash_entry(trash_files_dir, &name, &path, info.as_ref()) {
                return None;
            }
            // Display metadata is read in the worker, never while painting the UI.
            // It does not grant restore or deletion eligibility.
            let title = (|| {
                crate::fs_scope::Directory::open(&path).ok()?;
                let meta = path.join("meta.json");
                if !fs::symlink_metadata(&meta).ok()?.is_file() { return None; }
                let metadata: crate::managed_storage::ModMetadata = serde_json::from_slice(&fs::read(meta).ok()?).ok()?;
                let title = metadata.custom_name.filter(|name| !name.trim().is_empty())
                    .unwrap_or_else(|| if metadata.display_name.trim().is_empty() { metadata.name } else { metadata.display_name });
                Some(title)
            })();
            Some(TrashEntry {
                title,
                name,
                path: path.to_string_lossy().to_string(),
                original_path: info.as_ref().and_then(|info| info.original_path.clone()),
                deletion_date: info.as_ref().and_then(|info| info.deletion_date.clone()),
            })
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(entries)
}

pub fn restore_trashed_mod(
    managed_root: &Path,
    game_mods_dir: &Path,
    trash_files_dir: &Path,
    trash_name: &str,
) -> Result<RestoreResult, ManagerError> {
    let _guard = crate::operation::acquire(managed_root)?;
    crate::fs_scope::component(trash_name)?;
    crate::fs_scope::Directory::open(trash_files_dir)?;
    let entry = trash_files_dir.join(trash_name);
    validate_trashinfo_scope(trash_files_dir, trash_name)?;
    let info = read_trashinfo(trash_files_dir, trash_name);
    let kind = fs::symlink_metadata(&entry).map_err(|e| {
        ManagerError::new(
            if e.kind() == std::io::ErrorKind::NotFound {
                ErrorCode::NotFound
            } else {
                ErrorCode::IoError
            },
            e.to_string(),
        )
    })?;
    if !is_mod_trash_entry(trash_files_dir, trash_name, &entry, info.as_ref()) {
        return Err(ManagerError::new(
            ErrorCode::InvalidPath,
            "Trash entry does not contain mod data",
        ));
    }
    let mut moves: Vec<(PathBuf, PathBuf)> = vec![];
    let metadata_destination = |bundle: &Path| -> Result<PathBuf, ManagerError> {
        let bundle_kind = fs::symlink_metadata(bundle).map_err(crate::fs_scope::io_error)?;
        if !bundle_kind.is_dir() || bundle_kind.file_type().is_symlink() {
            return Err(ManagerError::new(
                ErrorCode::InvalidPath,
                "Managed restore bundle must be a real directory",
            ));
        }
        let path = bundle.join("meta.json");
        if fs::symlink_metadata(&path)
            .map_err(crate::fs_scope::io_error)?
            .file_type()
            .is_symlink()
        {
            return Err(ManagerError::new(
                ErrorCode::InvalidPath,
                "Symlink metadata",
            ));
        }
        let metadata: crate::managed_storage::ModMetadata =
            serde_json::from_slice(&fs::read(path).map_err(crate::fs_scope::io_error)?)
                .map_err(|e| ManagerError::new(ErrorCode::InvalidPath, e.to_string()))?;
        crate::fs_scope::component(&metadata.mod_id)?;
        if metadata.version != 1 {
            return Err(ManagerError::new(
                ErrorCode::InvalidPath,
                "Unsupported metadata version",
            ));
        }
        for rel in &metadata.files {
            crate::fs_scope::relative(Path::new(rel))?;
        }
        Ok(managed_root.join("mods").join(metadata.mod_id))
    };
    if kind.is_dir() && entry.join("meta.json").is_file() {
        moves.push((entry.clone(), metadata_destination(&entry)?));
    } else if kind.is_dir() {
        for child in fs::read_dir(&entry).map_err(crate::fs_scope::io_error)? {
            let child = child.map_err(crate::fs_scope::io_error)?;
            let source = child.path();
            let destination = if child.file_name() == "metadata" {
                metadata_destination(&source)?
            } else {
                game_mods_dir.join(child.file_name())
            };
            moves.push((source, destination));
        }
    } else {
        let name = info
            .as_ref()
            .and_then(|i| i.original_path.as_ref())
            .and_then(|p| Path::new(p).file_name())
            .map(|n| n.to_owned())
            .unwrap_or_else(|| {
                trash_name
                    .rsplit_once('-')
                    .map(|(n, _)| n)
                    .unwrap_or(trash_name)
                    .into()
            });
        moves.push((entry.clone(), game_mods_dir.join(name)));
    }
    if moves.is_empty() {
        return Err(ManagerError::new(
            ErrorCode::InvalidPath,
            "Empty trash entry",
        ));
    }
    // Validate every destination before moving any source, including dangling leaves.
    for (_, destination) in &moves {
        let (root, relative) = if let Ok(relative) = destination.strip_prefix(managed_root) {
            (managed_root, relative)
        } else {
            (
                game_mods_dir,
                destination.strip_prefix(game_mods_dir).map_err(|_| {
                    ManagerError::new(ErrorCode::InvalidPath, "Restore path escaped")
                })?,
            )
        };
        crate::fs_scope::ensure_no_symlink_parents(root, relative)?;
        if crate::fs_scope::exists(destination) {
            return Err(ManagerError::new(
                ErrorCode::PathCollision,
                format!("Restore target exists: {}", destination.display()),
            ));
        }
    }
    for (_, destination) in &moves {
        crate::fs_scope::create_dir_all(destination.parent().unwrap())?;
        crate::fs_scope::Directory::open(destination.parent().unwrap())?;
    }
    let mut transaction = crate::operation::Transaction::new(managed_root, "restore")?;
    let result = (|| {
        for (source, destination) in &moves {
            transaction.move_path(source, destination)?;
        }
        if entry.is_dir() {
            transaction.move_path(
                &entry,
                &trash_files_dir.join(format!(".ts4-restored-wrapper-{}", uuid::Uuid::new_v4())),
            )?;
        }
        if let Some(root) = trash_files_dir.parent() {
            let path = root.join("info").join(format!("{trash_name}.trashinfo"));
            if crate::fs_scope::exists(&path) {
                transaction.move_path(
                    &path,
                    &root
                        .join("info")
                        .join(format!(".ts4-restored-index-{}", uuid::Uuid::new_v4())),
                )?;
            }
        }
        Ok(())
    })();
    if let Err(error) = result {
        transaction.rollback()?;
        return Err(error);
    }
    transaction.commit()?;
    Ok(RestoreResult {
        restored_path: moves.last().unwrap().1.to_string_lossy().into(),
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TrashInfo {
    original_path: Option<String>,
    deletion_date: Option<String>,
}

fn read_trashinfo(trash_files_dir: &Path, trash_name: &str) -> Option<TrashInfo> {
    let trash_root = trash_files_dir.parent()?;
    let raw = fs::read_to_string(
        trash_root
            .join("info")
            .join(format!("{trash_name}.trashinfo")),
    )
    .ok()?;
    let mut info = TrashInfo {
        original_path: None,
        deletion_date: None,
    };
    for line in raw.lines() {
        if let Some(path) = line.strip_prefix("Path=") {
            info.original_path = Some(path.to_string());
        } else if let Some(date) = line.strip_prefix("DeletionDate=") {
            info.deletion_date = Some(date.to_string());
        }
    }
    Some(info)
}

fn is_mod_trash_entry(
    _trash_files_dir: &Path,
    _trash_name: &str,
    entry_path: &Path,
    info: Option<&TrashInfo>,
) -> bool {
    if entry_contains_mod_data(entry_path) {
        return true;
    }

    info.and_then(|info| info.original_path.as_deref())
        .is_some_and(|path| {
            path.contains("sims4-mod-manager")
                || path.contains("/The Sims 4/Mods/")
                || path.ends_with("/The Sims 4/Mods")
        })
}

fn entry_contains_mod_data(path: &Path) -> bool {
    let legacy_single_name = path
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.rsplit_once('-'))
        .filter(|(_, suffix)| !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit()))
        .is_some_and(|(name, _)| {
            Path::new(name)
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(is_mod_extension)
        });
    let Ok(kind) = fs::symlink_metadata(path) else {
        return false;
    };
    if kind.file_type().is_symlink() {
        return legacy_single_name
            || path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(is_mod_extension);
    }
    if path.join("meta.json").is_file() || path.join("metadata/meta.json").is_file() {
        return true;
    }

    if path.is_file() {
        return legacy_single_name
            || path
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(is_mod_extension);
    }

    let Ok(entries) = fs::read_dir(path) else {
        return false;
    };

    entries.flatten().any(|entry| {
        let child = entry.path();
        if fs::symlink_metadata(&child).is_ok_and(|m| m.is_dir()) {
            entry_contains_mod_data(&child)
        } else {
            child
                .extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(is_mod_extension)
        }
    })
}

fn is_mod_extension(ext: &str) -> bool {
    ext.eq_ignore_ascii_case("package") || ext.eq_ignore_ascii_case("ts4script")
}

fn issue_for_skipped_link(rel: &str, _reason: &str) -> IssueEvent {
    IssueEvent {
        id: format!("uninstall-skip:{rel}"),
        severity: "warning".to_string(),
        message: format!("Skipped unmanaged file during uninstall: {rel}"),
        code: Some("EXTERNAL_LINK".to_string()),
    }
}

fn write_trashinfo(
    trash_files_dir: &Path,
    trashed_name: &str,
    original_path: &Path,
    transaction: &mut crate::operation::Transaction,
) -> Result<(), ManagerError> {
    let Some(trash_root) = trash_files_dir.parent() else {
        return Ok(());
    };
    let info_dir = trash_root.join("info");
    crate::fs_scope::create_dir_all(&info_dir)?;
    let deletion_date = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S");
    let content = format!(
        "[Trash Info]\nPath={}\nDeletionDate={}\n",
        original_path.to_string_lossy(),
        deletion_date
    );
    transaction.write_file(
        &info_dir.join(format!("{trashed_name}.trashinfo")),
        content.as_bytes(),
    )
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use crate::managed_storage::{create_managed_mod, ImportRequest};
    use crate::toggle::apply_toggle;

    use super::{list_trash_entries, restore_trashed_mod, uninstall_managed_mod};

    fn setup(tmp: &TempDir) -> (String, std::path::PathBuf) {
        let import = tmp.path().join("import/modA");
        fs::create_dir_all(&import).expect("import dir");
        fs::write(import.join("a.package"), b"AAA").expect("a");

        let meta = create_managed_mod(
            tmp.path(),
            ImportRequest {
                name: "ModA".to_string(),
                slug: None,
                source_dir: import,
            },
        )
        .expect("managed");

        let mods_dir = tmp.path().join("Game/Mods");
        fs::create_dir_all(&mods_dir).expect("mods dir");
        (meta.mod_id, mods_dir)
    }

    #[test]
    fn uninstall_moves_managed_folder_to_trash_and_removes_symlinks() {
        let tmp = TempDir::new().expect("tmp");
        let (mod_id, mods_dir) = setup(&tmp);
        apply_toggle(tmp.path(), &mods_dir, &mod_id, true).expect("enable");
        assert!(mods_dir.join("a.package").exists());

        let trash_files = tmp.path().join("Trash/files");
        let result =
            uninstall_managed_mod(tmp.path(), &mods_dir, &trash_files, &mod_id).expect("uninstall");

        assert!(!tmp.path().join("mods").join(&mod_id).exists());
        assert!(!mods_dir.join("a.package").exists());
        assert!(std::path::Path::new(&result.trashed_path)
            .join("meta.json")
            .exists());
        assert!(tmp
            .path()
            .join("Trash/info")
            .read_dir()
            .expect("info dir")
            .next()
            .is_some());
        assert!(result.issues.is_empty());
    }

    #[test]
    fn uninstall_without_links_still_moves_to_trash() {
        let tmp = TempDir::new().expect("tmp");
        let (mod_id, mods_dir) = setup(&tmp);
        let trash_files = tmp.path().join("Trash/files");

        let result =
            uninstall_managed_mod(tmp.path(), &mods_dir, &trash_files, &mod_id).expect("uninstall");

        assert!(!tmp.path().join("mods").join(&mod_id).exists());
        assert!(std::path::Path::new(&result.trashed_path).exists());
    }

    #[test]
    fn uninstall_does_not_remove_unmanaged_file_at_link_path() {
        let tmp = TempDir::new().expect("tmp");
        let (mod_id, mods_dir) = setup(&tmp);
        apply_toggle(tmp.path(), &mods_dir, &mod_id, true).expect("enable");
        fs::remove_file(mods_dir.join("a.package")).expect("remove symlink");
        fs::write(mods_dir.join("a.package"), b"user file").expect("user file");

        let trash_files = tmp.path().join("Trash/files");
        let result =
            uninstall_managed_mod(tmp.path(), &mods_dir, &trash_files, &mod_id).expect("uninstall");

        assert_eq!(
            fs::read(mods_dir.join("a.package")).expect("read"),
            b"user file"
        );
        assert!(result
            .issues
            .iter()
            .any(|issue| issue.code.as_deref() == Some("EXTERNAL_LINK")));
    }

    #[test]
    fn uninstall_rejects_unmanaged_installed_folder_and_invalid_metadata() {
        let tmp = TempDir::new().unwrap();
        let game = tmp.path().join("Game/Mods");
        fs::create_dir_all(game.join("LooseMod")).unwrap();
        fs::write(game.join("LooseMod/main.package"), b"original").unwrap();
        fs::create_dir_all(tmp.path().join("mods/LooseMod")).unwrap();
        fs::write(tmp.path().join("mods/LooseMod/meta.json"), b"{}").unwrap();
        assert!(uninstall_managed_mod(
            tmp.path(),
            &game,
            &tmp.path().join("Trash/files"),
            "LooseMod"
        )
        .is_err());
        assert_eq!(
            fs::read(game.join("LooseMod/main.package")).unwrap(),
            b"original"
        );
    }
    #[test]
    fn uninstall_rejects_unmanaged_prefix_group() {
        let tmp = TempDir::new().unwrap();
        let game = tmp.path().join("Game/Mods");
        fs::create_dir_all(&game).unwrap();
        fs::write(game.join("LooseMod_a.package"), b"original").unwrap();
        assert!(uninstall_managed_mod(
            tmp.path(),
            &game,
            &tmp.path().join("Trash/files"),
            "LooseMod"
        )
        .is_err());
        assert_eq!(
            fs::read(game.join("LooseMod_a.package")).unwrap(),
            b"original"
        );
    }

    #[test]
    fn list_trash_entries_filters_unrelated_user_trash() {
        let tmp = TempDir::new().expect("tmp");
        let trash_files = tmp.path().join("Trash/files");
        let trash_info = tmp.path().join("Trash/info");
        fs::create_dir_all(&trash_files).expect("trash files");
        fs::create_dir_all(&trash_info).expect("trash info");
        fs::write(trash_files.join("notes.txt"), b"notes").expect("notes");
        fs::write(
            trash_info.join("notes.txt.trashinfo"),
            "[Trash Info]\nPath=/home/me/notes.txt\n",
        )
        .expect("notes info");
        fs::create_dir_all(trash_files.join("LooseMod-123")).expect("mod trash");
        fs::write(trash_files.join("LooseMod-123/LooseMod.package"), b"pkg").expect("pkg");
        fs::write(
            trash_info.join("LooseMod-123.trashinfo"),
            "[Trash Info]\nPath=/home/me/Documents/Electronic Arts/The Sims 4/Mods/LooseMod\n",
        )
        .expect("mod info");

        let entries = list_trash_entries(&trash_files).expect("list");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "LooseMod-123");
        assert_eq!(
            entries[0].original_path.as_deref(),
            Some("/home/me/Documents/Electronic Arts/The Sims 4/Mods/LooseMod")
        );
    }

    #[test]
    fn list_trash_entries_includes_original_path_and_deletion_date() {
        let tmp = TempDir::new().expect("tmp");
        let (id, mods_dir) = setup(&tmp);
        let trash_files = tmp.path().join("Trash/files");
        let trashed =
            uninstall_managed_mod(tmp.path(), &mods_dir, &trash_files, &id).expect("uninstall");
        let trash_name = std::path::Path::new(&trashed.trashed_path)
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();

        let entries = list_trash_entries(&trash_files).expect("list");

        assert_eq!(entries[0].name, trash_name);
        assert_eq!(serde_json::to_value(&entries[0]).unwrap()["title"], "ModA");
        assert_eq!(
            entries[0].original_path.as_deref(),
            Some(tmp.path().join("mods").join(id).to_string_lossy().as_ref())
        );
        assert!(entries[0]
            .deletion_date
            .as_deref()
            .unwrap_or_default()
            .contains('T'));
    }

    #[test]
    fn restore_trash_entry_moves_installed_files_back() {
        let tmp = TempDir::new().expect("tmp");
        let mods_dir = tmp.path().join("Game/Mods");
        fs::create_dir_all(&mods_dir).unwrap();
        let trash_files = tmp.path().join("Trash/files");
        let trash_name = "legacy-123".to_string();
        fs::create_dir_all(trash_files.join(&trash_name).join("LooseMod")).unwrap();
        fs::write(
            trash_files.join(&trash_name).join("LooseMod/main.package"),
            b"pkg",
        )
        .unwrap();
        let entries = list_trash_entries(&trash_files).expect("list");
        assert_eq!(entries[0].name, trash_name);
        assert!(entries[0].title.is_none());
        restore_trashed_mod(tmp.path(), &mods_dir, &trash_files, &trash_name).expect("restore");

        assert!(mods_dir.join("LooseMod/main.package").exists());
        assert!(!trash_files.join(trash_name).exists());
    }

    #[test]
    fn uninstall_missing_managed_folder_fails_without_creating_trash() {
        let tmp = TempDir::new().expect("tmp");
        let mods_dir = tmp.path().join("Game/Mods");
        fs::create_dir_all(&mods_dir).expect("mods dir");

        let trash_files = tmp.path().join("Trash/files");
        let err = uninstall_managed_mod(tmp.path(), &mods_dir, &trash_files, "missing")
            .expect_err("missing");

        assert_eq!(err.code.as_str(), "NOT_FOUND");
        assert!(!trash_files.exists());
    }
}

fn validate_trashinfo_scope(trash_files_dir: &Path, name: &str) -> Result<(), ManagerError> {
    let root = trash_files_dir
        .parent()
        .ok_or_else(|| ManagerError::new(ErrorCode::InvalidPath, "Trash root missing"))?;
    crate::fs_scope::Directory::open(root)?;
    let relative = PathBuf::from("info").join(format!("{name}.trashinfo"));
    crate::fs_scope::ensure_no_symlink_parents(root, &relative)?;
    let leaf = root.join(relative);
    if crate::fs_scope::exists(&leaf)
        && !fs::symlink_metadata(&leaf)
            .map_err(crate::fs_scope::io_error)?
            .is_file()
    {
        return Err(ManagerError::new(
            ErrorCode::InvalidPath,
            "Trash index must be a real file",
        ));
    }
    Ok(())
}
