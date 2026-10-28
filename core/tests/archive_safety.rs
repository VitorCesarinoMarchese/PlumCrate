use std::{fs, io::Write};
use plumcrate_core::archive_import::extract_archive;
use plumcrate_core::error::ErrorCode;
use zip::{write::SimpleFileOptions, ZipWriter};

#[test]
fn normalized_duplicate_zip_paths_fail_before_extraction() {
    let temp = tempfile::tempdir().unwrap();
    let archive = temp.path().join("duplicate.zip");
    let mut writer = ZipWriter::new(fs::File::create(&archive).unwrap());
    for name in ["Group/a.package", "Group\\a.package"] {
        writer
            .start_file(name, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"package").unwrap();
    }
    writer.finish().unwrap();
    let destination = temp.path().join("extract");
    assert!(extract_archive(&archive, &destination).is_err());
    assert!(!destination.join("Group/a.package").exists());
}

#[test]
fn zip_extraction_never_overwrites_existing_files() {
    let temp = tempfile::tempdir().unwrap();
    let archive = temp.path().join("archive.zip");
    let mut writer = ZipWriter::new(fs::File::create(&archive).unwrap());
    writer
        .start_file("test.package", SimpleFileOptions::default())
        .unwrap();
    writer.write_all(b"archive package").unwrap();
    writer.finish().unwrap();
    let destination = temp.path().join("extract");
    fs::create_dir_all(&destination).unwrap();
    fs::write(destination.join("test.package"), b"user original").unwrap();
    assert!(extract_archive(&archive, &destination).is_err());
    assert_eq!(
        fs::read(destination.join("test.package")).unwrap(),
        b"user original"
    );
}

#[test]
fn zip_symlink_entries_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let archive = temp.path().join("link.zip");
    let mut writer = ZipWriter::new(fs::File::create(&archive).unwrap());
    writer
        .add_symlink(
            "link.package",
            "../user.package",
            SimpleFileOptions::default(),
        )
        .unwrap();
    writer.finish().unwrap();
    assert!(extract_archive(&archive, &temp.path().join("extract")).is_err());
}

#[test]
fn zip_rejects_an_entry_declaring_more_than_two_gibibytes() {
    let temp = tempfile::tempdir().unwrap();
    let archive = temp.path().join("oversized.zip");
    let mut writer = ZipWriter::new(fs::File::create(&archive).unwrap());
    writer
        .start_file("large.package", SimpleFileOptions::default())
        .unwrap();
    writer.write_all(b"package").unwrap();
    writer.finish().unwrap();
    let mut bytes = fs::read(&archive).unwrap();
    let central = bytes
        .windows(4)
        .position(|window| window == b"PK\x01\x02")
        .unwrap();
    bytes[central + 24..central + 28].copy_from_slice(&2_147_483_649_u32.to_le_bytes());
    fs::write(&archive, bytes).unwrap();
    let destination = temp.path().join("extract");
    assert!(extract_archive(&archive, &destination).is_err());
    assert!(!destination.join("large.package").exists());
}

#[test]
fn unvalidated_external_extractors_are_not_used() {
    let temp = tempfile::tempdir().unwrap();
    for extension in ["rar", "7z"] {
        let archive = temp.path().join(format!("untrusted.{extension}"));
        fs::write(&archive, b"untrusted input").unwrap();
        assert_eq!(
            extract_archive(&archive, temp.path()).unwrap_err().code,
            ErrorCode::ArchiveUnsupported
        );
    }
}

#[cfg(unix)]
#[test]
fn archive_import_rejects_a_redirected_staging_directory() {
    use std::os::unix::fs::symlink;
    use plumcrate_core::archive_import::import_archive_to_managed;
    let temp = tempfile::tempdir().unwrap();
    let managed = temp.path().join("managed");
    let outside = temp.path().join("outside");
    fs::create_dir_all(&managed).unwrap();
    fs::create_dir_all(&outside).unwrap();
    symlink(&outside, managed.join("tmp")).unwrap();
    let archive = temp.path().join("safe.zip");
    let mut writer = ZipWriter::new(fs::File::create(&archive).unwrap());
    writer
        .start_file("test.package", SimpleFileOptions::default())
        .unwrap();
    writer.write_all(b"package").unwrap();
    writer.finish().unwrap();
    assert!(import_archive_to_managed(&managed, &archive, "Test".into(), None).is_err());
    assert_eq!(fs::read_dir(outside).unwrap().count(), 0);
    assert!(!managed.join("mods").exists());
}
