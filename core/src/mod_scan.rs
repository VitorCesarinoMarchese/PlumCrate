use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{ErrorCode, ManagerError};
use crate::managed_storage::{read_managed_mod, ModMetadata, SourceAttachmentMetadata};
use crate::metadata_names::detect_display_name;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ModSource {
    Managed,
    External,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScannedMod {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub name: String,
    pub files: Vec<String>,
    pub mod_files: Vec<String>,
    pub preview: Option<String>,
    #[serde(rename = "sourceUrl", skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(rename = "sourceAttachment", skip_serializing_if = "Option::is_none")]
    pub source_attachment: Option<SourceAttachmentMetadata>,
    pub source: ModSource,
    pub enabled: bool,
    pub group_path: Vec<String>,
}

#[derive(Debug, Clone)]
struct FileEntry {
    relative: String,
    absolute: PathBuf,
    is_symlink: bool,
    symlink_target: Option<PathBuf>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum GroupOwner {
    Managed(String),
    External(String),
}

pub fn scan_mods(mods_dir: &Path, managed_root: &Path) -> Result<Vec<ScannedMod>, ManagerError> {
    let mut groups: BTreeMap<GroupOwner, (String, Vec<FileEntry>)> = BTreeMap::new();
    let mut stack = vec![mods_dir.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).map_err(|e| scan_error(&dir, e))?;
        for entry in entries {
            let entry = entry.map_err(|e| scan_error(&dir, e))?;
            let path = entry.path();
            let meta = fs::symlink_metadata(&path).map_err(|e| scan_error(&path, e))?;

            if meta.is_dir() {
                stack.push(path);
                continue;
            }

            if !meta.is_file() && !meta.file_type().is_symlink() {
                continue;
            }

            let Ok(rel) = path.strip_prefix(mods_dir) else {
                continue;
            };

            let relative = rel.to_string_lossy().replace('\\', "/");
            let key = group_key(rel);
            let symlink_target = if meta.file_type().is_symlink() {
                Some(fs::read_link(&path).map_err(|e| scan_error(&path, e))?).map(|target| {
                    let absolute = if target.is_absolute() {
                        target
                    } else {
                        dir.join(target)
                    };
                    absolute.canonicalize().unwrap_or(absolute)
                })
            } else {
                None
            };

            let owner = symlink_target
                .as_ref()
                .and_then(|target| managed_mod_id_from_target(target, managed_root))
                .map(GroupOwner::Managed)
                .unwrap_or_else(|| GroupOwner::External(key.clone()));
            groups
                .entry(owner)
                .or_insert_with(|| (key, vec![]))
                .1
                .push(FileEntry {
                    relative,
                    absolute: path,
                    is_symlink: meta.file_type().is_symlink(),
                    symlink_target,
                });
        }
    }

    let mut scanned = groups
        .into_iter()
        .filter_map(|(_, (key, mut files))| {
            files.sort_by(|a, b| a.relative.cmp(&b.relative));
            let scanned = build_scanned_mod(key, files, managed_root);
            (!scanned.mod_files.is_empty()).then_some(scanned)
        })
        .collect::<Vec<_>>();

    append_disabled_managed_mods(&mut scanned, managed_root)?;
    Ok(scanned)
}

fn scan_error(path: &Path, error: std::io::Error) -> ManagerError {
    ManagerError::new(ErrorCode::IoError, format!("Scan incomplete. Cannot read {}: {error}", path.display()))
}

fn append_disabled_managed_mods(scanned: &mut Vec<ScannedMod>, managed_root: &Path) -> Result<(), ManagerError> {
    let managed_mods = managed_root.join("mods");
    let entries = match fs::read_dir(&managed_mods) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(scan_error(&managed_mods, e)),
    };
    for entry in entries {
        let entry = entry.map_err(|e| scan_error(&managed_mods, e))?;
        let mod_id = entry.file_name().to_string_lossy().to_string();
        // Recovery holdings are retained originals, not catalog entries.
        if mod_id.starts_with(".ts4-holding-") {
            continue;
        }
        if scanned
            .iter()
            .any(|mod_entry| mod_entry.id.as_deref() == Some(&mod_id))
        {
            continue;
        }
        let meta = read_managed_mod(managed_root, &mod_id).map_err(|e| {
            ManagerError::new(e.code, format!("Scan incomplete. Cannot read managed mod {mod_id}: {}", e.message))
        })?;
        let scanned_mod = scanned_mod_from_metadata(meta);
        if !scanned_mod.mod_files.is_empty() {
            scanned.push(scanned_mod);
        }
    }
    Ok(())
}

fn scanned_mod_from_metadata(meta: ModMetadata) -> ScannedMod {
    let name = meta.effective_display_name().to_string();
    let mod_files = meta
        .files
        .iter()
        .filter(|path| is_mod_file(path))
        .cloned()
        .collect::<Vec<_>>();
    ScannedMod {
        key: meta.mod_id.clone(),
        id: Some(meta.mod_id),
        name: name.clone(),
        files: meta.files,
        mod_files,
        preview: meta.local_preview_path.or(meta.preview_url),
        source_url: meta.source_url,
        source_attachment: meta.source_attachment,
        source: ModSource::Managed,
        enabled: false,
        group_path: vec![name],
    }
}

fn build_scanned_mod(key: String, files: Vec<FileEntry>, managed_root: &Path) -> ScannedMod {
    let enabled = files
        .iter()
        .any(|file| is_mod_file(&file.relative) && file.absolute.is_file());
    let all_files = files.iter().map(|f| f.relative.clone()).collect::<Vec<_>>();

    let mod_files = files
        .iter()
        .filter(|f| is_mod_file(&f.relative))
        .map(|f| f.relative.clone())
        .collect::<Vec<_>>();

    let source = if files.iter().any(|f| {
        f.is_symlink
            && f.symlink_target
                .as_ref()
                .is_some_and(|target| target.starts_with(managed_root.join("mods")))
    }) {
        ModSource::Managed
    } else {
        ModSource::External
    };

    let group_path = key.split('/').map(ToString::to_string).collect::<Vec<_>>();
    let raw_name = group_path.last().cloned().unwrap_or_else(|| key.clone());
    let metadata = managed_metadata_from_files(&files, managed_root);
    let id = metadata.as_ref().map(|meta| meta.mod_id.clone());
    let source_url = metadata.as_ref().and_then(|meta| meta.source_url.clone());
    let source_attachment = metadata
        .as_ref()
        .and_then(|meta| meta.source_attachment.clone());
    let preview = metadata
        .as_ref()
        .and_then(|meta| {
            meta.local_preview_path
                .clone()
                .or_else(|| meta.preview_url.clone())
        })
        .or_else(|| choose_preview(&files));
    let name = metadata
        .as_ref()
        .map(|meta| meta.effective_display_name().to_string())
        .unwrap_or_else(|| detect_display_name(&raw_name));

    ScannedMod {
        key,
        id,
        name,
        files: all_files,
        mod_files,
        preview,
        source_url,
        source_attachment,
        source,
        enabled,
        group_path,
    }
}

fn managed_metadata_from_files(files: &[FileEntry], managed_root: &Path) -> Option<ModMetadata> {
    files
        .iter()
        .filter_map(|f| f.symlink_target.as_ref())
        .filter_map(|target| managed_mod_id_from_target(target, managed_root))
        .find_map(|mod_id| read_managed_mod(managed_root, &mod_id).ok())
}

fn managed_mod_id_from_target(target: &Path, managed_root: &Path) -> Option<String> {
    let rel = target.strip_prefix(managed_root.join("mods")).ok()?;
    let mut comps = rel.components();
    let mod_id = comps.next()?.as_os_str().to_string_lossy().to_string();
    (comps.next()?.as_os_str() == "files").then_some(mod_id)
}

fn group_key(relative: &Path) -> String {
    let mut comps = relative.components();
    let Some(first) = comps.next() else {
        return "unknown".to_string();
    };

    if comps.next().is_some() {
        return first.as_os_str().to_string_lossy().to_string();
    }

    let filename = first.as_os_str().to_string_lossy().to_string();
    filename_prefix(&filename)
}

fn filename_prefix(filename: &str) -> String {
    let stem = filename.split('.').next().unwrap_or(filename);
    if let Some(rest) = stem.trim_start().strip_prefix('[') {
        if let Some((prefix, _)) = rest.split_once(']') {
            let prefix = prefix.trim();
            if !prefix.is_empty() {
                return prefix.to_string();
            }
        }
    }

    let mut out = String::new();

    for ch in stem.chars() {
        if matches!(ch, '_' | '-' | ' ') {
            break;
        }
        out.push(ch);
    }

    if out.is_empty() {
        stem.to_string()
    } else {
        out
    }
}

fn is_mod_file(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".package") || lower.ends_with(".ts4script")
}

fn is_preview_candidate(path: &str) -> bool {
    let lower = path.to_ascii_lowercase();
    lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.contains("preview.")
        || lower.contains("cover.")
}

fn choose_preview(files: &[FileEntry]) -> Option<String> {
    files
        .iter()
        .filter(|f| is_preview_candidate(&f.relative))
        .max_by_key(|f| {
            let dims = image::image_dimensions(&f.absolute).unwrap_or((0, 0));
            let area = (dims.0 as u64) * (dims.1 as u64);
            let size = fs::metadata(&f.absolute).map(|m| m.len()).unwrap_or(0);
            (area, size)
        })
        .map(|f| f.absolute.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use image::{ImageBuffer, Rgba};
    use tempfile::TempDir;

    use crate::managed_storage::{
        write_managed_mod, ModMetadata, SourceAttachmentMetadata, SourceEvidence,
    };

    use super::{scan_mods, ModSource};

    #[test]
    fn missing_mods_folder_is_not_a_successful_empty_scan() {
        let root = TempDir::new().unwrap();
        assert!(scan_mods(&root.path().join("missing"), &root.path().join("managed")).is_err());
    }

    #[test]
    fn corrupt_managed_metadata_is_not_silently_skipped() {
        let root = TempDir::new().unwrap();
        let mods = root.path().join("Mods");
        let managed = root.path().join("managed");
        fs::create_dir_all(&mods).unwrap();
        fs::create_dir_all(managed.join("mods/broken")).unwrap();
        fs::write(managed.join("mods/broken/meta.json"), "{broken").unwrap();
        assert!(scan_mods(&mods, &managed).is_err());
    }

    #[test]
    #[cfg(unix)]
    fn keeps_managed_mods_and_external_files_separate_in_shared_folder() {
        use crate::managed_storage::{create_managed_mod, ImportRequest};
        use crate::toggle::apply_toggle;

        let root = TempDir::new().expect("tmp");
        let mods = root.path().join("Mods");
        let managed = root.path().join("managed");
        fs::create_dir_all(&mods).expect("mods");
        let mut ids = vec![];
        for name in ["One", "Two"] {
            let source = root.path().join(name);
            fs::create_dir_all(source.join("Shared")).expect("source");
            fs::write(source.join(format!("Shared/{name}.package")), name).expect("file");
            let meta = create_managed_mod(
                &managed,
                ImportRequest {
                    name: name.to_string(),
                    slug: None,
                    source_dir: source,
                },
            )
            .expect("import");
            apply_toggle(&managed, &mods, &meta.mod_id, true).expect("enable");
            ids.push(meta.mod_id);
        }
        fs::write(mods.join("Shared/External.package"), b"external").expect("external");

        let scanned = scan_mods(&mods, &managed).unwrap();
        assert_eq!(
            scanned.len(),
            3,
            "each owner must have its own catalog entry"
        );
        for id in ids {
            let entry = scanned
                .iter()
                .find(|entry| entry.id.as_deref() == Some(&id))
                .expect("owner");
            assert_eq!(
                entry.mod_files.len(),
                1,
                "another mod's files must not join this owner"
            );
            assert_eq!(serde_json::to_value(entry).expect("json")["enabled"], true);
        }
        let external = scanned
            .iter()
            .find(|entry| entry.source == ModSource::External)
            .expect("external");
        assert_eq!(external.mod_files, vec!["Shared/External.package"]);
    }

    #[test]
    #[cfg(unix)]
    fn reports_enabled_state_and_resolves_relative_managed_links() {
        use crate::managed_storage::{create_managed_mod, ImportRequest};

        let root = TempDir::new().expect("tmp");
        let mods = root.path().join("Mods");
        let managed = root.path().join("managed");
        let source = root.path().join("source");
        fs::create_dir_all(&mods).expect("mods");
        fs::create_dir_all(&source).expect("source");
        fs::write(source.join("Example.package"), b"package").expect("file");
        let meta = create_managed_mod(
            &managed,
            ImportRequest {
                name: "Example".to_string(),
                slug: None,
                source_dir: source,
            },
        )
        .expect("import");
        let stored = scan_mods(&mods, &managed).unwrap();
        assert_eq!(
            serde_json::to_value(&stored[0]).expect("json")["enabled"],
            false
        );

        std::os::unix::fs::symlink(
            format!("../managed/mods/{}/files/Example.package", meta.mod_id),
            mods.join("Example.package"),
        )
        .expect("link");
        let installed = scan_mods(&mods, &managed).unwrap();
        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].id.as_deref(), Some(meta.mod_id.as_str()));
        assert_eq!(
            serde_json::to_value(&installed[0]).expect("json")["enabled"],
            true
        );
    }

    #[test]
    fn scans_recursively_and_groups_by_folder() {
        let root = TempDir::new().expect("tmp");
        let mods = root.path().join("Mods");
        let managed = root.path().join("managed");

        fs::create_dir_all(mods.join("MyMod/sub")).expect("dirs");
        fs::create_dir_all(&managed).expect("managed");
        fs::write(mods.join("MyMod/main.package"), b"pkg").expect("file");
        fs::write(mods.join("MyMod/sub/readme.txt"), b"txt").expect("file");

        let scanned = scan_mods(&mods, &managed).unwrap();
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].key, "MyMod");
        assert_eq!(scanned[0].files.len(), 2);
    }

    #[test]
    fn groups_bracketed_loose_cc_by_full_creator_prefix() {
        let root = TempDir::new().expect("tmp");
        let mods = root.path().join("Mods");
        let managed = root.path().join("managed");

        fs::create_dir_all(&mods).expect("mods");
        fs::create_dir_all(&managed).expect("managed");
        fs::write(
            mods.join("[Gabymelove Sims] Converse Platform High Tops (M) • CF Edition — Base Colors Update C80.package"),
            b"pkg",
        )
        .expect("pkg");

        let scanned = scan_mods(&mods, &managed).unwrap();
        assert_eq!(scanned[0].key, "Gabymelove Sims");
        assert_eq!(scanned[0].name, "Gabymelove Sims");
    }

    #[test]
    fn groups_root_files_by_filename_prefix_fallback() {
        let root = TempDir::new().expect("tmp");
        let mods = root.path().join("Mods");
        let managed = root.path().join("managed");

        fs::create_dir_all(&mods).expect("mods");
        fs::create_dir_all(&managed).expect("managed");
        fs::write(mods.join("CoolMod_A.package"), b"a").expect("a");
        fs::write(mods.join("CoolMod_B.ts4script"), b"b").expect("b");

        let scanned = scan_mods(&mods, &managed).unwrap();
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].key, "CoolMod");
    }

    #[test]
    fn includes_assets_only_when_group_contains_mod_files() {
        let root = TempDir::new().expect("tmp");
        let mods = root.path().join("Mods");
        let managed = root.path().join("managed");

        fs::create_dir_all(mods.join("Pack")).expect("dirs");
        fs::create_dir_all(&managed).expect("managed");
        fs::write(mods.join("Pack/a.package"), b"pkg").expect("pkg");
        fs::write(mods.join("Pack/b.ts4script"), b"ts4").expect("ts4");
        fs::write(mods.join("Pack/cover.jpg"), b"jpg").expect("jpg");
        fs::write(mods.join("Pack/readme.md"), b"md").expect("md");

        let scanned = scan_mods(&mods, &managed).unwrap();
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].files.len(), 4);
        assert_eq!(scanned[0].mod_files.len(), 2);
    }

    #[test]
    fn excludes_files_and_folders_without_mod_files() {
        let root = TempDir::new().expect("tmp");
        let mods = root.path().join("Mods");
        let managed = root.path().join("managed");

        fs::create_dir_all(mods.join("Resource.dasdas")).expect("dirs");
        fs::create_dir_all(mods.join("RealMod")).expect("real");
        fs::create_dir_all(&managed).expect("managed");
        fs::write(mods.join("Resource.dasdas/readme.txt"), b"txt").expect("txt");
        fs::write(mods.join("orphan.txt"), b"txt").expect("orphan");
        fs::write(mods.join("RealMod/main.package"), b"pkg").expect("pkg");

        let scanned = scan_mods(&mods, &managed).unwrap();
        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].key, "RealMod");
    }

    #[test]
    fn chooses_preview_by_biggest_resolution_then_size() {
        let root = TempDir::new().expect("tmp");
        let mods = root.path().join("Mods");
        let managed = root.path().join("managed");

        fs::create_dir_all(mods.join("Pack")).expect("dirs");
        fs::create_dir_all(&managed).expect("managed");

        let small = ImageBuffer::<Rgba<u8>, _>::from_pixel(64, 64, Rgba([1, 2, 3, 255]));
        let big = ImageBuffer::<Rgba<u8>, _>::from_pixel(256, 256, Rgba([1, 2, 3, 255]));

        fs::write(mods.join("Pack/main.package"), b"pkg").expect("pkg");
        small
            .save(mods.join("Pack/preview_small.png"))
            .expect("save small");
        big.save(mods.join("Pack/preview_big.png"))
            .expect("save big");

        let scanned = scan_mods(&mods, &managed).unwrap();
        assert_eq!(
            scanned[0].preview.as_deref(),
            Some(mods.join("Pack/preview_big.png").to_string_lossy().as_ref())
        );
    }

    #[test]
    fn includes_disabled_managed_mods_from_metadata() {
        let root = TempDir::new().expect("tmp");
        let mods = root.path().join("Mods");
        let managed = root.path().join("managed");
        fs::create_dir_all(&mods).expect("mods");
        fs::create_dir_all(managed.join("mods/id/files")).expect("managed");
        fs::write(managed.join("mods/id/files/Imported.package"), b"pkg").expect("pkg");
        write_managed_mod(
            &managed,
            &ModMetadata {
                version: 1,
                created_by: "sims4-mod-manager".to_string(),
                mod_id: "id".to_string(),
                name: "Imported".to_string(),
                display_name: "Imported".to_string(),
                detected_name: Some("Imported".to_string()),
                custom_name: None,
                slug: None,
                files: vec!["Imported.package".to_string()],
                source: "local".to_string(),
                source_url: None,
                preview_url: None,
                local_preview_path: None,
                source_attachment: None,
                locked_name: false,
                updated_at: None,
            },
        )
        .expect("meta");

        let scanned = scan_mods(&mods, &managed).unwrap();

        assert_eq!(scanned.len(), 1);
        assert_eq!(scanned[0].id.as_deref(), Some("id"));
        assert_eq!(scanned[0].name, "Imported");
        assert_eq!(scanned[0].source, ModSource::Managed);
    }

    #[test]
    fn marks_regular_installed_files_as_external() {
        let root = TempDir::new().expect("tmp");
        let mods = root.path().join("Mods");
        let managed = root.path().join("managed");

        fs::create_dir_all(mods.join("InstalledMod")).expect("mods");
        fs::create_dir_all(&managed).expect("managed");
        fs::write(mods.join("InstalledMod/main.package"), b"pkg").expect("pkg");

        let scanned = scan_mods(&mods, &managed).unwrap();
        assert_eq!(scanned[0].source, ModSource::External);
    }

    #[test]
    fn marks_external_when_symlink_target_outside_managed() {
        let root = TempDir::new().expect("tmp");
        let mods = root.path().join("Mods");
        let managed = root.path().join("managed");
        let external_root = root.path().join("outside");

        fs::create_dir_all(&mods).expect("mods");
        fs::create_dir_all(&managed).expect("managed");
        fs::create_dir_all(&external_root).expect("outside");
        fs::write(external_root.join("x.package"), b"pkg").expect("pkg");

        #[cfg(unix)]
        std::os::unix::fs::symlink(external_root.join("x.package"), mods.join("Ext_x.package"))
            .expect("symlink");

        let scanned = scan_mods(&mods, &managed).unwrap();
        assert_eq!(scanned[0].source, ModSource::External);
    }

    #[test]
    fn managed_symlink_uses_metadata_mod_id() {
        let root = TempDir::new().expect("tmp");
        let mods = root.path().join("Mods");
        let managed = root.path().join("managed");
        let managed_file = managed
            .join("mods/uuid-123/files/McCmdCenter_AllModules_2026_2_0/mc_cmd_center.package");

        fs::create_dir_all(&mods).expect("mods");
        fs::create_dir_all(managed_file.parent().expect("parent")).expect("managed tree");
        fs::write(&managed_file, b"pkg").expect("pkg");
        write_managed_mod(
            &managed,
            &ModMetadata {
                version: 1,
                created_by: "sims4-mod-manager".to_string(),
                mod_id: "uuid-123".to_string(),
                name: "McCmdCenter_AllModules_2026_2_0".to_string(),
                display_name: "Mc Command Center".to_string(),
                detected_name: Some("Mc Command Center".to_string()),
                custom_name: None,
                slug: None,
                files: vec!["McCmdCenter_AllModules_2026_2_0/mc_cmd_center.package".to_string()],
                source: "local".to_string(),
                source_url: None,
                preview_url: None,
                local_preview_path: None,
                source_attachment: None,
                locked_name: false,
                updated_at: None,
            },
        )
        .expect("write meta");

        fs::create_dir_all(mods.join("McCmdCenter_AllModules_2026_2_0")).expect("mod dir");
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            &managed_file,
            mods.join("McCmdCenter_AllModules_2026_2_0/mc_cmd_center.package"),
        )
        .expect("symlink");

        let scanned = scan_mods(&mods, &managed).unwrap();
        assert_eq!(scanned[0].key, "McCmdCenter_AllModules_2026_2_0");
        assert_eq!(scanned[0].id.as_deref(), Some("uuid-123"));
    }

    #[test]
    fn managed_symlink_includes_metadata_source_url() {
        let root = TempDir::new().expect("tmp");
        let mods = root.path().join("Mods");
        let managed = root.path().join("managed");
        let managed_file = managed.join("mods/id/files/Example.package");

        fs::create_dir_all(&mods).expect("mods");
        fs::create_dir_all(managed_file.parent().expect("parent")).expect("managed tree");
        fs::write(&managed_file, b"pkg").expect("pkg");
        write_managed_mod(
            &managed,
            &ModMetadata {
                version: 1,
                created_by: "sims4-mod-manager".to_string(),
                mod_id: "id".to_string(),
                name: "Example".to_string(),
                display_name: "Example".to_string(),
                detected_name: Some("Example".to_string()),
                custom_name: None,
                slug: None,
                files: vec!["Example.package".to_string()],
                source: "curseforge".to_string(),
                source_url: Some("https://www.curseforge.com/sims4/mods/example".to_string()),
                preview_url: Some("https://img.example/cover.jpg".to_string()),
                local_preview_path: None,
                source_attachment: Some(SourceAttachmentMetadata {
                    provider_id: "curseforge".to_string(),
                    project_id: Some(551680),
                    file_id: Some(67890),
                    installed_release: None,
                    source_url: "https://www.curseforge.com/sims4/mods/example".to_string(),
                    title: "Example".to_string(),
                    author: Some("Creator".to_string()),
                    confidence: Some(95),
                    reasons: vec!["Exact fingerprint match".to_string()],
                    evidence: vec![SourceEvidence {
                        kind: "fingerprint".to_string(),
                        description: "Exact fingerprint match".to_string(),
                        weight: 95,
                    }],
                    attached_by: "user".to_string(),
                    attached_at: "2026-05-13T12:34:56Z".to_string(),
                }),
                locked_name: false,
                updated_at: None,
            },
        )
        .expect("write meta");

        #[cfg(unix)]
        std::os::unix::fs::symlink(&managed_file, mods.join("Example.package")).expect("symlink");

        let scanned = scan_mods(&mods, &managed).unwrap();
        assert_eq!(
            scanned[0].source_url.as_deref(),
            Some("https://www.curseforge.com/sims4/mods/example")
        );
        assert_eq!(
            scanned[0].preview.as_deref(),
            Some("https://img.example/cover.jpg")
        );
        assert_eq!(
            scanned[0]
                .source_attachment
                .as_ref()
                .and_then(|source| source.project_id),
            Some(551680)
        );
    }

    #[test]
    fn managed_symlink_uses_custom_metadata_name() {
        let root = TempDir::new().expect("tmp");
        let mods = root.path().join("Mods");
        let managed = root.path().join("managed");
        let managed_file = managed.join("mods/id/files/McCmdCenter_AllModules_2026_2_0.package");

        fs::create_dir_all(&mods).expect("mods");
        fs::create_dir_all(managed_file.parent().expect("parent")).expect("managed tree");
        fs::write(&managed_file, b"pkg").expect("pkg");
        write_managed_mod(
            &managed,
            &ModMetadata {
                version: 1,
                created_by: "sims4-mod-manager".to_string(),
                mod_id: "id".to_string(),
                name: "McCmdCenter_AllModules_2026_2_0".to_string(),
                display_name: "My MCCC".to_string(),
                detected_name: Some("Mc Command Center".to_string()),
                custom_name: Some("My MCCC".to_string()),
                slug: None,
                files: vec!["McCmdCenter_AllModules_2026_2_0.package".to_string()],
                source: "local".to_string(),
                source_url: None,
                preview_url: None,
                local_preview_path: None,
                source_attachment: None,
                locked_name: false,
                updated_at: None,
            },
        )
        .expect("write meta");

        #[cfg(unix)]
        std::os::unix::fs::symlink(
            &managed_file,
            mods.join("McCmdCenter_AllModules_2026_2_0.package"),
        )
        .expect("symlink");

        let scanned = scan_mods(&mods, &managed).unwrap();
        assert_eq!(scanned[0].name, "My MCCC");
    }

    #[test]
    fn marks_managed_when_symlink_target_inside_managed() {
        let root = TempDir::new().expect("tmp");
        let mods = root.path().join("Mods");
        let managed = root.path().join("managed");
        let managed_file = managed.join("mods/id/files/y.package");

        fs::create_dir_all(&mods).expect("mods");
        fs::create_dir_all(managed_file.parent().expect("parent")).expect("managed tree");
        fs::write(&managed_file, b"pkg").expect("pkg");

        fs::write(managed.join("mods/id/meta.json"), serde_json::json!({
            "version": 1, "createdBy": "sims4-mod-manager", "modId": "id",
            "name": "Local", "displayName": "Local", "files": ["y.package"],
            "source": "local"
        }).to_string()).unwrap();

        #[cfg(unix)]
        std::os::unix::fs::symlink(&managed_file, mods.join("Managed_y.package")).expect("symlink");

        let scanned = scan_mods(&mods, &managed).unwrap();
        assert_eq!(scanned[0].source, ModSource::Managed);
    }
}
