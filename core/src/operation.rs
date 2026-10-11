//! Shared app writer authority and durable operation-specific rollback records.
use crate::{
    error::{ErrorCode, ManagerError},
    fs_scope,
    toggle::IssueEvent,
    transfer,
};
use serde::{Deserialize, Serialize};
use std::{
    cell::Cell,
    fs,
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard},
};
static WRITER: Mutex<()> = Mutex::new(());
thread_local! {static DEPTH:Cell<usize>=const{Cell::new(0)};}
pub struct Guard {
    _mutex: Option<MutexGuard<'static, ()>>,
    _file: Option<fs::File>,
}
impl Drop for Guard {
    fn drop(&mut self) {
        DEPTH.with(|d| d.set(d.get() - 1));
    }
}
#[cfg(unix)]
pub fn acquire(root: &Path) -> Result<Guard, ManagerError> {
    if DEPTH.with(|d| d.get() > 0) {
        DEPTH.with(|d| d.set(d.get() + 1));
        return Ok(Guard {
            _mutex: None,
            _file: None,
        });
    }
    let mutex = WRITER
        .lock()
        .map_err(|_| ManagerError::new(ErrorCode::InternalError, "Writer lock poisoned"))?;
    use std::os::fd::AsRawFd;
    let file = shared_lock_file()?;
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } < 0 {
        return Err(fs_scope::io_error(std::io::Error::last_os_error()));
    }
    DEPTH.with(|d| d.set(1));
    let guard = Guard {
        _mutex: Some(mutex),
        _file: Some(file),
    };
    let issues = recover_inner(root)?;
    if !issues.is_empty() {
        return Err(ManagerError::new(
            ErrorCode::IoError,
            format!("Recovery requires attention: {}", issues[0].message),
        ));
    }
    Ok(guard)
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum Step {
    Move {
        source: PathBuf,
        destination: PathBuf,
        holding: PathBuf,
        fingerprint: String,
        #[serde(default)]
        copied: bool,
    },
    CreateLink {
        root: PathBuf,
        relative: PathBuf,
        target: PathBuf,
    },
    WriteFile {
        path: PathBuf,
        before: Option<Vec<u8>>,
        after: Vec<u8>,
    },
    RemoveLink {
        root: PathBuf,
        relative: PathBuf,
        target: PathBuf,
    },
}
#[derive(Serialize, Deserialize)]
struct Journal {
    version: u32,
    kind: String,
    id: String,
    steps: Vec<Step>,
    managed_root: PathBuf,
    scopes: Vec<PathBuf>,
}
pub struct Transaction {
    _guard: Guard,
    path: PathBuf,
    journal: Journal,
}
impl Transaction {
    pub fn new(root: &Path, kind: &str) -> Result<Self, ManagerError> {
        let guard = acquire(root)?;
        fs_scope::create_dir_all(root)?;
        fs_scope::ensure_no_symlink_parents(root, Path::new("operations/new.json"))?;
        fs_scope::create_dir_all(&root.join("operations"))?;
        let id = uuid::Uuid::new_v4().to_string();
        let path = root.join("operations").join(format!("{id}.json"));
        let t = Self {
            _guard: guard,
            path,
            journal: Journal {
                version: 1,
                kind: kind.into(),
                id,
                steps: vec![],
                managed_root: fs::canonicalize(root).map_err(fs_scope::io_error)?,
                scopes: vec![],
            },
        };
        t.persist()?;
        Ok(t)
    }
    fn persist(&self) -> Result<(), ManagerError> {
        use sha2::{Digest, Sha256};
        let body = serde_json::to_value(&self.journal)
            .map_err(|e| ManagerError::new(ErrorCode::InternalError, e.to_string()))?;
        let bytes = serde_json::to_vec(&body)
            .map_err(|e| ManagerError::new(ErrorCode::InternalError, e.to_string()))?;
        let envelope =
            serde_json::json!({"checksum":format!("{:x}",Sha256::digest(bytes)),"journal":body});
        fs_scope::atomic_write(
            &self.path,
            &serde_json::to_vec_pretty(&envelope)
                .map_err(|e| ManagerError::new(ErrorCode::InternalError, e.to_string()))?,
        )
    }
    fn record(&mut self, mut step: Step) -> Result<(), ManagerError> {
        let absolute_leaf = |p: &Path| -> Result<PathBuf, ManagerError> {
            let parent = p
                .parent()
                .ok_or_else(|| ManagerError::new(ErrorCode::InvalidPath, "Path has no parent"))?;
            Ok(fs::canonicalize(parent)
                .map_err(fs_scope::io_error)?
                .join(p.file_name().ok_or_else(|| {
                    ManagerError::new(ErrorCode::InvalidPath, "Path has no leaf")
                })?))
        };
        match &mut step {
            Step::Move {
                source,
                destination,
                holding,
                ..
            } => {
                *source = absolute_leaf(source)?;
                *destination = absolute_leaf(destination)?;
                *holding = absolute_leaf(holding)?;
            }
            Step::CreateLink { root, .. } | Step::RemoveLink { root, .. } => {
                *root = fs::canonicalize(&root).map_err(fs_scope::io_error)?;
            }
            Step::WriteFile { path, .. } => {
                *path = absolute_leaf(path)?;
            }
        }
        let paths: Vec<&Path> = match &step {
            Step::Move {
                source,
                destination,
                ..
            } => vec![source.parent().unwrap(), destination.parent().unwrap()],
            Step::CreateLink { root, .. } | Step::RemoveLink { root, .. } => vec![root],
            Step::WriteFile { path, .. } => vec![path.parent().unwrap()],
        };
        for path in paths {
            let scope = fs::canonicalize(path).map_err(fs_scope::io_error)?;
            fs_scope::Directory::open(&scope)?;
            if !self.journal.scopes.contains(&scope) {
                self.journal.scopes.push(scope);
            }
        }
        self.journal.steps.push(step);
        self.persist()
    }
    pub fn move_path(&mut self, source: &Path, destination: &Path) -> Result<(), ManagerError> {
        if fs_scope::exists(destination) {
            return Err(ManagerError::new(
                ErrorCode::PathCollision,
                "Move destination exists",
            ));
        }
        let fingerprint = transfer::fingerprint(source)?;
        let holding = source.parent().unwrap().join(format!(
            ".ts4-holding-{}-{}",
            self.journal.id,
            self.journal.steps.len()
        ));
        self.record(Step::Move {
            source: source.into(),
            destination: destination.into(),
            holding: holding.clone(),
            fingerprint,
            copied: false,
        })?;
        let result = match fs_scope::rename_no_replace(source, destination) {
            Ok(()) => Ok(()),
            Err(e) if e.message.contains("cross-device") || e.message.contains("os error 18") => {
                transfer::copy_verified(source, destination)?;
                if let Some(Step::Move { copied, .. }) = self.journal.steps.last_mut() {
                    *copied = true;
                }
                self.persist()?;
                fs_scope::rename_no_replace(source, &holding)
            }
            Err(e) => Err(e),
        };
        result?;
        checkpoint()
    }
    pub fn create_link(
        &mut self,
        root: &Path,
        relative: &Path,
        target: &Path,
    ) -> Result<(), ManagerError> {
        self.record(Step::CreateLink {
            root: root.into(),
            relative: relative.into(),
            target: target.into(),
        })?;
        fs_scope::Directory::open(root)?.symlink(relative, target)?;
        checkpoint()
    }
    pub fn remove_link(
        &mut self,
        root: &Path,
        relative: &Path,
        target: &Path,
    ) -> Result<(), ManagerError> {
        self.record(Step::RemoveLink {
            root: root.into(),
            relative: relative.into(),
            target: target.into(),
        })?;
        fs_scope::Directory::open(root)?.unlink_owned(relative, target)?;
        checkpoint()
    }
    pub fn write_file(&mut self, path: &Path, bytes: &[u8]) -> Result<(), ManagerError> {
        let before = if fs_scope::exists(path) {
            if !fs::symlink_metadata(path)
                .map_err(fs_scope::io_error)?
                .is_file()
            {
                return Err(ManagerError::new(
                    ErrorCode::InvalidPath,
                    "Journaled record must be a regular file",
                ));
            }
            Some(fs::read(path).map_err(fs_scope::io_error)?)
        } else {
            None
        };
        self.record(Step::WriteFile {
            path: path.into(),
            before,
            after: bytes.to_vec(),
        })?;
        fs_scope::atomic_write(path, bytes)?;
        checkpoint()
    }
    pub fn commit(self) -> Result<(), ManagerError> {
        fs::remove_file(&self.path).map_err(fs_scope::io_error)?;
        fs::File::open(self.path.parent().unwrap())
            .map_err(fs_scope::io_error)?
            .sync_all()
            .map_err(fs_scope::io_error)
    }
    pub fn rollback(self) -> Result<(), ManagerError> {
        let issues = rollback_journal(&self.journal)?;
        if issues.is_empty() {
            fs::remove_file(&self.path).map_err(fs_scope::io_error)
        } else {
            Err(ManagerError::new(
                ErrorCode::IoError,
                issues[0].message.clone(),
            ))
        }
    }
}
fn issue(id: &str, message: String) -> IssueEvent {
    IssueEvent {
        id: format!("recovery:{id}"),
        severity: "error".into(),
        message,
        code: Some("IO_ERROR".into()),
    }
}
fn rollback_journal(journal: &Journal) -> Result<Vec<IssueEvent>, ManagerError> {
    let mut issues = vec![];
    for step in journal.steps.iter().rev() {
        let result = (|| match step {
            Step::WriteFile {
                path,
                before,
                after,
            } => {
                let current = if fs_scope::exists(path) {
                    Some(fs::read(path).map_err(fs_scope::io_error)?)
                } else {
                    None
                };
                if current == *before {
                    return Ok(());
                }
                if current.as_ref() != Some(after) {
                    return Err(ManagerError::new(
                        ErrorCode::PathCollision,
                        "Record changed during operation; preserving it",
                    ));
                }
                if let Some(bytes) = before {
                    fs_scope::atomic_write(path, bytes)
                } else {
                    // Retain new bookkeeping as a recovery copy rather than deleting it.
                    fs_scope::rename_no_replace(
                        path,
                        &journal
                            .managed_root
                            .join("operations")
                            .join(format!(".ts4-record-recovery-{}", uuid::Uuid::new_v4())),
                    )
                }
            }
            Step::CreateLink {
                root,
                relative,
                target,
            } => {
                let p = root.join(relative);
                if !fs_scope::exists(&p) {
                    return Ok(());
                }
                fs_scope::Directory::open(root)?.unlink_owned(relative, target)
            }
            Step::RemoveLink {
                root,
                relative,
                target,
            } => {
                let p = root.join(relative);
                if fs_scope::exists(&p) {
                    if fs::read_link(&p).ok().as_ref() == Some(target) {
                        return Ok(());
                    }
                    return Err(ManagerError::new(
                        ErrorCode::PathCollision,
                        "Replacement occupies removed link path",
                    ));
                }
                fs_scope::Directory::open(root)?.symlink(relative, target)
            }
            Step::Move {
                source,
                destination,
                holding,
                fingerprint,
                copied,
            } => {
                let retire_copy = || -> Result<(), ManagerError> {
                    if !fs_scope::exists(destination) {
                        return Ok(());
                    }
                    if transfer::fingerprint(destination)? != *fingerprint {
                        return Err(ManagerError::new(
                            ErrorCode::PathCollision,
                            "Transfer destination changed; preserving it",
                        ));
                    }
                    let preserved = destination.parent().unwrap().join(format!(
                        ".ts4-recovery-copy-{}-{}",
                        journal.id,
                        uuid::Uuid::new_v4()
                    ));
                    fs_scope::rename_no_replace(destination, &preserved)
                };
                if fs_scope::exists(source) {
                    if transfer::fingerprint(source)? == *fingerprint {
                        if fs_scope::exists(destination) {
                            if *copied {
                                retire_copy()?;
                            } else {
                                return Err(ManagerError::new(ErrorCode::PathCollision,"Both transfer paths exist without a verified copy phase; preserving them"));
                            }
                        }
                        return Ok(());
                    }
                    return Err(ManagerError::new(
                        ErrorCode::PathCollision,
                        "Source path changed during operation",
                    ));
                }
                let original = if fs_scope::exists(holding) {
                    holding
                } else {
                    destination
                };
                if !fs_scope::exists(original) {
                    return Err(ManagerError::new(
                        ErrorCode::NotFound,
                        "Both transfer paths absent",
                    ));
                }
                if transfer::fingerprint(original)? != *fingerprint {
                    return Err(ManagerError::new(
                        ErrorCode::PathCollision,
                        "Transfer content changed; preserving all paths",
                    ));
                }
                if let Some(parent) = source.parent() {
                    fs_scope::create_dir_all(parent)?;
                }
                fs_scope::rename_no_replace(original, source)?;
                if *copied && original == holding {
                    retire_copy()?;
                }
                Ok(())
            }
        })();
        if let Err(e) = result {
            issues.push(issue(&journal.id, e.message));
            // A link that could not be undone can still depend on an earlier source move.
            // Independent move conflicts may continue recovering the other original paths.
            if matches!(step, Step::CreateLink { .. } | Step::RemoveLink { .. }) {
                break;
            }
        }
    }
    Ok(issues)
}
fn recover_inner(root: &Path) -> Result<Vec<IssueEvent>, ManagerError> {
    let directory = root.join("operations");
    if !fs_scope::exists(&directory) {
        return Ok(vec![]);
    }
    fs_scope::ensure_no_symlink_parents(root, Path::new("operations/record"))?;
    let mut issues = vec![];
    for entry in fs::read_dir(&directory).map_err(fs_scope::io_error)? {
        let path = entry.map_err(fs_scope::io_error)?.path();
        if path.extension().and_then(|p| p.to_str()) != Some("json") {
            continue;
        }
        if fs::symlink_metadata(&path)
            .map_err(fs_scope::io_error)?
            .file_type()
            .is_symlink()
        {
            issues.push(issue("invalid", "Journal is a symlink".into()));
            continue;
        }
        let journal = match decode_journal(&path, root) {
            Ok(j) => j,
            Err(e) => {
                issues.push(issue("invalid", e.message));
                continue;
            }
        };
        let pending = rollback_journal(&journal)?;
        if pending.is_empty() {
            fs::remove_file(path).map_err(fs_scope::io_error)?;
        } else {
            issues.extend(pending);
        }
    }
    Ok(issues)
}
#[cfg(unix)]
pub fn recover_pending(root: &Path) -> Result<Vec<IssueEvent>, ManagerError> {
    // Take the shared authority without triggering the mutation preflight twice.
    if DEPTH.with(|d| d.get() > 0) {
        return recover_inner(root);
    }
    let _mutex = WRITER
        .lock()
        .map_err(|_| ManagerError::new(ErrorCode::InternalError, "Writer lock poisoned"))?;
    use std::os::fd::AsRawFd;
    let file = shared_lock_file()?;
    if unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) } < 0 {
        return Err(fs_scope::io_error(std::io::Error::last_os_error()));
    }
    recover_inner(root)
}

#[cfg(all(test, unix))]
#[path = "operation_crash_tests.rs"]
mod crash_tests;

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[test]
    fn failed_link_undo_preserves_published_content_until_recovery() {
        let temp = tempfile::tempdir().unwrap();
        let managed = temp.path().join("managed");
        let staged = managed.join("tmp/staged");
        let published = managed.join("mods/published");
        let game = temp.path().join("Mods");
        let held_game = temp.path().join("held-Mods");
        fs::create_dir_all(&staged).unwrap();
        fs::create_dir_all(published.parent().unwrap()).unwrap();
        fs::create_dir_all(&game).unwrap();
        fs::write(staged.join("a.package"), b"content").unwrap();
        let mut transaction = Transaction::new(&managed, "folder_import").unwrap();
        transaction.move_path(&staged, &published).unwrap();
        transaction.create_link(&game, Path::new("a.package"), &published.join("a.package")).unwrap();
        fs::rename(&game, &held_game).unwrap();
        std::os::unix::fs::symlink(&held_game, &game).unwrap();
        assert!(transaction.rollback().is_err());
        assert!(published.join("a.package").is_file(), "rollback must preserve a source while its dependent link cannot be undone");
        assert_eq!(fs::read(held_game.join("a.package")).unwrap(), b"content");
        fs::remove_file(&game).unwrap();
        fs::rename(&held_game, &game).unwrap();
        let issues = recover_pending(&managed).unwrap();
        assert!(issues.is_empty(), "{issues:?}");
        assert!(!fs_scope::exists(&game.join("a.package")));
        assert!(!fs_scope::exists(&published));
        assert_eq!(fs::read(staged.join("a.package")).unwrap(), b"content");
    }

    #[test]
    fn injected_enable_failure_rolls_back_created_links() {
        let temp = tempfile::tempdir().unwrap();
        let managed = temp.path().join("managed");
        let game = temp.path().join("Mods");
        let source = temp.path().join("source");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&game).unwrap();
        fs::write(source.join("a.package"), b"a").unwrap();
        fs::write(source.join("b.package"), b"b").unwrap();
        let metadata = crate::managed_storage::create_managed_mod(
            &managed,
            crate::managed_storage::ImportRequest {
                name: "Example".into(),
                slug: None,
                source_dir: source,
            },
        )
        .unwrap();
        FAILURE_AFTER.with(|f| f.set(Some(1)));
        assert!(crate::toggle::apply_toggle(&managed, &game, &metadata.mod_id, true).is_err());
        assert!(!fs_scope::exists(&game.join("a.package")));
        assert!(!fs_scope::exists(&game.join("b.package")));
        assert!(recover_pending(&managed).unwrap().is_empty());
    }
    #[test]
    fn injected_restore_index_retirement_failure_restores_wrapper_and_index() {
        let temp = tempfile::tempdir().unwrap();
        let managed = temp.path().join("managed");
        let game = temp.path().join("Mods");
        let trash = temp.path().join("Trash/files");
        let entry = trash.join("legacy-123");
        let index = temp.path().join("Trash/info/legacy-123.trashinfo");
        fs::create_dir_all(&game).unwrap();
        fs::create_dir_all(&entry).unwrap();
        fs::create_dir_all(index.parent().unwrap()).unwrap();
        fs::write(entry.join("legacy.package"), b"original").unwrap();
        fs::write(&index, b"[Trash Info]\nPath=/The Sims 4/Mods/legacy\n").unwrap();
        // Fail after moving the file, retiring the wrapper, and retiring its index.
        FAILURE_AFTER.with(|f| f.set(Some(3)));
        assert!(
            crate::lifecycle::restore_trashed_mod(&managed, &game, &trash, "legacy-123").is_err()
        );
        assert_eq!(fs::read(entry.join("legacy.package")).unwrap(), b"original");
        assert!(index.is_file());
        assert!(!game.join("legacy.package").exists());
        assert!(recover_pending(&managed).unwrap().is_empty());
    }
    #[test]
    fn injected_migration_failure_restores_original_symlink() {
        let temp = tempfile::tempdir().unwrap();
        let managed = temp.path().join("managed");
        let game = temp.path().join("Game/Mods");
        fs::create_dir_all(&game).unwrap();
        let outside = temp.path().join("outside.package");
        fs::write(&outside, b"original").unwrap();
        std::os::unix::fs::symlink(&outside, game.join("Example_main.package")).unwrap();
        // Folder publication is the first move. Fail after preserving the original.
        FAILURE_AFTER.with(|f| f.set(Some(2)));
        assert!(
            crate::external_migration::migrate_external_mod(&managed, &game, "Example").is_err()
        );
        assert_eq!(
            fs::read_link(game.join("Example_main.package")).unwrap(),
            outside
        );
        assert_eq!(fs::read(outside).unwrap(), b"original");
        assert!(crate::mod_scan::scan_mods(&game, &managed)
            .unwrap()
            .iter()
            .all(|entry| entry.source != crate::mod_scan::ModSource::Managed));
        assert!(recover_pending(&managed).unwrap().is_empty());
    }
}

#[cfg(not(unix))]
pub fn acquire(_root: &Path) -> Result<Guard, ManagerError> {
    Err(ManagerError::new(
        ErrorCode::InternalError,
        "Shared filesystem mutation locking is unsupported on this platform",
    ))
}
#[cfg(not(unix))]
pub fn recover_pending(_root: &Path) -> Result<Vec<IssueEvent>, ManagerError> {
    Err(ManagerError::new(
        ErrorCode::InternalError,
        "Filesystem recovery is unsupported on this platform",
    ))
}
#[cfg(test)]
thread_local! { static FAILURE_AFTER: Cell<Option<usize>> = const { Cell::new(None) }; }
fn checkpoint() -> Result<(), ManagerError> {
    #[cfg(all(test, unix))]
    crash_tests::pause_after_side_effect();
    #[cfg(test)]
    {
        let fail = FAILURE_AFTER.with(|f| match f.get() {
            Some(1) => {
                f.set(None);
                true
            }
            Some(n) => {
                f.set(Some(n - 1));
                false
            }
            None => false,
        });
        if fail {
            return Err(ManagerError::new(
                ErrorCode::IoError,
                "Injected failure after filesystem side effect",
            ));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn shared_lock_file() -> Result<fs::File, ManagerError> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt};
    let uid = unsafe { libc::geteuid() };
    // A fixed path makes the authority independent of each application's TMPDIR.
    let path = PathBuf::from("/tmp").join(format!("ts4-mod-manager-writer-{uid}.lock"));
    let file = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(fs_scope::io_error)?;
    let metadata = file.metadata().map_err(fs_scope::io_error)?;
    if !metadata.is_file()
        || metadata.uid() != uid
        || metadata.nlink() != 1
        || metadata.mode() & 0o077 != 0
    {
        return Err(ManagerError::new(
            ErrorCode::PermissionDenied,
            "Shared writer lock must be a private regular file owned by this user",
        ));
    }
    Ok(file)
}

fn decode_journal(path: &Path, root: &Path) -> Result<Journal, ManagerError> {
    use sha2::{Digest, Sha256};
    let invalid = |message: &str| ManagerError::new(ErrorCode::InvalidPath, message);
    let value: serde_json::Value =
        serde_json::from_slice(&fs::read(path).map_err(fs_scope::io_error)?)
            .map_err(|_| invalid("Malformed journal envelope"))?;
    let body = value
        .get("journal")
        .ok_or_else(|| invalid("Missing journal envelope"))?;
    let digest = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(body).map_err(|_| invalid("Malformed journal body"))?)
    );
    if value.get("checksum").and_then(|v| v.as_str()) != Some(&digest) {
        return Err(invalid("Journal checksum mismatch; preserving record"));
    }
    let journal: Journal =
        serde_json::from_value(body.clone()).map_err(|_| invalid("Malformed journal fields"))?;
    fs_scope::component(&journal.id)?;
    if uuid::Uuid::parse_str(&journal.id).is_err()
        || path.file_stem().and_then(|v| v.to_str()) != Some(&journal.id)
        || journal.version != 1
        || !matches!(
            journal.kind.as_str(),
            "enable" | "disable" | "folder_import" | "external_migration" | "trash" | "restore"
        )
    {
        return Err(invalid(
            "Journal filename, identity, version or kind mismatch",
        ));
    }
    if fs::canonicalize(root).map_err(fs_scope::io_error)? != journal.managed_root {
        return Err(invalid("Journal belongs to a different managed root"));
    }
    let validate_absolute = |p: &Path| -> Result<(), ManagerError> {
        if !p.is_absolute() || fs_scope::normalize(p) != p || p == Path::new("/") {
            return Err(invalid("Unsafe journal scope"));
        }
        Ok(())
    };
    for scope in &journal.scopes {
        validate_absolute(scope)?;
    }
    let scoped = |p: &Path| -> Result<(), ManagerError> {
        validate_absolute(p)?;
        if !journal.scopes.iter().any(|scope| {
            p.strip_prefix(scope)
                .is_ok_and(|rel| fs_scope::relative(rel).is_ok())
        }) {
            return Err(invalid("Journal path escaped recorded approval scopes"));
        }
        Ok(())
    };
    for (index, step) in journal.steps.iter().enumerate() {
        match step {
            Step::Move {
                source,
                destination,
                holding,
                fingerprint,
                ..
            } => {
                scoped(source)?;
                scoped(destination)?;
                scoped(holding)?;
                if holding.parent() != source.parent()
                    || holding.file_name().and_then(|v| v.to_str())
                        != Some(format!(".ts4-holding-{}-{index}", journal.id).as_str())
                    || fingerprint.len() != 64
                    || !fingerprint.bytes().all(|b| b.is_ascii_hexdigit())
                {
                    return Err(invalid("Invalid transfer recovery holding or fingerprint"));
                }
            }
            Step::WriteFile { path, .. } => {
                scoped(path)?;
            }
            Step::CreateLink {
                root,
                relative,
                target,
            }
            | Step::RemoveLink {
                root,
                relative,
                target,
            } => {
                validate_absolute(root)?;
                if !journal.scopes.contains(root) {
                    return Err(invalid("Link root escaped approval scope"));
                }
                fs_scope::relative(relative)?;
                let resolved = if target.is_absolute() {
                    target.clone()
                } else {
                    root.join(relative).parent().unwrap().join(target)
                };
                if !fs_scope::normalize(&resolved).starts_with(journal.managed_root.join("mods")) {
                    return Err(invalid("Journal link target escaped managed storage"));
                }
            }
        }
    }
    Ok(journal)
}
