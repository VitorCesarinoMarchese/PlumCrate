//! Explicit browser download consent, held-file verification, and atomic import plus enable.
use crate::{
    curseforge_client::{
        classify_status, CurseForgeClient, CurseForgeTransport, SourceLookupError, SIMS4_GAME_ID,
    },
    error::{ErrorCode, ManagerError},
    fs_scope, link_ownership,
    managed_storage::{self, ImportRequest, ModMetadata, SourceAttachmentMetadata, SourceEvidence},
    mod_updates::{InstalledRelease, Release},
    operation::Transaction,
    toggle,
};
use chrono::{DateTime, Utc};
use md5::Md5;
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::{
    collections::{HashMap, HashSet},
    ffi::OsString,
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc, Mutex,
    },
};
const MAX_DOWNLOAD_BYTES: u64 = 2 * 1024 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserProject {
    pub project_id: u64,
    pub title: String,
    pub author: Option<String>,
    pub source_url: String,
    pub preview_url: Option<String>,
    pub releases: Vec<BrowserRelease>,
    pub complete: bool,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BrowserRelease {
    pub release: Release,
    pub length: u64,
    pub dependencies: FileDependencies,
    hashes: Vec<(u64, String)>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileDependencies {
    pub relations: Vec<ProjectRelation>,
    pub completeness: DependencyCompleteness,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DependencyCompleteness { Complete, Partial, Unknown }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProjectRelation { pub project_id: u64, pub kind: RelationKind }
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RelationKind { Embedded, Optional, Required, Tool, Incompatible, Include, Unknown(u64) }
impl RelationKind {
    pub fn can_install(self) -> bool { matches!(self, Self::Optional | Self::Required | Self::Tool) }
    pub fn label(self) -> &'static str {
        match self {
            Self::Embedded => "Embedded library", Self::Optional => "Optional",
            Self::Required => "Required", Self::Tool => "Tool", Self::Incompatible => "Incompatible",
            Self::Include => "Included", Self::Unknown(_) => "Unknown relation",
        }
    }
}
fn parse_dependencies(value: Option<&Value>) -> FileDependencies {
    let Some(values) = value.and_then(Value::as_array) else {
        return FileDependencies { relations: vec![], completeness: DependencyCompleteness::Unknown };
    };
    let mut result = FileDependencies { relations: vec![], completeness: DependencyCompleteness::Complete };
    for value in values.iter().take(128) {
        let Some(project_id) = value["modId"].as_u64().filter(|id| *id > 0) else {
            result.completeness = DependencyCompleteness::Partial; continue;
        };
        let Some(kind) = value["relationType"].as_u64() else {
            result.completeness = DependencyCompleteness::Partial; continue;
        };
        let kind = match kind { 1 => RelationKind::Embedded, 2 => RelationKind::Optional,
            3 => RelationKind::Required, 4 => RelationKind::Tool, 5 => RelationKind::Incompatible,
            6 => RelationKind::Include, n => RelationKind::Unknown(n) };
        let relation = ProjectRelation { project_id, kind };
        if !result.relations.contains(&relation) { result.relations.push(relation); }
    }
    if values.len() > 128 { result.completeness = DependencyCompleteness::Partial; }
    result
}
impl BrowserRelease {
    /// Derived release navigation only; saved source remains the exact API project page.
    pub fn navigation_url(&self, project: &BrowserProject) -> String {
        format!(
            "{}/files/{}",
            project.source_url.trim_end_matches('/'),
            self.release.file_id
        )
    }
}
#[derive(Clone, Default)]
pub struct ImportPermit(Arc<AtomicU8>);
impl ImportPermit {
    /// False means publication already owns the permit and must finish or roll back.
    pub fn cancel(&self) -> bool {
        self.0
            .compare_exchange(0, 1, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
            || self.is_cancelled()
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire) == 1
    }
    pub fn is_publishing(&self) -> bool {
        self.0.load(Ordering::Acquire) == 2
    }
    fn check(&self) -> Result<(), ManagerError> {
        if self.is_cancelled() {
            Err(invalid("Browser import was cancelled"))
        } else {
            Ok(())
        }
    }
    fn begin_publication(&self) -> Result<(), ManagerError> {
        self.0
            .compare_exchange(0, 2, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| ())
            .map_err(|_| invalid("Browser import no longer has an active publication permit"))
    }
}
fn invalid(message: &str) -> ManagerError {
    ManagerError::new(ErrorCode::InvalidPath, message)
}
fn provider_error(error: SourceLookupError) -> ManagerError {
    let code = match error {
        SourceLookupError::MissingApiKey => ErrorCode::SourceMissingApiKey,
        SourceLookupError::Unauthorized => ErrorCode::SourceUnauthorized,
        SourceLookupError::RateLimited => ErrorCode::SourceRateLimited,
        SourceLookupError::Network => ErrorCode::SourceNetwork,
        SourceLookupError::InvalidResponse => ErrorCode::SourceInvalidResponse,
        SourceLookupError::ProviderUnavailable => ErrorCode::SourceProviderUnavailable,
        SourceLookupError::NoUsableEvidence => ErrorCode::SourceNoUsableEvidence,
    };
    ManagerError::new(code, "Could not read CurseForge browser download metadata")
}
fn response<T: CurseForgeTransport>(
    client: &CurseForgeClient<T>,
    path: &str,
) -> Result<Value, ManagerError> {
    let (status, body) = client.get_update_response(path).map_err(provider_error)?;
    classify_status(status).map_err(provider_error)?;
    serde_json::from_str(&body).map_err(|_| provider_error(SourceLookupError::InvalidResponse))
}
fn project_url(url: &str) -> bool {
    [
        "https://www.curseforge.com/sims4/",
        "https://curseforge.com/sims4/",
    ]
    .iter()
    .any(|prefix| url.starts_with(prefix) && url.len() > prefix.len())
        && !url.chars().any(|c| c.is_control() || c.is_whitespace())
        && !url.contains(['?', '#', '\\'])
}
fn supported_name(name: &str) -> bool {
    fs_scope::component(name).is_ok()
        && matches!(
            Path::new(name)
                .extension()
                .and_then(|x| x.to_str())
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some("zip" | "package" | "ts4script")
        )
}
fn parse_file(value: &Value, project_id: u64) -> Option<BrowserRelease> {
    let file_id = value["id"].as_u64().filter(|id| *id > 0)?;
    if value["modId"].as_u64() != Some(project_id)
        || value["releaseType"].as_u64() != Some(1)
        || value["isAvailable"].as_bool() != Some(true)
        || !matches!(value["fileStatus"].as_u64(), Some(4 | 10))
        || value
            .get("isEarlyAccessContent")
            .is_some_and(|v| v.as_bool() != Some(false))
    {
        return None;
    }
    let file_name = value["fileName"].as_str()?.to_string();
    if !supported_name(&file_name) {
        return None;
    }
    let length = value["fileLength"]
        .as_u64()
        .filter(|n| *n > 0 && *n <= MAX_DOWNLOAD_BYTES)?;
    let published_at = DateTime::parse_from_rfc3339(value["fileDate"].as_str()?)
        .ok()?
        .with_timezone(&Utc);
    let versions = value["gameVersions"]
        .as_array()?
        .iter()
        .map(|v| v.as_str().map(str::to_string))
        .collect::<Option<Vec<_>>>()?;
    let mut hashes = Vec::new();
    for hash in value["hashes"].as_array()? {
        let algorithm = hash["algo"].as_u64()?;
        if algorithm != 1 && algorithm != 2 {
            continue;
        }
        let digest = hash["value"].as_str()?.to_ascii_lowercase();
        if digest.len() != if algorithm == 1 { 40 } else { 32 }
            || !digest.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return None;
        }
        if hashes.iter().any(|(a, d)| *a == algorithm && *d != digest) {
            return None;
        }
        if !hashes.contains(&(algorithm, digest.clone())) {
            hashes.push((algorithm, digest));
        }
    }
    if hashes.is_empty() {
        return None;
    }
    hashes.sort();
    Some(BrowserRelease {
        release: Release {
            project_id,
            file_id,
            name: value["displayName"]
                .as_str()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or(&file_name)
                .into(),
            file_name,
            published_at,
            game_versions: versions,
        },
        length,
        dependencies: parse_dependencies(value.get("dependencies")),
        hashes,
    })
}
pub fn load_browser_project<T: CurseForgeTransport>(
    client: &CurseForgeClient<T>,
    project_id: u64,
) -> Result<BrowserProject, ManagerError> {
    if project_id == 0 {
        return Err(provider_error(SourceLookupError::InvalidResponse));
    }
    let project = response(client, &format!("mods/{project_id}"))?;
    let data = &project["data"];
    let source_url = data["links"]["websiteUrl"].as_str().unwrap_or_default();
    let title = data["name"].as_str().filter(|s| !s.trim().is_empty());
    if data["id"].as_u64() != Some(project_id)
        || data["gameId"].as_u64() != Some(u64::from(SIMS4_GAME_ID))
        || !project_url(source_url)
        || title.is_none()
    {
        return Err(provider_error(SourceLookupError::InvalidResponse));
    }
    let author = data["authors"]
        .as_array()
        .map(|authors| {
            authors
                .iter()
                .filter_map(|a| a["name"].as_str().filter(|s| !s.trim().is_empty()))
                .collect::<Vec<_>>()
                .join(", ")
        })
        .filter(|a| !a.is_empty());
    let preview_url = data["logo"]["url"]
        .as_str()
        .filter(|s| {
            s.starts_with("https://") && !s.chars().any(|c| c.is_control() || c.is_whitespace())
        })
        .map(str::to_string);
    let mut result = BrowserProject {
        project_id,
        title: title.unwrap().into(),
        author,
        source_url: source_url.into(),
        preview_url,
        releases: Vec::new(),
        complete: false,
    };
    let mut seen = HashSet::new();
    let mut index = 0;
    for _ in 0..10 {
        let page = response(
            client,
            &format!("mods/{project_id}/files?pageSize=50&index={index}"),
        )?;
        let Some(files) = page["data"].as_array() else {
            return Err(provider_error(SourceLookupError::InvalidResponse));
        };
        let pagination = &page["pagination"];
        let count = files.len() as u64;
        if count > 50
            || pagination["index"].as_u64() != Some(index)
            || pagination["pageSize"].as_u64() != Some(50)
            || pagination["resultCount"].as_u64() != Some(count)
        {
            break;
        }
        let Some(total) = pagination["totalCount"].as_u64() else {
            break;
        };
        if total < index + count {
            break;
        }
        let mut duplicate = false;
        for value in files {
            if let Some(id) = value["id"].as_u64() {
                if !seen.insert(id) {
                    duplicate = true;
                    result.releases.retain(|f| f.release.file_id != id);
                    continue;
                }
            }
            if let Some(file) = parse_file(value, project_id) {
                result.releases.push(file);
            }
        }
        if duplicate {
            break;
        }
        index += count;
        if index == total {
            result.complete = true;
            break;
        }
        if count == 0 {
            break;
        }
    }
    result.releases.sort_by(|a, b| {
        b.release
            .published_at
            .cmp(&a.release.published_at)
            .then(a.release.file_id.cmp(&b.release.file_id))
    });
    Ok(result)
}

#[cfg(unix)]
#[derive(Clone, PartialEq, Eq)]
struct Stamp {
    device: u64,
    inode: u64,
    length: u64,
    modified: (i64, i64),
    changed: (i64, i64),
}
#[cfg(unix)]
fn stamp(file: &fs::File) -> Result<Stamp, ManagerError> {
    use std::os::unix::fs::MetadataExt;
    let meta = file.metadata().map_err(fs_scope::io_error)?;
    if !meta.is_file() {
        return Err(invalid("Download is not a regular file"));
    }
    Ok(Stamp {
        device: meta.dev(),
        inode: meta.ino(),
        length: meta.len(),
        modified: (meta.mtime(), meta.mtime_nsec()),
        changed: (meta.ctime(), meta.ctime_nsec()),
    })
}
#[derive(Clone)]
pub struct MatchedDownload(Arc<Mutex<Option<fs::File>>>);
#[cfg(unix)]
pub struct DownloadWatch {
    directory: fs_scope::Directory,
    release: BrowserRelease,
    previous: HashMap<OsString, Stamp>,
    rejected: HashMap<OsString, Stamp>,
}
#[cfg(unix)]
impl DownloadWatch {
    pub fn new(directory: &Path, release: &BrowserRelease) -> Result<Self, ManagerError> {
        Ok(Self {
            directory: fs_scope::Directory::open(directory)?,
            release: release.clone(),
            previous: HashMap::new(),
            rejected: HashMap::new(),
        })
    }
    pub fn poll(&mut self, permit: &ImportPermit) -> Result<Option<MatchedDownload>, ManagerError> {
        permit.check()?;
        let names = self.directory.entry_names(1024)?;
        self.previous.retain(|name, _| names.contains(name));
        self.rejected.retain(|name, _| names.contains(name));
        let mut hashed = 0;
        for name in names {
            permit.check()?;
            // Browser temporary suffixes and unrelated extensions never become candidates.
            if !supported_name(&name.to_string_lossy()) {
                continue;
            }
            let Ok(mut file) = self.directory.open_regular(Path::new(&name)) else {
                continue;
            };
            let current = stamp(&file)?;
            let stable = self.previous.get(&name) == Some(&current);
            self.previous.insert(name.clone(), current.clone());
            if !stable
                || current.length != self.release.length
                || self.rejected.get(&name) == Some(&current)
                || hashed >= 8
            {
                continue;
            }
            hashed += 1;
            if verify_stream(&mut file, None, &self.release, permit).is_ok()
                && stamp(&file)? == current
            {
                return Ok(Some(MatchedDownload(Arc::new(Mutex::new(Some(file))))));
            }
            permit.check()?;
            self.rejected.insert(name, current);
        }
        Ok(None)
    }
}
fn verify_stream(
    file: &mut fs::File,
    mut output: Option<&mut fs::File>,
    release: &BrowserRelease,
    permit: &ImportPermit,
) -> Result<(), ManagerError> {
    file.seek(SeekFrom::Start(0)).map_err(fs_scope::io_error)?;
    let mut sha1 = Sha1::new();
    let mut md5 = Md5::new();
    let mut bytes = 0u64;
    let mut buffer = [0u8; 65536];
    loop {
        permit.check()?;
        let count = file.read(&mut buffer).map_err(fs_scope::io_error)?;
        if count == 0 {
            break;
        }
        bytes += count as u64;
        if bytes > release.length {
            return Err(invalid("Downloaded file length changed"));
        }
        sha1.update(&buffer[..count]);
        md5.update(&buffer[..count]);
        if let Some(out) = output.as_deref_mut() {
            out.write_all(&buffer[..count])
                .map_err(fs_scope::io_error)?;
        }
    }
    let hashes = [
        (1, format!("{:x}", sha1.finalize())),
        (2, format!("{:x}", md5.finalize())),
    ];
    if bytes != release.length
        || !release
            .hashes
            .iter()
            .all(|expected| hashes.contains(expected))
    {
        return Err(invalid(
            "Downloaded file does not match every provider hash and byte length",
        ));
    }
    Ok(())
}

fn existing_release(
    root: &Path,
    project: &BrowserProject,
    release: &BrowserRelease,
) -> Result<Option<ModMetadata>, ManagerError> {
    let mods = root.join("mods");
    if !fs_scope::exists(&mods) {
        return Ok(None);
    }
    let directory = fs_scope::Directory::open(&mods)?;
    for name in directory.entry_names(100_000)? {
        let Some(id) = name.to_str() else {
            continue;
        };
        let Ok(meta) = managed_storage::read_managed_mod(root, id) else {
            continue;
        };
        let Some(source) = &meta.source_attachment else {
            continue;
        };
        let Some(installed) = &source.installed_release else {
            continue;
        };
        if source.provider_id == "curseforge"
            && source.project_id == Some(project.project_id)
            && installed.project_id == project.project_id
            && installed.file_id == release.release.file_id
        {
            let digest =
                fs_scope::Directory::open(&mods.join(id).join("files"))?.mod_content_digest()?;
            if digest != installed.content_digest
                || source.source_url != project.source_url
                || meta.source_url.as_deref() != Some(&project.source_url)
            {
                return Err(invalid("This release is already managed but its source or content changed; review it before importing again"));
            }
            return Ok(Some(meta));
        }
    }
    Ok(None)
}
fn enable_plan(root: &Path, game: &Path, id: &str) -> Result<toggle::DryRunResult, ManagerError> {
    let mut dry = toggle::dry_run_toggle(root, game, id, true)?;
    let mut owned = HashSet::new();
    for op in &mut dry.operations {
        if op.action == "skip" && link_ownership::owned(root, id, game, &op.path).unwrap_or(false) {
            owned.insert(op.path.clone());
            op.action = "keep_symlink".into();
            op.reason = None;
        }
    }
    dry.issues
        .retain(|issue| !owned.iter().any(|p| issue.id == format!("collision:{p}")));
    dry.can_apply = !dry.operations.iter().any(|op| op.action == "skip");
    if !dry.can_apply {
        return Err(ManagerError::new(
            ErrorCode::PathCollision,
            "Game Mods paths are occupied; existing files were preserved",
        ));
    }
    Ok(dry)
}

#[cfg(unix)]
pub fn import_browser_download(
    root: &Path,
    game: &Path,
    project: &BrowserProject,
    release: &BrowserRelease,
    matched: &MatchedDownload,
    permit: &ImportPermit,
) -> Result<ModMetadata, ManagerError> {
    let mut source = matched
        .0
        .lock()
        .map_err(|_| invalid("Downloaded file handle unavailable"))?
        .take()
        .ok_or_else(|| invalid("Downloaded file was already consumed; start a new watch"))?;
    permit.check()?;
    if release.release.project_id != project.project_id
        || !project.releases.contains(release)
        || !project_url(&project.source_url)
    {
        return Err(invalid(
            "Chosen release does not belong to the confirmed provider project",
        ));
    }
    let original = stamp(&source)?;
    let _game_directory = fs_scope::Directory::open(game)?;
    let _guard = crate::operation::acquire(root)?;
    fs_scope::create_dir_all(&root.join("tmp"))?;
    let snapshot = tempfile::Builder::new()
        .prefix("browser-import-")
        .tempdir_in(root.join("tmp"))
        .map_err(fs_scope::io_error)?;
    let snapshot_path = snapshot.path().join(&release.release.file_name);
    let mut snapshot_file = fs_scope::Directory::open(snapshot.path())?
        .create_file(Path::new(&release.release.file_name))?;
    verify_stream(&mut source, Some(&mut snapshot_file), release, permit)?;
    snapshot_file.sync_all().map_err(fs_scope::io_error)?;
    if stamp(&source)? != original {
        return Err(invalid("Download changed while copying; start a new watch"));
    }
    drop(source);
    drop(snapshot_file);
    if let Some(meta) = existing_release(root, project, release)? {
        let dry = enable_plan(root, game, &meta.mod_id)?;
        permit.begin_publication()?;
        if dry.operations.iter().all(|op| op.action == "keep_symlink") {
            return Ok(meta);
        }
        let mut transaction = Transaction::new(root, "enable")?;
        if let Err(error) =
            toggle::apply_toggle_transaction(root, game, &meta.mod_id, true, &dry, &mut transaction)
        {
            transaction.rollback()?;
            return Err(error);
        }
        transaction.commit()?;
        return Ok(meta);
    }
    permit.check()?;
    let source_dir = snapshot.path().join("content");
    fs_scope::create_dir_all(&source_dir)?;
    if Path::new(&release.release.file_name)
        .extension()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("zip"))
    {
        crate::archive_import::extract_archive_checked(&snapshot_path, &source_dir, || {
            permit.check()
        })?;
    } else {
        let mut input = fs::File::open(&snapshot_path).map_err(fs_scope::io_error)?;
        let mut output = fs_scope::Directory::open(&source_dir)?
            .create_file(Path::new(&release.release.file_name))?;
        verify_stream(&mut input, Some(&mut output), release, permit)?;
        output.sync_all().map_err(fs_scope::io_error)?;
    }
    permit.check()?;
    let mut transaction = Transaction::new(root, "folder_import")?;
    let result = (|| {
        let meta = managed_storage::create_managed_mod_transaction_initialized(
            root,
            ImportRequest {
                name: project.title.clone(),
                slug: None,
                source_dir,
            },
            &mut transaction,
            |meta, files_root| {
                permit.check()?;
                if !meta.files.iter().any(|name| {
                    matches!(
                        Path::new(name)
                            .extension()
                            .and_then(|x| x.to_str())
                            .map(str::to_ascii_lowercase)
                            .as_deref(),
                        Some("package" | "ts4script")
                    )
                }) {
                    return Err(invalid(
                        "Archive contains no Sims 4 package or script files",
                    ));
                }
                let installed = InstalledRelease {
                    project_id: project.project_id,
                    file_id: release.release.file_id,
                    content_digest: fs_scope::Directory::open(files_root)?.mod_content_digest()?,
                    confirmed_at: Utc::now().to_rfc3339(),
                };
                meta.source = "curseforge".into();
                meta.source_url = Some(project.source_url.clone());
                meta.preview_url = project.preview_url.clone();
                meta.source_attachment = Some(SourceAttachmentMetadata { provider_id: "curseforge".into(), project_id: Some(project.project_id), file_id: Some(release.release.file_id), installed_release: Some(installed), source_url: project.source_url.clone(), title: project.title.clone(), author: project.author.clone(), confidence: Some(100), reasons: vec!["User confirmed browser import; downloaded bytes matched the selected provider release".into()], evidence: vec![SourceEvidence { kind: "file_hash".into(), description: format!("Verified {} provider hash(es) and {} bytes for file {}", release.hashes.len(), release.length, release.release.file_id), weight: 100 }], attached_by: "user".into(), attached_at: Utc::now().to_rfc3339() });
                // Validate collision paths before publishing; the actual toggle rechecks under the same authority.
                for name in &meta.files {
                    fs_scope::ensure_no_symlink_parents(game, Path::new(name))?;
                    if fs_scope::exists(&game.join(name)) {
                        return Err(ManagerError::new(
                            ErrorCode::PathCollision,
                            "Game Mods path is occupied; original files were preserved",
                        ));
                    }
                }
                Ok(())
            },
            || permit.begin_publication(),
            || permit.check(),
        )?;
        let dry = enable_plan(root, game, &meta.mod_id)?;
        toggle::apply_toggle_transaction(root, game, &meta.mod_id, true, &dry, &mut transaction)?;
        Ok(meta)
    })();
    match result {
        Ok(meta) => {
            transaction.commit()?;
            Ok(meta)
        }
        Err(error) => {
            transaction.rollback()?;
            Err(error)
        }
    }
}

#[cfg(test)]
mod permit_tests {
    use super::*;
    #[test]
    fn cancellation_and_publication_have_exactly_one_winner() {
        for _ in 0..100 {
            let permit = ImportPermit::default();
            let barrier = Arc::new(std::sync::Barrier::new(2));
            let other = permit.clone();
            let thread_barrier = barrier.clone();
            let cancel = std::thread::spawn(move || {
                thread_barrier.wait();
                other.cancel()
            });
            barrier.wait();
            let publishing = permit.begin_publication().is_ok();
            assert_ne!(publishing, cancel.join().unwrap());
            assert_eq!(permit.is_publishing(), publishing);
            assert_eq!(permit.is_cancelled(), !publishing);
            assert!(permit.begin_publication().is_err());
        }
    }
}
