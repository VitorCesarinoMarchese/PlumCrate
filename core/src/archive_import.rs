use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use uuid::Uuid;
use zip::read::ZipArchive;

use crate::error::{ErrorCode, ManagerError};
use crate::fs_scope;
use crate::managed_storage::{create_managed_mod, ImportRequest, ModMetadata};

const MAX_ZIP_ENTRIES: usize = 100_000;
const MAX_ZIP_ENTRY_BYTES: u64 = 2 * 1024 * 1024 * 1024;
const MAX_ZIP_TOTAL_BYTES: u64 = 8 * 1024 * 1024 * 1024;

pub fn import_archive_to_managed(
    managed_root: &Path,
    archive_path: &Path,
    name: String,
    slug: Option<String>,
) -> Result<ModMetadata, ManagerError> {
    let temp_extract = managed_root
        .join("tmp")
        .join(format!("extract-{}", Uuid::new_v4()));
    fs_scope::create_dir_all(&temp_extract)?;

    let extract_result = extract_archive(archive_path, &temp_extract);
    if let Err(err) = extract_result {
        let _ = fs::remove_dir_all(&temp_extract);
        return Err(err);
    }

    let import = create_managed_mod(
        managed_root,
        ImportRequest {
            name,
            slug,
            source_dir: temp_extract.clone(),
        },
    );

    let _ = fs::remove_dir_all(&temp_extract);
    import
}

pub fn extract_archive(archive_path: &Path, destination: &Path) -> Result<(), ManagerError> {
    extract_archive_checked(archive_path, destination, || Ok(()))
}

pub(crate) fn extract_archive_checked(
    archive_path: &Path,
    destination: &Path,
    mut check: impl FnMut() -> Result<(), ManagerError>,
) -> Result<(), ManagerError> {
    check()?;
    let ext = archive_path
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    match ext.as_str() {
        "zip" => extract_zip(archive_path, destination, &mut check),
        _ => Err(ManagerError::new(
            ErrorCode::ArchiveUnsupported,
            format!("Unsupported archive extension: .{ext}"),
        )),
    }
}

fn extract_zip(
    archive_path: &Path,
    destination: &Path,
    check: &mut impl FnMut() -> Result<(), ManagerError>,
) -> Result<(), ManagerError> {
    let file = fs::File::open(archive_path).map_err(|e| {
        ManagerError::new(
            ErrorCode::ArchiveExtractionFailed,
            format!("Open zip failed {}: {e}", archive_path.display()),
        )
    })?;

    let mut archive = ZipArchive::new(file).map_err(|e| {
        ManagerError::new(
            ErrorCode::ArchiveExtractionFailed,
            format!("Parse zip failed {}: {e}", archive_path.display()),
        )
    })?;

    if archive.len() > MAX_ZIP_ENTRIES {
        return Err(extraction_error("ZIP exceeds the 100,000-entry limit"));
    }
    let mut paths = HashMap::new();
    let mut total_bytes = 0_u64;
    // Inspect the complete archive before writing any of its contents.
    for i in 0..archive.len() {
        check()?;
        let entry = archive.by_index(i).map_err(|e| {
            ManagerError::new(
                ErrorCode::ArchiveExtractionFailed,
                format!("Read zip entry failed: {e}"),
            )
        })?;

        let name = std::str::from_utf8(entry.name_raw())
            .map_err(|_| extraction_error("ZIP entry name is not UTF-8"))?
            .replace('\\', "/");
        let out_path = sanitize_zip_path(destination, &name)?;
        let kind = entry.unix_mode().unwrap_or(0) & 0o170000;
        if !matches!(kind, 0 | 0o100000 | 0o040000) {
            return Err(extraction_error(format!(
                "ZIP contains a link or special file: {name}"
            )));
        }
        if paths.insert(out_path.clone(), entry.is_dir()).is_some() {
            return Err(extraction_error(format!("Duplicate ZIP path: {name}")));
        }
        if entry.size() > MAX_ZIP_ENTRY_BYTES {
            return Err(extraction_error(format!("ZIP entry exceeds 2 GiB: {name}")));
        }
        total_bytes = total_bytes
            .checked_add(entry.size())
            .filter(|total| *total <= MAX_ZIP_TOTAL_BYTES)
            .ok_or_else(|| extraction_error("ZIP exceeds the 8 GiB expanded-size limit"))?;
        check_extraction_path(destination, &out_path)?;
        if let Ok(existing) = fs::symlink_metadata(&out_path) {
            if !(entry.is_dir() && existing.is_dir() && !existing.file_type().is_symlink()) {
                return Err(extraction_error(format!(
                    "Extraction target already exists: {name}"
                )));
            }
        }
    }
    for path in paths.keys() {
        let mut parent = path.parent();
        while let Some(ancestor) = parent.filter(|ancestor| ancestor.starts_with(destination)) {
            if paths.get(ancestor) == Some(&false) {
                return Err(extraction_error("ZIP file and directory paths conflict"));
            }
            parent = ancestor.parent();
        }
    }

    fs_scope::create_dir_all(destination)?;
    let directory = fs_scope::Directory::open(destination)?;
    for i in 0..archive.len() {
        check()?;
        let mut entry = archive
            .by_index(i)
            .map_err(|error| extraction_error(error.to_string()))?;
        let name = entry.name().replace('\\', "/");
        let path = sanitize_zip_path(destination, &name)?;
        let relative = path
            .strip_prefix(destination)
            .map_err(|_| extraction_error("Extraction path escapes its root"))?;
        check_extraction_path(destination, &path)?;
        if entry.is_dir() {
            directory.parent(&relative.join(".extract-directory"), true)?;
            continue;
        }
        let mut output = directory.create_file(relative)?;
        let expected = entry.size();
        let mut copied = 0;
        let mut limited = (&mut entry).take(expected + 1);
        let mut buffer = [0; 65536];
        loop {
            check()?;
            let count = limited
                .read(&mut buffer)
                .map_err(|error| extraction_error(format!("Read ZIP entry failed: {error}")))?;
            if count == 0 {
                break;
            }
            std::io::Write::write_all(&mut output, &buffer[..count])
                .map_err(|error| extraction_error(format!("Write ZIP entry failed: {error}")))?;
            copied += count as u64;
        }
        if copied != expected {
            return Err(extraction_error(format!("ZIP entry size mismatch: {name}")));
        }
        output
            .sync_all()
            .map_err(|error| extraction_error(error.to_string()))?;
    }
    Ok(())
}

fn extraction_error(message: impl Into<String>) -> ManagerError {
    ManagerError::new(ErrorCode::ArchiveExtractionFailed, message)
}

fn check_extraction_path(root: &Path, path: &Path) -> Result<(), ManagerError> {
    let relative = path
        .strip_prefix(root)
        .map_err(|_| extraction_error("Extraction path escapes its root"))?;
    let mut current = root.to_path_buf();
    for component in std::iter::once(None).chain(relative.components().map(Some)) {
        if let Some(component) = component {
            current.push(component);
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(extraction_error("Extraction path contains a symlink"))
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(extraction_error(error.to_string())),
        }
    }
    Ok(())
}

fn sanitize_zip_path(destination: &Path, entry_name: &str) -> Result<PathBuf, ManagerError> {
    let candidate = Path::new(entry_name);

    if candidate.is_absolute()
        || entry_name.as_bytes().get(1) == Some(&b':')
        || entry_name.contains('\0')
    {
        return Err(ManagerError::new(
            ErrorCode::ArchiveExtractionFailed,
            format!("Absolute path entry blocked: {entry_name}"),
        ));
    }

    if candidate
        .components()
        .any(|c| !matches!(c, std::path::Component::Normal(_)))
        || candidate.as_os_str().is_empty()
    {
        return Err(ManagerError::new(
            ErrorCode::ArchiveExtractionFailed,
            format!("Path traversal entry blocked: {entry_name}"),
        ));
    }

    Ok(destination.join(candidate))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::Write;

    use tempfile::TempDir;
    use zip::write::SimpleFileOptions;
    use zip::CompressionMethod;

    use super::{extract_archive, import_archive_to_managed};

    fn make_zip(path: &std::path::Path, entries: Vec<(&str, &[u8])>) {
        let file = fs::File::create(path).expect("zip create");
        let mut zip = zip::ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        for (name, content) in entries {
            zip.start_file(name, options).expect("start file");
            zip.write_all(content).expect("write file");
        }

        zip.finish().expect("finish zip");
    }

    #[test]
    fn cancellation_stops_archive_extraction_before_writing_entries() {
        let tmp = TempDir::new().unwrap();
        let archive = tmp.path().join("mod.zip");
        let destination = tmp.path().join("content");
        make_zip(&archive, vec![("a.package", b"AAA"), ("b.package", b"BBB")]);
        let mut checks = 0;
        let result = super::extract_archive_checked(&archive, &destination, || {
            checks += 1;
            if checks == 2 {
                Err(crate::error::ManagerError::new(
                    crate::error::ErrorCode::InvalidPath,
                    "cancelled",
                ))
            } else {
                Ok(())
            }
        });
        assert!(result.is_err());
        assert!(!destination.join("a.package").exists());
        assert!(!destination.join("b.package").exists());
    }

    #[test]
    fn imports_zip_happy_path_into_managed_storage() {
        let tmp = TempDir::new().expect("tmp");
        let archive = tmp.path().join("mod.zip");
        make_zip(
            &archive,
            vec![("a.package", b"AAA"), ("docs/readme.txt", b"TXT")],
        );

        let meta = import_archive_to_managed(
            tmp.path(),
            &archive,
            "ZipMod".to_string(),
            Some("zipmod".to_string()),
        )
        .expect("import");

        assert_eq!(meta.version, 1);
        assert!(meta.files.contains(&"a.package".to_string()));
        assert!(tmp
            .path()
            .join("mods")
            .join(&meta.mod_id)
            .join("files/a.package")
            .is_file());
    }

    #[test]
    fn unsupported_extension_returns_archive_unsupported() {
        let tmp = TempDir::new().expect("tmp");
        let archive = tmp.path().join("mod.tar");
        fs::write(&archive, b"fake").expect("write");

        let err = extract_archive(&archive, tmp.path()).expect_err("must fail");
        assert_eq!(err.code.as_str(), "ARCHIVE_UNSUPPORTED");
    }

    #[test]
    fn corrupt_zip_returns_extraction_failed() {
        let tmp = TempDir::new().expect("tmp");
        let archive = tmp.path().join("mod.zip");
        fs::write(&archive, b"not-zip").expect("write");

        let err = extract_archive(&archive, tmp.path()).expect_err("must fail");
        assert_eq!(err.code.as_str(), "ARCHIVE_EXTRACTION_FAILED");
    }

    #[test]
    fn blocks_path_traversal_entries() {
        let tmp = TempDir::new().expect("tmp");
        let archive = tmp.path().join("mod.zip");
        make_zip(&archive, vec![("../escape.package", b"x")]);

        let extract_to = tmp.path().join("extract");
        fs::create_dir_all(&extract_to).expect("extract dir");

        let err = extract_archive(&archive, &extract_to).expect_err("must fail");
        assert_eq!(err.code.as_str(), "ARCHIVE_EXTRACTION_FAILED");
    }

    #[test]
    fn extracted_files_stay_under_destination_root() {
        let tmp = TempDir::new().expect("tmp");
        let archive = tmp.path().join("mod.zip");
        make_zip(&archive, vec![("nested/a.package", b"x")]);

        let extract_to = tmp.path().join("extract");
        fs::create_dir_all(&extract_to).expect("extract dir");
        extract_archive(&archive, &extract_to).expect("extract");

        assert!(extract_to.join("nested/a.package").is_file());
        assert!(!tmp.path().join("a.package").exists());
    }

    #[test]
    fn seven_zip_requires_a_bounded_extractor() {
        let tmp = TempDir::new().expect("tmp");
        let archive = tmp.path().join("mod.7z");
        fs::write(&archive, b"fake-7z").expect("write");

        let err = extract_archive(&archive, tmp.path()).expect_err("unsupported extractor");
        assert_eq!(err.code.as_str(), "ARCHIVE_UNSUPPORTED");
    }

    #[test]
    fn rar_requires_a_bounded_extractor() {
        let tmp = TempDir::new().expect("tmp");
        let archive = tmp.path().join("mod.rar");
        fs::write(&archive, b"fake-rar").expect("write");

        let err = extract_archive(&archive, tmp.path()).expect_err("unsupported extractor");
        assert_eq!(err.code.as_str(), "ARCHIVE_UNSUPPORTED");
    }
}
