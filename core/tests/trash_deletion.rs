#![cfg(unix)]
use std::{fs, path::PathBuf};
use tempfile::TempDir;
use plumcrate_core::{
    lifecycle::{delete_reviewed_trash, review_trash_deletion, uninstall_managed_mod},
    managed_storage::{create_managed_mod, ImportRequest},
};

fn trashed(temp: &TempDir, name: &str) -> (PathBuf, PathBuf, String) {
    let source = temp.path().join(name);
    fs::create_dir(&source).unwrap();
    fs::write(source.join("mod.package"), name).unwrap();
    let managed = temp.path().join("managed");
    let meta = create_managed_mod(
        &managed,
        ImportRequest {
            name: name.into(),
            slug: Some(name.into()),
            source_dir: source,
        },
    )
    .unwrap();
    let trash = temp.path().join("Trash/files");
    let result =
        uninstall_managed_mod(&managed, &temp.path().join("Mods"), &trash, &meta.mod_id).unwrap();
    let name = PathBuf::from(result.trashed_path)
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    (managed, trash, name)
}

#[test]
fn empty_mod_trash_deletes_only_reviewed_app_bundles_and_their_indexes() {
    let temp = TempDir::new().unwrap();
    let (managed, trash, first) = trashed(&temp, "one");
    let (_, _, second) = trashed(&temp, "two");
    fs::write(trash.join("unrelated.package"), b"user trash").unwrap();
    let review = review_trash_deletion(&managed, &trash, &[]).unwrap();
    assert_eq!(review.entries().len(), 2);
    assert!(trash.join(&first).exists(), "review must not delete");
    delete_reviewed_trash(&managed, &trash, &review).unwrap();
    for name in [first, second] {
        assert!(!trash.join(&name).exists());
        assert!(!temp
            .path()
            .join("Trash/info")
            .join(format!("{name}.trashinfo"))
            .exists());
    }
    assert_eq!(
        fs::read(trash.join("unrelated.package")).unwrap(),
        b"user trash"
    );
    assert_eq!(
        fs::read(temp.path().join("one/mod.package")).unwrap(),
        b"one"
    );
}

#[test]
fn delete_one_preserves_others_and_refuses_changed_content_before_deleting_anything() {
    let temp = TempDir::new().unwrap();
    let (managed, trash, first) = trashed(&temp, "one");
    let (_, _, second) = trashed(&temp, "two");
    let review = review_trash_deletion(&managed, &trash, &[]).unwrap();
    fs::write(
        trash.join(&second).join("files/mod.package"),
        b"changed after review",
    )
    .unwrap();
    assert!(delete_reviewed_trash(&managed, &trash, &review).is_err());
    assert!(trash.join(&first).exists());
    assert!(trash.join(&second).exists());
    let review = review_trash_deletion(&managed, &trash, std::slice::from_ref(&first)).unwrap();
    delete_reviewed_trash(&managed, &trash, &review).unwrap();
    assert!(!trash.join(first).exists());
    assert!(trash.join(second).exists());
}

#[test]
fn deletion_rejects_unmanaged_entries_traversal_symlinks_and_other_scopes() {
    use std::os::unix::fs::symlink;
    let temp = TempDir::new().unwrap();
    let (managed, trash, name) = trashed(&temp, "one");
    fs::write(trash.join("other.package"), b"unmanaged").unwrap();
    for invalid in ["other.package", "../one", "", "/tmp"] {
        assert!(review_trash_deletion(&managed, &trash, &[invalid.into()]).is_err());
    }
    let review = review_trash_deletion(&managed, &trash, std::slice::from_ref(&name)).unwrap();
    assert!(delete_reviewed_trash(&temp.path().join("other-managed"), &trash, &review).is_err());
    symlink(temp.path().join("one"), trash.join(&name).join("escape")).unwrap();
    assert!(delete_reviewed_trash(&managed, &trash, &review).is_err());
    assert_eq!(
        fs::read(temp.path().join("one/mod.package")).unwrap(),
        b"one"
    );
    fs::remove_file(trash.join(&name).join("escape")).unwrap();
    let info = temp.path().join("Trash/info");
    fs::rename(&info, temp.path().join("saved-info")).unwrap();
    symlink(temp.path().join("saved-info"), &info).unwrap();
    assert!(delete_reviewed_trash(&managed, &trash, &review).is_err());
    assert!(trash.join(name).exists());
}
