#![cfg(unix)]

use std::{fs, path::PathBuf};
use tempfile::TempDir;
use plumcrate_core::{
    error::ErrorCode,
    lifecycle::{restore_trashed_mod, uninstall_managed_mod},
    managed_storage::{create_managed_mod, ImportRequest},
};

fn imported(temp: &TempDir) -> (PathBuf, PathBuf, String) {
    let source = temp.path().join("source");
    let managed = temp.path().join("managed");
    let game = temp.path().join("instance/Mods");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&game).unwrap();
    fs::write(source.join("test.package"), b"managed original").unwrap();
    let meta = create_managed_mod(
        &managed,
        ImportRequest {
            name: "Test".into(),
            slug: None,
            source_dir: source,
        },
    )
    .unwrap();
    (managed, game, meta.mod_id)
}

#[test]
fn managed_trash_never_moves_an_unmanaged_group() {
    let temp = tempfile::tempdir().unwrap();
    let game = temp.path().join("Mods");
    fs::create_dir_all(game.join("UserMod")).unwrap();
    fs::write(game.join("UserMod/user.package"), b"unmanaged original").unwrap();
    let result = uninstall_managed_mod(
        &temp.path().join("managed"),
        &game,
        &temp.path().join("Trash/files"),
        "UserMod",
    );
    assert!(result.is_err());
    assert_eq!(
        fs::read(game.join("UserMod/user.package")).unwrap(),
        b"unmanaged original"
    );
}

#[test]
fn mixed_legacy_restore_includes_managed_metadata() {
    let temp = tempfile::tempdir().unwrap();
    let (managed, game, id) = imported(&temp);
    let trash = temp.path().join("Trash/files");
    let entry = trash.join("legacy-mixed-123");
    fs::create_dir_all(&entry).unwrap();
    fs::rename(managed.join("mods").join(&id), entry.join("metadata")).unwrap();
    fs::write(entry.join("legacy.package"), b"legacy original").unwrap();
    restore_trashed_mod(&managed, &game, &trash, "legacy-mixed-123").unwrap();
    assert_eq!(
        fs::read(managed.join("mods").join(&id).join("files/test.package")).unwrap(),
        b"managed original"
    );
    assert!(managed.join("mods").join(&id).join("meta.json").is_file());
    assert_eq!(
        fs::read(game.join("legacy.package")).unwrap(),
        b"legacy original"
    );
}

#[test]
fn restore_preflights_every_collision_before_moving_any_file() {
    let temp = tempfile::tempdir().unwrap();
    let (managed, game, _) = imported(&temp);
    let trash = temp.path().join("Trash/files");
    let entry = trash.join("legacy-123");
    fs::create_dir_all(&entry).unwrap();
    fs::write(entry.join("first.package"), b"first original").unwrap();
    fs::write(entry.join("last.package"), b"last original").unwrap();
    fs::write(game.join("last.package"), b"existing user file").unwrap();
    let result = restore_trashed_mod(&managed, &game, &trash, "legacy-123");
    assert_eq!(result.unwrap_err().code, ErrorCode::PathCollision);
    assert_eq!(
        fs::read(entry.join("first.package")).unwrap(),
        b"first original"
    );
    assert_eq!(
        fs::read(entry.join("last.package")).unwrap(),
        b"last original"
    );
    assert!(!game.join("first.package").exists());
    assert_eq!(
        fs::read(game.join("last.package")).unwrap(),
        b"existing user file"
    );
}

#[test]
fn trash_and_restore_work_across_filesystems() {
    use std::os::unix::fs::MetadataExt;
    let temp = tempfile::tempdir().unwrap();
    let cross = tempfile::Builder::new()
        .prefix("ts4-trash-test-")
        .tempdir_in("/dev/shm")
        .unwrap();
    assert_ne!(
        fs::metadata(temp.path()).unwrap().dev(),
        fs::metadata(cross.path()).unwrap().dev(),
        "fixture requires distinct filesystems"
    );
    let (managed, game, id) = imported(&temp);
    let trash = cross.path().join("Trash/files");
    let result = uninstall_managed_mod(&managed, &game, &trash, &id).unwrap();
    assert!(!managed.join("mods").join(&id).exists());
    assert!(
        plumcrate_core::mod_scan::scan_mods(&game, &managed).unwrap().is_empty(),
        "preserved source holding must not appear as a managed mod"
    );
    let trashed = PathBuf::from(result.trashed_path);
    assert_eq!(
        fs::read(trashed.join("files/test.package")).unwrap(),
        b"managed original"
    );
    restore_trashed_mod(
        &managed,
        &game,
        &trash,
        trashed.file_name().unwrap().to_str().unwrap(),
    )
    .unwrap();
    assert_eq!(
        fs::read(managed.join("mods").join(&id).join("files/test.package")).unwrap(),
        b"managed original"
    );
}

#[test]
fn trash_enumerates_links_for_every_instance_and_preserves_foreign_replacements() {
    let temp = tempfile::tempdir().unwrap();
    let (managed, game, id) = imported(&temp);
    let other = temp.path().join("other/Mods");
    fs::create_dir_all(&other).unwrap();
    plumcrate_core::toggle::apply_toggle(&managed, &game, &id, true).unwrap();
    plumcrate_core::toggle::apply_toggle(&managed, &other, &id, true).unwrap();
    let trash = temp.path().join("Trash/files");
    uninstall_managed_mod(&managed, &game, &trash, &id).unwrap();
    assert!(fs::symlink_metadata(game.join("test.package")).is_err());
    assert!(fs::symlink_metadata(other.join("test.package")).is_err());
}
#[test]
fn restore_does_not_follow_a_replaced_destination_parent() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let (managed, game, id) = imported(&temp);
    let trash = temp.path().join("Trash/files");
    let entry = trash.join("legacy-123");
    fs::create_dir_all(entry.join("metadata")).unwrap();
    fs::rename(
        managed.join("mods").join(&id),
        entry.join("metadata/bundle"),
    )
    .unwrap();
    // A normal managed bundle entry restored into a substituted mods parent must fail.
    let outside = temp.path().join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::remove_dir(managed.join("mods")).unwrap();
    symlink(&outside, managed.join("mods")).unwrap();
    let bundle = entry.join("metadata/bundle");
    fs::rename(&bundle, trash.join("bundle-123")).unwrap();
    assert!(restore_trashed_mod(&managed, &game, &trash, "bundle-123").is_err());
    assert!(fs::read_dir(outside).unwrap().next().is_none());
    assert_eq!(
        fs::read(trash.join("bundle-123/files/test.package")).unwrap(),
        b"managed original"
    );
}
#[test]
fn restore_preserves_single_files_and_rejects_dangling_collision() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let (managed, game, _) = imported(&temp);
    let trash = temp.path().join("Trash/files");
    fs::create_dir_all(&trash).unwrap();
    fs::write(trash.join("legacy.package-123"), b"single original").unwrap();
    symlink("missing", game.join("legacy.package")).unwrap();
    assert_eq!(
        restore_trashed_mod(&managed, &game, &trash, "legacy.package-123")
            .unwrap_err()
            .code,
        ErrorCode::PathCollision
    );
    assert_eq!(
        fs::read(trash.join("legacy.package-123")).unwrap(),
        b"single original"
    );
    fs::remove_file(game.join("legacy.package")).unwrap();
    restore_trashed_mod(&managed, &game, &trash, "legacy.package-123").unwrap();
    assert_eq!(
        fs::read(game.join("legacy.package")).unwrap(),
        b"single original"
    );
}

#[test]
fn restore_rejects_unrelated_trash_and_redirected_info_before_moving_mod_bytes() {
    use std::os::unix::fs::symlink;
    let temp = tempfile::tempdir().unwrap();
    let (managed, game, _) = imported(&temp);
    let trash = temp.path().join("Trash/files");
    fs::create_dir_all(&trash).unwrap();
    fs::write(trash.join("notes.txt"), b"user notes").unwrap();
    assert!(restore_trashed_mod(&managed, &game, &trash, "notes.txt").is_err());
    assert_eq!(fs::read(trash.join("notes.txt")).unwrap(), b"user notes");
    let outside = temp.path().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("legacy-123.trashinfo"), b"user index").unwrap();
    symlink(&outside, temp.path().join("Trash/info")).unwrap();
    fs::create_dir(trash.join("legacy-123")).unwrap();
    fs::write(trash.join("legacy-123/a.package"), b"original mod").unwrap();
    assert!(restore_trashed_mod(&managed, &game, &trash, "legacy-123").is_err());
    assert_eq!(
        fs::read(outside.join("legacy-123.trashinfo")).unwrap(),
        b"user index"
    );
    assert_eq!(
        fs::read(trash.join("legacy-123/a.package")).unwrap(),
        b"original mod"
    );
}

#[test]
fn cross_device_wrapper_restore_retires_wrapper_and_preserves_originals() {
    use std::os::unix::fs::MetadataExt;
    use plumcrate_core::lifecycle::list_trash_entries;
    let temp = tempfile::tempdir().unwrap();
    let cross = tempfile::Builder::new()
        .prefix("ts4-wrapper-restore-")
        .tempdir_in("/dev/shm")
        .unwrap();
    assert_ne!(
        fs::metadata(temp.path()).unwrap().dev(),
        fs::metadata(cross.path()).unwrap().dev()
    );
    let managed = temp.path().join("managed");
    let game = temp.path().join("Mods");
    let trash = cross.path().join("Trash/files");
    let entry = trash.join("legacy-123");
    let index = cross.path().join("Trash/info/legacy-123.trashinfo");
    fs::create_dir_all(&game).unwrap();
    fs::create_dir_all(&entry).unwrap();
    fs::create_dir_all(index.parent().unwrap()).unwrap();
    fs::write(entry.join("legacy.package"), b"original wrapper bytes").unwrap();
    let info = b"[Trash Info]\nPath=/The Sims 4/Mods/legacy\n";
    fs::write(&index, info).unwrap();
    assert_eq!(list_trash_entries(&trash).unwrap().len(), 1);
    restore_trashed_mod(&managed, &game, &trash, "legacy-123").unwrap();
    assert_eq!(
        fs::read(game.join("legacy.package")).unwrap(),
        b"original wrapper bytes"
    );
    assert!(!entry.exists());
    assert!(!index.exists());
    assert!(list_trash_entries(&trash).unwrap().is_empty());
    let preserved = fs::read_dir(&trash)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with(".ts4-restored-wrapper-")
        })
        .unwrap();
    let holding = fs::read_dir(preserved)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(fs::read(holding).unwrap(), b"original wrapper bytes");
    let retired_index = fs::read_dir(index.parent().unwrap())
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    assert_eq!(fs::read(retired_index).unwrap(), info);
}
