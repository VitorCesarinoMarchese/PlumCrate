use super::{read_trashinfo, validate_trashinfo_scope};
use crate::{
    error::{ErrorCode, ManagerError},
    fs_scope,
    managed_storage::ModMetadata,
};
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct TrashDeletionEntry {
    pub name: String,
    pub title: String,
    digest: String,
    index_digest: String,
}

/// A snapshot of app-owned trash, produced without changing any files.
#[derive(Debug, Clone)]
pub struct TrashDeletion {
    managed: PathBuf,
    trash: PathBuf,
    entries: Vec<TrashDeletionEntry>,
}
impl TrashDeletion {
    pub fn entries(&self) -> &[TrashDeletionEntry] {
        &self.entries
    }
}

fn invalid(message: &str) -> ManagerError {
    ManagerError::new(ErrorCode::InvalidPath, message)
}

fn inspect(managed: &Path, trash: &Path, name: &str) -> Result<TrashDeletionEntry, ManagerError> {
    fs_scope::component(name)?;
    fs_scope::Directory::open(trash)?;
    validate_trashinfo_scope(trash, name)?;
    let path = trash.join(name);
    let directory = fs_scope::Directory::open(&path)?;
    if !fs::symlink_metadata(path.join("meta.json"))
        .map_err(fs_scope::io_error)?
        .is_file()
    {
        return Err(invalid("Managed metadata must be a real file"));
    }
    let metadata: ModMetadata =
        serde_json::from_slice(&fs::read(path.join("meta.json")).map_err(fs_scope::io_error)?)
            .map_err(|_| invalid("Only managed mod bundles can be permanently deleted"))?;
    fs_scope::component(&metadata.mod_id)?;
    let suffix = name
        .strip_prefix(&format!("{}-", metadata.mod_id))
        .ok_or_else(|| invalid("Trash entry does not match its managed mod"))?;
    if metadata.version != 1
        || metadata.created_by != "sims4-mod-manager"
        || uuid::Uuid::parse_str(suffix).is_err()
    {
        return Err(invalid("Trash entry was not created by this app"));
    }
    let info = read_trashinfo(trash, name)
        .ok_or_else(|| invalid("Missing trash ownership information"))?;
    let expected = managed.join("mods").join(&metadata.mod_id);
    if info
        .original_path
        .as_deref()
        .map(Path::new)
        .map(fs_scope::normalize)
        != Some(fs_scope::normalize(&expected))
    {
        return Err(invalid("Trash entry belongs to another location"));
    }
    let index = trash
        .parent()
        .ok_or_else(|| invalid("Missing trash root"))?
        .join("info")
        .join(format!("{name}.trashinfo"));
    // Hash only verified app-owned bundles; unrelated system trash is never read recursively.
    let digest = directory.mod_content_digest()?;
    let index_digest = format!(
        "{:x}",
        Sha256::digest(fs::read(index).map_err(fs_scope::io_error)?)
    );
    Ok(TrashDeletionEntry {
        name: name.into(),
        title: metadata.effective_display_name().into(),
        digest,
        index_digest,
    })
}

/// An empty name list reviews all eligible app-owned bundles, never the system trash.
pub fn review_trash_deletion(
    managed: &Path,
    trash: &Path,
    names: &[String],
) -> Result<TrashDeletion, ManagerError> {
    let _guard = crate::operation::acquire(managed)?;
    let mut entries = vec![];
    if names.is_empty() {
        if trash.exists() {
            fs_scope::Directory::open(trash)?;
            for child in fs::read_dir(trash).map_err(fs_scope::io_error)? {
                let child = child.map_err(fs_scope::io_error)?;
                if let Some(name) = child.file_name().to_str() {
                    if let Ok(entry) = inspect(managed, trash, name) {
                        entries.push(entry);
                    }
                }
            }
        }
    } else {
        for name in names {
            if !entries
                .iter()
                .any(|entry: &TrashDeletionEntry| entry.name == *name)
            {
                entries.push(inspect(managed, trash, name)?);
            }
        }
    }
    if entries.is_empty() {
        return Err(invalid(
            "No app-managed mods eligible for permanent deletion",
        ));
    }
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(TrashDeletion {
        managed: managed.into(),
        trash: trash.into(),
        entries,
    })
}

/// Call only after explicit user confirmation of this exact review.
pub fn delete_reviewed_trash(
    managed: &Path,
    trash: &Path,
    review: &TrashDeletion,
) -> Result<(), ManagerError> {
    let _guard = crate::operation::acquire(managed)?;
    if managed != review.managed || trash != review.trash {
        return Err(invalid("Trash location changed. Review again."));
    }
    // Recheck the entire batch before deleting any entry.
    for entry in &review.entries {
        let current = inspect(managed, trash, &entry.name)?;
        if current.digest != entry.digest || current.index_digest != entry.index_digest {
            return Err(invalid(
                "Trash contents changed. Review again before deleting.",
            ));
        }
    }
    for entry in &review.entries {
        let files = fs_scope::Directory::open(trash)?;
        validate_trashinfo_scope(trash, &entry.name)?;
        let info = fs_scope::Directory::open(
            &trash
                .parent()
                .ok_or_else(|| invalid("Missing trash root"))?
                .join("info"),
        )?;
        files.remove_reviewed_entry(&entry.name, &entry.digest, true)?;
        info.remove_reviewed_entry(
            &format!("{}.trashinfo", entry.name),
            &entry.index_digest,
            false,
        )?;
    }
    Ok(())
}
