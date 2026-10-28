#![cfg(unix)]
use std::{fs, os::unix::fs::symlink};
use plumcrate_core::{
    external_migration::migrate_external_mod,
    managed_storage::{create_managed_mod, ImportRequest},
    operation::recover_pending,
};
#[test]
fn migration_preserves_original_symlink_object_and_outside_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let managed = temp.path().join("managed");
    let game = temp.path().join("Game/Mods");
    fs::create_dir_all(&game).unwrap();
    let outside = temp.path().join("outside.package");
    fs::write(&outside, b"original").unwrap();
    symlink(&outside, game.join("External_main.package")).unwrap();
    let result = migrate_external_mod(&managed, &game, "External").unwrap();
    let index: serde_json::Value = serde_json::from_slice(
        &fs::read(
            managed
                .join("backups")
                .join(format!("{}.json", result.managed_mod_id)),
        )
        .unwrap(),
    )
    .unwrap();
    let backup = std::path::Path::new(index["originals"].as_str().unwrap());
    assert_eq!(
        fs::read_link(backup.join("External_main.package")).unwrap(),
        outside
    );
    assert_eq!(fs::read(outside).unwrap(), b"original");
    assert!(recover_pending(&managed).unwrap().is_empty());
}
#[test]
fn rejected_import_never_publishes_partial_bundle() {
    let temp = tempfile::tempdir().unwrap();
    let managed = temp.path().join("managed");
    let source = temp.path().join("source");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.package"), b"ok").unwrap();
    symlink("missing", source.join("z.package")).unwrap();
    assert!(create_managed_mod(
        &managed,
        ImportRequest {
            name: "Bad".into(),
            slug: None,
            source_dir: source
        }
    )
    .is_err());
    assert!(
        !managed.join("mods").exists()
            || fs::read_dir(managed.join("mods")).unwrap().next().is_none()
    );
    recover_pending(&managed).unwrap();
}

#[test]
fn restart_rolls_back_a_moved_original_and_created_link() {
    use plumcrate_core::operation::Transaction;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("managed");
    let game = temp.path().join("Mods");
    fs::create_dir_all(&game).unwrap();
    let original = game.join("original.package");
    let backup = game.join("preserved.package");
    fs::write(&original, b"original bytes").unwrap();
    let mut transaction = Transaction::new(&root, "external_migration").unwrap();
    transaction.move_path(&original, &backup).unwrap();
    transaction
        .create_link(
            &game,
            std::path::Path::new("original.package"),
            &root.join("mods/id/files/managed-target"),
        )
        .unwrap();
    drop(transaction);
    assert!(recover_pending(&root).unwrap().is_empty());
    assert_eq!(fs::read(&original).unwrap(), b"original bytes");
    assert!(!backup.exists());
    assert!(recover_pending(&root).unwrap().is_empty());
}
#[test]
fn recovery_preserves_a_new_user_replacement_and_keeps_journal() {
    use plumcrate_core::operation::Transaction;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("managed");
    let original = temp.path().join("original.package");
    let backup = temp.path().join("backup.package");
    fs::write(&original, b"original bytes").unwrap();
    let mut transaction = Transaction::new(&root, "trash").unwrap();
    transaction.move_path(&original, &backup).unwrap();
    drop(transaction);
    fs::write(&original, b"new user bytes").unwrap();
    assert!(!recover_pending(&root).unwrap().is_empty());
    assert_eq!(fs::read(&original).unwrap(), b"new user bytes");
    assert_eq!(fs::read(&backup).unwrap(), b"original bytes");
    assert!(fs::read_dir(root.join("operations"))
        .unwrap()
        .next()
        .is_some());
}
#[test]
fn restart_recovers_the_original_holding_across_devices() {
    use plumcrate_core::operation::Transaction;
    let temp = tempfile::tempdir().unwrap();
    let cross = tempfile::Builder::new()
        .prefix("ts4-recovery-")
        .tempdir_in("/dev/shm")
        .unwrap();
    let root = temp.path().join("managed");
    let original = temp.path().join("original.package");
    let destination = cross.path().join("copy.package");
    fs::write(&original, b"cross-device original").unwrap();
    let mut transaction = Transaction::new(&root, "trash").unwrap();
    transaction.move_path(&original, &destination).unwrap();
    drop(transaction);
    assert!(recover_pending(&root).unwrap().is_empty());
    assert_eq!(fs::read(&original).unwrap(), b"cross-device original");
}
#[test]
fn failure_after_first_restore_move_rolls_back_all_planned_paths() {
    use plumcrate_core::operation::Transaction;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("managed");
    let source = temp.path().join("first.package");
    let destination = temp.path().join("restored.package");
    fs::write(&source, b"first").unwrap();
    let mut transaction = Transaction::new(&root, "restore").unwrap();
    transaction.move_path(&source, &destination).unwrap();
    let missing = temp.path().join("missing.package");
    assert!(transaction
        .move_path(&missing, &temp.path().join("other.package"))
        .is_err());
    transaction.rollback().unwrap();
    assert_eq!(fs::read(source).unwrap(), b"first");
    assert!(!destination.exists());
}
#[test]
fn held_directory_mutation_survives_a_parent_path_swap() {
    use plumcrate_core::fs_scope::Directory;
    let temp = tempfile::tempdir().unwrap();
    let game = temp.path().join("Mods");
    let moved = temp.path().join("original-Mods");
    let outside = temp.path().join("outside");
    fs::create_dir(&game).unwrap();
    fs::create_dir(&outside).unwrap();
    let directory = Directory::open(&game).unwrap();
    fs::rename(&game, &moved).unwrap();
    symlink(&outside, &game).unwrap();
    directory
        .symlink(
            std::path::Path::new("test.package"),
            std::path::Path::new("/managed/target"),
        )
        .unwrap();
    assert!(fs::symlink_metadata(moved.join("test.package"))
        .unwrap()
        .file_type()
        .is_symlink());
    assert!(fs::read_dir(outside).unwrap().next().is_none());
}

#[test]
#[ignore = "child fixture invoked by writer_authority_serializes_independent_processes"]
fn process_lock_child() {
    let root = std::path::PathBuf::from(std::env::var_os("TS4_LOCK_FIXTURE_ROOT").unwrap());
    fs::write(root.join("child-started"), b"started").unwrap();
    let _guard = plumcrate_core::operation::acquire(&root).unwrap();
    fs::write(root.join("child-acquired"), b"acquired").unwrap();
}
#[test]
fn writer_authority_serializes_independent_processes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let guard = plumcrate_core::operation::acquire(root).unwrap();
    let different_tmp = root.join("different-tmp");
    fs::create_dir(&different_tmp).unwrap();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "process_lock_child", "--ignored"])
        .env("TS4_LOCK_FIXTURE_ROOT", root)
        .env("TMPDIR", &different_tmp)
        .spawn()
        .unwrap();
    let start = std::time::Instant::now();
    while !root.join("child-started").exists() {
        assert!(start.elapsed() < std::time::Duration::from_secs(10));
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    std::thread::sleep(std::time::Duration::from_millis(100));
    assert!(!root.join("child-acquired").exists());
    drop(guard);
    assert!(child.wait().unwrap().success());
    assert!(root.join("child-acquired").exists());
}

#[test]
fn restart_restores_ownership_record_together_with_disabled_links() {
    use plumcrate_core::{link_ownership, operation::Transaction, toggle};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("managed");
    let source = temp.path().join("source");
    let game = temp.path().join("Mods");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&game).unwrap();
    fs::write(source.join("a.package"), b"a").unwrap();
    let meta = create_managed_mod(
        &root,
        ImportRequest {
            name: "Test".into(),
            slug: None,
            source_dir: source,
        },
    )
    .unwrap();
    toggle::apply_toggle(&root, &game, &meta.mod_id, true).unwrap();
    let record = link_ownership::record_path(&root, &meta.mod_id, &game).unwrap();
    let original = fs::read(&record).unwrap();
    let target = fs::read_link(game.join("a.package")).unwrap();
    let mut transaction = Transaction::new(&root, "disable").unwrap();
    transaction
        .remove_link(&game, std::path::Path::new("a.package"), &target)
        .unwrap();
    link_ownership::write_transactional(&root, &meta.mod_id, &game, vec![], &mut transaction)
        .unwrap();
    drop(transaction);
    assert!(recover_pending(&root).unwrap().is_empty());
    assert_eq!(fs::read(&record).unwrap(), original);
    assert_eq!(fs::read_link(game.join("a.package")).unwrap(), target);
    assert!(toggle::dry_run_toggle(&root, &game, &meta.mod_id, false)
        .unwrap()
        .operations
        .iter()
        .any(|o| o.action == "remove_symlink"));
}
#[test]
fn corrupted_journal_path_is_preserved_without_touching_an_outside_file() {
    use plumcrate_core::operation::Transaction;
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("managed");
    let from = temp.path().join("from");
    let to = temp.path().join("to");
    fs::create_dir(&from).unwrap();
    fs::create_dir(&to).unwrap();
    fs::write(from.join("original.package"), b"original").unwrap();
    let outside = temp.path().join("outside.package");
    fs::write(&outside, b"outside user file").unwrap();
    let mut transaction = Transaction::new(&root, "trash").unwrap();
    transaction
        .move_path(
            &from.join("original.package"),
            &to.join("preserved.package"),
        )
        .unwrap();
    drop(transaction);
    let path = fs::read_dir(root.join("operations"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["journal"]["steps"][0]["destination"] = outside.to_string_lossy().to_string().into();
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(!recover_pending(&root).unwrap().is_empty());
    assert!(path.exists());
    assert_eq!(fs::read(&outside).unwrap(), b"outside user file");
    assert_eq!(fs::read(to.join("preserved.package")).unwrap(), b"original");
    // Even a recomputed corruption checksum does not authorize an unrecorded scope.
    use sha2::{Digest, Sha256};
    value["checksum"] = format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(&value["journal"]).unwrap())
    )
    .into();
    fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
    assert!(!recover_pending(&root).unwrap().is_empty());
    assert_eq!(fs::read(&outside).unwrap(), b"outside user file");
}
