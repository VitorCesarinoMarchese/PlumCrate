use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{ErrorCode, ManagerError};
use crate::fs_scope;
use crate::metadata_names::detect_display_name;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceEvidence {
    pub kind: String,
    pub description: String,
    pub weight: i16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceAttachmentMetadata {
    pub provider_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub installed_release: Option<crate::mod_updates::InstalledRelease>,
    pub source_url: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub confidence: Option<u8>,
    #[serde(default)]
    pub reasons: Vec<String>,
    #[serde(default)]
    pub evidence: Vec<SourceEvidence>,
    pub attached_by: String,
    pub attached_at: String,
}

#[derive(Debug, Clone)]
pub struct ImportRequest {
    pub name: String,
    pub slug: Option<String>,
    pub source_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModMetadata {
    pub version: u32,
    pub created_by: String,
    pub mod_id: String,
    pub name: String,
    #[serde(default)]
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detected_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_name: Option<String>,
    pub slug: Option<String>,
    pub files: Vec<String>,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_preview_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_attachment: Option<SourceAttachmentMetadata>,
    #[serde(default)]
    pub locked_name: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
}

impl ModMetadata {
    pub fn effective_display_name(&self) -> &str {
        self.custom_name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .or_else(|| {
                self.detected_name
                    .as_deref()
                    .filter(|name| !name.trim().is_empty())
            })
            .or_else(|| {
                (!self.display_name.trim().is_empty()).then_some(self.display_name.as_str())
            })
            .unwrap_or(&self.name)
    }
}

pub fn create_managed_mod(
    managed_root: &Path,
    req: ImportRequest,
) -> Result<ModMetadata, ManagerError> {
    let _guard = crate::operation::acquire(managed_root)?;
    let mut transaction = crate::operation::Transaction::new(managed_root, "folder_import")?;
    let metadata = match create_managed_mod_transaction(managed_root, req, &mut transaction) {
        Ok(metadata) => metadata,
        Err(error) => {
            transaction.rollback()?;
            return Err(error);
        }
    };
    transaction.commit()?;
    Ok(metadata)
}

pub(crate) fn create_managed_mod_transaction(
    managed_root: &Path,
    req: ImportRequest,
    transaction: &mut crate::operation::Transaction,
) -> Result<ModMetadata, ManagerError> {
    create_managed_mod_transaction_initialized(
        managed_root,
        req,
        transaction,
        |_, _| Ok(()),
        || Ok(()),
        || Ok(()),
    )
}

pub(crate) fn create_managed_mod_transaction_initialized(
    managed_root: &Path,
    req: ImportRequest,
    transaction: &mut crate::operation::Transaction,
    initialize: impl FnOnce(&mut ModMetadata, &Path) -> Result<(), ManagerError>,
    before_publish: impl FnOnce() -> Result<(), ManagerError>,
    check: impl Fn() -> Result<(), ManagerError>,
) -> Result<ModMetadata, ManagerError> {
    if !req.source_dir.is_dir() {
        return Err(ManagerError::new(
            ErrorCode::InvalidPath,
            format!("Import source dir invalid: {}", req.source_dir.display()),
        ));
    }

    let mod_id = Uuid::new_v4().to_string();
    fs_scope::create_dir_all(managed_root)?;
    fs_scope::ensure_no_symlink_parents(managed_root, Path::new("mods/staging"))?;
    let final_root = managed_root.join("mods").join(&mod_id);
    let mod_root = managed_root.join("tmp").join(format!("import-{mod_id}"));
    let files_root = mod_root.join("files");

    fs_scope::create_dir_all(&files_root)?;
    fs_scope::create_dir_all(&mod_root.join("links"))?;

    fs_scope::Directory::open(&req.source_dir)?;
    let files = copy_recursive_checked(&req.source_dir, &files_root, &check)?;

    let name = req.name;
    let detected_name = detect_display_name(&name);
    let mut meta = ModMetadata {
        version: 1,
        created_by: "sims4-mod-manager".to_string(),
        mod_id: mod_id.clone(),
        display_name: detected_name.clone(),
        detected_name: Some(detected_name),
        custom_name: None,
        name,
        slug: req.slug,
        files,
        source: "local".to_string(),
        source_url: None,
        preview_url: None,
        local_preview_path: None,
        source_attachment: None,
        locked_name: false,
        updated_at: None,
    };

    initialize(&mut meta, &files_root)?;
    fs_scope::atomic_write(
        &mod_root.join("meta.json"),
        &serde_json::to_vec_pretty(&meta)
            .map_err(|e| ManagerError::new(ErrorCode::InternalError, e.to_string()))?,
    )?;
    fs_scope::create_dir_all(final_root.parent().unwrap())?;
    before_publish()?;
    transaction.move_path(&mod_root, &final_root)?;

    Ok(meta)
}

pub fn write_managed_mod(managed_root: &Path, meta: &ModMetadata) -> Result<(), ManagerError> {
    let _guard = crate::operation::acquire(managed_root)?;
    validate_metadata(meta, &meta.mod_id)?;
    let rel = PathBuf::from("mods").join(&meta.mod_id).join("meta.json");
    fs_scope::ensure_no_symlink_parents(managed_root, &rel)?;
    let meta_path = managed_root.join(rel);
    fs_scope::create_dir_all(meta_path.parent().unwrap())?;
    fs_scope::atomic_write(
        &meta_path,
        &serde_json::to_vec_pretty(meta)
            .map_err(|e| ManagerError::new(ErrorCode::InternalError, e.to_string()))?,
    )
}

fn validate_metadata(meta: &ModMetadata, id: &str) -> Result<(), ManagerError> {
    fs_scope::component(id)?;
    if meta.version != 1 || meta.mod_id != id {
        return Err(ManagerError::new(
            ErrorCode::InvalidPath,
            "Metadata identity or version mismatch",
        ));
    }
    for file in &meta.files {
        fs_scope::relative(Path::new(file))?;
    }
    Ok(())
}

pub fn read_managed_mod(managed_root: &Path, mod_id: &str) -> Result<ModMetadata, ManagerError> {
    fs_scope::component(mod_id)?;
    let rel = PathBuf::from("mods").join(mod_id).join("meta.json");
    fs_scope::ensure_no_symlink_parents(managed_root, &rel)?;
    let meta_path = managed_root.join(rel);
    if fs::symlink_metadata(&meta_path).is_ok_and(|m| m.file_type().is_symlink()) {
        return Err(ManagerError::new(
            ErrorCode::InvalidPath,
            "Metadata is a symlink",
        ));
    }

    let content = fs::read_to_string(&meta_path).map_err(|_| {
        ManagerError::new(
            ErrorCode::NotFound,
            format!("Metadata not found: {}", meta_path.display()),
        )
    })?;

    let meta: ModMetadata = serde_json::from_str(&content).map_err(|e| {
        ManagerError::new(ErrorCode::InvalidPath, format!("Metadata malformed: {e}"))
    })?;

    if meta.version != 1 {
        return Err(ManagerError::new(
            ErrorCode::InvalidPath,
            format!("Unsupported metadata version: {}", meta.version),
        ));
    }

    validate_metadata(&meta, mod_id)?;
    Ok(meta)
}

fn read_managed_mod_or_create_local(
    managed_root: &Path,
    mod_id: &str,
) -> Result<ModMetadata, ManagerError> {
    read_managed_mod(managed_root, mod_id)
}

pub fn set_custom_display_name(
    managed_root: &Path,
    mod_id: &str,
    custom_name: String,
) -> Result<ModMetadata, ManagerError> {
    let _guard = crate::operation::acquire(managed_root)?;
    let mut meta = read_managed_mod_or_create_local(managed_root, mod_id)?;
    let trimmed = custom_name.trim();
    if trimmed.is_empty() {
        return Err(ManagerError::new(
            ErrorCode::InvalidPath,
            "Display name cannot be empty",
        ));
    }

    meta.custom_name = Some(trimmed.to_string());
    meta.display_name = trimmed.to_string();
    write_managed_mod(managed_root, &meta)?;
    Ok(meta)
}

pub fn set_source_url(
    managed_root: &Path,
    mod_id: &str,
    source_url: String,
    provider_id: Option<String>,
    display_name: Option<String>,
    preview_url: Option<String>,
    source_attachment: Option<SourceAttachmentMetadata>,
) -> Result<ModMetadata, ManagerError> {
    let _guard = crate::operation::acquire(managed_root)?;
    let mut meta = read_managed_mod_or_create_local(managed_root, mod_id)?;
    let trimmed = source_url.trim();
    if trimmed.is_empty() {
        return Err(ManagerError::new(
            ErrorCode::InvalidPath,
            "Source URL cannot be empty",
        ));
    }

    let normalized_provider = provider_id.filter(|provider| !provider.trim().is_empty());
    let mut source_attachment = source_attachment;
    if let Some(attachment) = source_attachment.as_mut() {
        let provider = normalized_provider
            .as_ref()
            .unwrap_or(&attachment.provider_id);
        attachment.installed_release = meta
            .source_attachment
            .as_ref()
            .filter(|previous| {
                previous.provider_id == *provider
                    && previous.project_id == attachment.project_id
                    && previous.source_url == trimmed
            })
            .and_then(|previous| previous.installed_release.clone());
    }

    meta.source_url = Some(trimmed.to_string());
    if let Some(provider) = normalized_provider.clone() {
        meta.source = provider;
    }
    if let Some(title) = display_name
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty())
    {
        meta.detected_name = Some(title.clone());
        if meta
            .custom_name
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
        {
            meta.display_name = title;
        }
    }
    meta.preview_url = preview_url
        .map(|url| url.trim().to_string())
        .filter(|url| !url.is_empty());
    meta.source_attachment = source_attachment.map(|mut attachment| {
        attachment.source_url = trimmed.to_string();
        if let Some(provider) = normalized_provider {
            attachment.provider_id = provider;
        }
        attachment
    });
    write_managed_mod(managed_root, &meta)?;
    Ok(meta)
}

pub fn remove_source_url(managed_root: &Path, mod_id: &str) -> Result<ModMetadata, ManagerError> {
    let _guard = crate::operation::acquire(managed_root)?;
    let mut meta = read_managed_mod_or_create_local(managed_root, mod_id)?;
    meta.source_url = None;
    meta.source_attachment = None;
    meta.source = "local".to_string();
    write_managed_mod(managed_root, &meta)?;
    Ok(meta)
}

pub fn read_managed_mod_or_default(
    managed_root: &Path,
    mod_id: &str,
    detected_name: &str,
) -> ModMetadata {
    read_managed_mod(managed_root, mod_id).unwrap_or_else(|_| ModMetadata {
        version: 1,
        created_by: "sims4-mod-manager".to_string(),
        mod_id: mod_id.to_string(),
        name: detected_name.to_string(),
        display_name: detected_name.to_string(),
        detected_name: Some(detected_name.to_string()),
        custom_name: None,
        slug: None,
        files: vec![],
        source: "local".to_string(),
        source_url: None,
        preview_url: None,
        local_preview_path: None,
        source_attachment: None,
        locked_name: false,
        updated_at: None,
    })
}

fn copy_recursive_checked(
    from: &Path,
    to: &Path,
    check: &impl Fn() -> Result<(), ManagerError>,
) -> Result<Vec<String>, ManagerError> {
    let mut stack = vec![from.to_path_buf()];
    let mut collected = vec![];

    while let Some(dir) = stack.pop() {
        check()?;
        let entries = fs::read_dir(&dir).map_err(|e| {
            ManagerError::new(
                ErrorCode::IoError,
                format!("Read dir failed {}: {e}", dir.display()),
            )
        })?;

        for entry in entries {
            check()?;
            let entry = entry.map_err(fs_scope::io_error)?;
            let src = entry.path();
            let meta = fs::symlink_metadata(&src).map_err(|e| {
                ManagerError::new(
                    ErrorCode::IoError,
                    format!("Metadata failed {}: {e}", src.display()),
                )
            })?;

            if meta.is_dir() {
                stack.push(src);
                continue;
            }

            if !meta.is_file() {
                return Err(ManagerError::new(
                    ErrorCode::InvalidPath,
                    "Import contains a symlink or special file",
                ));
            }

            let rel = src.strip_prefix(from).map_err(|e| {
                ManagerError::new(
                    ErrorCode::InternalError,
                    format!("Relative path failed {}: {e}", src.display()),
                )
            })?;

            let dst = to.join(rel);
            if let Some(parent) = dst.parent() {
                fs_scope::create_dir_all(parent)?;
            }

            #[cfg(unix)]
            let mut input = {
                use std::os::unix::fs::OpenOptionsExt;
                fs::OpenOptions::new()
                    .read(true)
                    .custom_flags(libc::O_NOFOLLOW)
                    .open(&src)
                    .map_err(fs_scope::io_error)?
            };
            #[cfg(not(unix))]
            let mut input = fs::File::open(&src).map_err(fs_scope::io_error)?;
            let mut output = fs_scope::create_file(to, rel)?;
            let mut buffer = [0; 65536];
            loop {
                check()?;
                let count =
                    std::io::Read::read(&mut input, &mut buffer).map_err(fs_scope::io_error)?;
                if count == 0 {
                    break;
                }
                std::io::Write::write_all(&mut output, &buffer[..count])
                    .map_err(fs_scope::io_error)?;
            }
            output.sync_all().map_err(fs_scope::io_error)?;

            collected.push(rel.to_string_lossy().replace('\\', "/"));
        }
    }

    collected.sort();
    Ok(collected)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::{
        create_managed_mod, read_managed_mod, read_managed_mod_or_default, write_managed_mod,
        ImportRequest, SourceAttachmentMetadata, SourceEvidence,
    };

    #[test]
    fn creates_managed_mod_with_uuid_directory() {
        let tmp = TempDir::new().expect("tmp");
        let src = tmp.path().join("import");
        fs::create_dir_all(&src).expect("src");
        fs::write(src.join("a.package"), b"x").expect("file");

        let meta = create_managed_mod(
            tmp.path(),
            ImportRequest {
                name: "Test".to_string(),
                slug: Some("test".to_string()),
                source_dir: src,
            },
        )
        .expect("create");

        assert_eq!(meta.mod_id.len(), 36);
        assert!(tmp
            .path()
            .join("mods")
            .join(&meta.mod_id)
            .join("files")
            .is_dir());
    }

    #[test]
    fn writes_metadata_with_version_one() {
        let tmp = TempDir::new().expect("tmp");
        let src = tmp.path().join("import");
        fs::create_dir_all(&src).expect("src");
        fs::write(src.join("a.package"), b"x").expect("file");

        let meta = create_managed_mod(
            tmp.path(),
            ImportRequest {
                name: "Test".to_string(),
                slug: None,
                source_dir: src,
            },
        )
        .expect("create");

        let meta_path = tmp.path().join("mods").join(&meta.mod_id).join("meta.json");
        let raw = fs::read_to_string(meta_path).expect("read");
        assert!(raw.contains("\"version\": 1"));
    }

    #[test]
    fn manifest_includes_non_mod_assets() {
        let tmp = TempDir::new().expect("tmp");
        let src = tmp.path().join("import");
        fs::create_dir_all(src.join("docs")).expect("src");
        fs::write(src.join("a.package"), b"x").expect("file");
        fs::write(src.join("docs/readme.txt"), b"txt").expect("file");

        let meta = create_managed_mod(
            tmp.path(),
            ImportRequest {
                name: "Assets".to_string(),
                slug: None,
                source_dir: src,
            },
        )
        .expect("create");

        assert!(meta.files.contains(&"a.package".to_string()));
        assert!(meta.files.contains(&"docs/readme.txt".to_string()));
    }

    #[test]
    fn read_validates_supported_schema_version() {
        let tmp = TempDir::new().expect("tmp");
        let base = tmp.path().join("mods/abc");
        fs::create_dir_all(&base).expect("base");
        fs::write(
            base.join("meta.json"),
            r#"{"version":1,"createdBy":"sims4-mod-manager","modId":"abc","name":"X","slug":null,"files":[],"source":"managed"}"#,
        )
        .expect("meta");

        let meta = read_managed_mod(tmp.path(), "abc").expect("read");
        assert_eq!(meta.version, 1);
    }

    #[test]
    fn reads_and_writes_extended_metadata_fields() {
        let tmp = TempDir::new().expect("tmp");
        let src = tmp.path().join("import");
        fs::create_dir_all(&src).expect("src");
        fs::write(src.join("a.package"), b"x").expect("file");

        let mut meta = create_managed_mod(
            tmp.path(),
            ImportRequest {
                name: "Detected Name".to_string(),
                slug: None,
                source_dir: src,
            },
        )
        .expect("create");

        meta.detected_name = Some("Detected Name".to_string());
        meta.custom_name = Some("Custom Name".to_string());
        meta.source_url = Some("https://example.test/mod".to_string());
        write_managed_mod(tmp.path(), &meta).expect("write");

        let read = read_managed_mod(tmp.path(), &meta.mod_id).expect("read");
        assert_eq!(read.detected_name.as_deref(), Some("Detected Name"));
        assert_eq!(read.custom_name.as_deref(), Some("Custom Name"));
        assert_eq!(read.source_url.as_deref(), Some("https://example.test/mod"));
    }

    #[test]
    fn removes_source_url_from_metadata() {
        let tmp = TempDir::new().expect("tmp");
        let src = tmp.path().join("import");
        fs::create_dir_all(&src).expect("src");
        fs::write(src.join("a.package"), b"x").expect("file");
        let meta = create_managed_mod(
            tmp.path(),
            ImportRequest {
                name: "Detected".to_string(),
                slug: None,
                source_dir: src,
            },
        )
        .expect("create");
        super::set_source_url(
            tmp.path(),
            &meta.mod_id,
            "https://www.curseforge.com/sims4/mods/example".to_string(),
            Some("curseforge".to_string()),
            None,
            None,
            None,
        )
        .expect("attach");

        let updated = super::remove_source_url(tmp.path(), &meta.mod_id).expect("remove");

        assert_eq!(updated.source_url, None);
        assert_eq!(updated.source, "local");
        assert_eq!(updated.files, vec!["a.package"]);
    }

    #[test]
    fn attaches_source_url_by_imported_bundle_name_when_id_is_missing() {
        let tmp = TempDir::new().expect("tmp");
        let src = tmp
            .path()
            .join("import")
            .join("McCmdCenter_AllModules_2026_2_0");
        fs::create_dir_all(&src).expect("src");
        fs::write(src.join("mc_cmd_center.package"), b"x").expect("file");
        create_managed_mod(
            tmp.path(),
            ImportRequest {
                name: "McCmdCenter_AllModules_2026_2_0".to_string(),
                slug: None,
                source_dir: src.parent().expect("parent").to_path_buf(),
            },
        )
        .expect("create");

        let error = super::set_source_url(
            tmp.path(),
            "McCmdCenter_AllModules_2026_2_0",
            "https://www.curseforge.com/sims4/mods/mc-command-center".into(),
            Some("curseforge".into()),
            None,
            None,
            None,
        )
        .expect_err("Missing bundle must not create metadata");
        assert_eq!(error.code, crate::error::ErrorCode::NotFound);
        assert!(!tmp
            .path()
            .join("mods/McCmdCenter_AllModules_2026_2_0")
            .exists());
    }

    #[test]
    fn attaches_source_url_in_metadata() {
        let tmp = TempDir::new().expect("tmp");
        let src = tmp.path().join("import");
        fs::create_dir_all(&src).expect("src");
        fs::write(src.join("a.package"), b"x").expect("file");
        let meta = create_managed_mod(
            tmp.path(),
            ImportRequest {
                name: "Detected".to_string(),
                slug: None,
                source_dir: src,
            },
        )
        .expect("create");

        let updated = super::set_source_url(
            tmp.path(),
            &meta.mod_id,
            "https://www.curseforge.com/sims4/mods/example".to_string(),
            Some("curseforge".to_string()),
            None,
            None,
            None,
        )
        .expect("attach");

        assert_eq!(
            updated.source_url.as_deref(),
            Some("https://www.curseforge.com/sims4/mods/example")
        );
        assert_eq!(updated.source, "curseforge");
        assert_eq!(
            read_managed_mod(tmp.path(), &meta.mod_id)
                .expect("read")
                .source_url
                .as_deref(),
            Some("https://www.curseforge.com/sims4/mods/example")
        );
    }

    #[test]
    fn attaches_resolved_source_metadata() {
        let tmp = TempDir::new().expect("tmp");
        let src = tmp.path().join("import");
        fs::create_dir_all(&src).expect("src");
        fs::write(src.join("a.package"), b"x").expect("file");
        let meta = create_managed_mod(
            tmp.path(),
            ImportRequest {
                name: "Detected".to_string(),
                slug: None,
                source_dir: src,
            },
        )
        .expect("create");

        let updated = super::set_source_url(
            tmp.path(),
            &meta.mod_id,
            "https://modthesims.info/d/123456/example".to_string(),
            Some("modthesims".to_string()),
            Some("Real Mod Title".to_string()),
            Some("https://static.modthesims.info/cover.jpg".to_string()),
            None,
        )
        .expect("attach");

        assert_eq!(updated.source, "modthesims");
        assert_eq!(updated.display_name, "Real Mod Title");
        assert_eq!(
            updated.preview_url.as_deref(),
            Some("https://static.modthesims.info/cover.jpg")
        );
    }

    #[test]
    fn attaches_richer_source_attachment_metadata() {
        let tmp = TempDir::new().expect("tmp");
        let src = tmp.path().join("import");
        fs::create_dir_all(&src).expect("src");
        fs::write(src.join("a.package"), b"x").expect("file");
        let meta = create_managed_mod(
            tmp.path(),
            ImportRequest {
                name: "Detected".to_string(),
                slug: None,
                source_dir: src,
            },
        )
        .expect("create");

        let attachment = SourceAttachmentMetadata {
            provider_id: "curseforge".to_string(),
            project_id: Some(551680),
            file_id: Some(67890),
            installed_release: None,
            source_url: "https://www.curseforge.com/sims4/mods/mc-command-center".to_string(),
            title: "MC Command Center".to_string(),
            author: Some("Deaderpool".to_string()),
            confidence: Some(95),
            reasons: vec!["Exact fingerprint match".to_string()],
            evidence: vec![SourceEvidence {
                kind: "fingerprint".to_string(),
                description: "Exact fingerprint match".to_string(),
                weight: 95,
            }],
            attached_by: "user".to_string(),
            attached_at: "2026-05-13T12:34:56Z".to_string(),
        };

        let updated = super::set_source_url(
            tmp.path(),
            &meta.mod_id,
            attachment.source_url.clone(),
            Some(attachment.provider_id.clone()),
            Some(attachment.title.clone()),
            None,
            Some(attachment.clone()),
        )
        .expect("attach");

        assert_eq!(updated.source_attachment.as_ref(), Some(&attachment));
        let raw = fs::read_to_string(tmp.path().join("mods").join(&meta.mod_id).join("meta.json"))
            .expect("read");
        assert!(raw.contains("\"sourceAttachment\""));
        assert!(raw.contains("\"attachedBy\": \"user\""));
    }

    #[test]
    fn updates_custom_display_name_in_metadata() {
        let tmp = TempDir::new().expect("tmp");
        let src = tmp.path().join("import");
        fs::create_dir_all(&src).expect("src");
        fs::write(src.join("a.package"), b"x").expect("file");

        let meta = create_managed_mod(
            tmp.path(),
            ImportRequest {
                name: "Detected".to_string(),
                slug: None,
                source_dir: src,
            },
        )
        .expect("create");

        let updated =
            super::set_custom_display_name(tmp.path(), &meta.mod_id, "Custom Display".to_string())
                .expect("update");

        assert_eq!(updated.custom_name.as_deref(), Some("Custom Display"));
        assert_eq!(updated.display_name, "Custom Display");
        assert_eq!(updated.effective_display_name(), "Custom Display");

        let read = read_managed_mod(tmp.path(), &meta.mod_id).expect("read");
        assert_eq!(read.custom_name.as_deref(), Some("Custom Display"));
    }

    #[test]
    fn custom_name_takes_priority_over_detected_name() {
        let tmp = TempDir::new().expect("tmp");
        let base = tmp.path().join("mods/abc");
        fs::create_dir_all(&base).expect("base");
        fs::write(
            base.join("meta.json"),
            r#"{"version":1,"createdBy":"sims4-mod-manager","modId":"abc","name":"Fallback","displayName":"Fallback","detectedName":"Detected","customName":"Custom","slug":null,"files":[],"source":"local"}"#,
        )
        .expect("meta");

        let meta = read_managed_mod(tmp.path(), "abc").expect("read");
        assert_eq!(meta.effective_display_name(), "Custom");
    }

    #[test]
    fn missing_or_invalid_metadata_can_fall_back_without_crashing() {
        let tmp = TempDir::new().expect("tmp");

        let missing = read_managed_mod_or_default(tmp.path(), "missing", "Detected");
        assert_eq!(missing.effective_display_name(), "Detected");

        let base = tmp.path().join("mods/bad");
        fs::create_dir_all(&base).expect("base");
        fs::write(base.join("meta.json"), "{not-json}").expect("bad meta");

        let malformed = read_managed_mod_or_default(tmp.path(), "bad", "Detected Bad");
        assert_eq!(malformed.effective_display_name(), "Detected Bad");
    }

    #[test]
    fn rejects_malformed_or_unsupported_metadata() {
        let tmp = TempDir::new().expect("tmp");
        let base = tmp.path().join("mods/abc");
        fs::create_dir_all(&base).expect("base");
        fs::write(base.join("meta.json"), "{not-json}").expect("meta");

        let malformed = read_managed_mod(tmp.path(), "abc").expect_err("must fail");
        assert_eq!(malformed.code.as_str(), "INVALID_PATH");

        fs::write(
            base.join("meta.json"),
            r#"{"version":2,"createdBy":"sims4-mod-manager","modId":"abc","name":"X","slug":null,"files":[],"source":"managed"}"#,
        )
        .expect("meta2");

        let unsupported = read_managed_mod(tmp.path(), "abc").expect_err("must fail");
        assert_eq!(unsupported.code.as_str(), "INVALID_PATH");
    }
}
