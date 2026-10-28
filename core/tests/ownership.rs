#![cfg(unix)]

use std::{fs, os::unix::fs::symlink, path::PathBuf};
use tempfile::TempDir;
use plumcrate_core::{
    error::ErrorCode,
    managed_storage::{create_managed_mod, read_managed_mod, ImportRequest},
    toggle::{apply_toggle, dry_run_toggle},
};

struct Fixture {
    temp: TempDir,
    managed: PathBuf,
    game: PathBuf,
    id: String,
}

impl Fixture {
    fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        let managed = temp.path().join("managed");
        let game = temp.path().join("instance/Mods");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&game).unwrap();
        fs::write(source.join("test.package"), b"original bytes").unwrap();
        let meta = create_managed_mod(
            &managed,
            ImportRequest {
                name: "Test".into(),
                slug: None,
                source_dir: source,
            },
        )
        .unwrap();
        Self {
            temp,
            managed,
            game,
            id: meta.mod_id,
        }
    }

    fn enable(&self) {
        assert!(
            apply_toggle(&self.managed, &self.game, &self.id, true)
                .unwrap()
                .applied
        );
    }
}

#[test]
fn disable_preserves_a_replacement_symlink() {
    let fixture = Fixture::new();
    fixture.enable();
    let link = fixture.game.join("test.package");
    fs::remove_file(&link).unwrap();
    let outside = fixture.temp.path().join("user.package");
    fs::write(&outside, b"user bytes").unwrap();
    symlink(&outside, &link).unwrap();
    let preview = dry_run_toggle(&fixture.managed, &fixture.game, &fixture.id, false).unwrap();
    assert!(!preview
        .operations
        .iter()
        .any(|op| op.action == "remove_symlink"));
    assert!(preview
        .issues
        .iter()
        .any(|issue| issue.code.as_deref() == Some("EXTERNAL_LINK")));
    let _ = apply_toggle(&fixture.managed, &fixture.game, &fixture.id, false);
    assert_eq!(fs::read_link(&link).unwrap(), outside);
    assert_eq!(fs::read(&link).unwrap(), b"user bytes");
}

#[test]
fn disable_removes_dangling_owned_links() {
    let fixture = Fixture::new();
    fixture.enable();
    fs::remove_file(
        fixture
            .managed
            .join("mods")
            .join(&fixture.id)
            .join("files/test.package"),
    )
    .unwrap();
    let result = apply_toggle(&fixture.managed, &fixture.game, &fixture.id, false).unwrap();
    assert!(result.applied);
    assert!(fs::symlink_metadata(fixture.game.join("test.package")).is_err());
}

#[test]
fn enable_never_overwrites_a_dangling_unmanaged_link() {
    let fixture = Fixture::new();
    let link = fixture.game.join("test.package");
    let missing = fixture.temp.path().join("missing.package");
    symlink(&missing, &link).unwrap();
    let preview = dry_run_toggle(&fixture.managed, &fixture.game, &fixture.id, true).unwrap();
    assert!(!preview.can_apply);
    assert_eq!(fs::read_link(&link).unwrap(), missing);
}

#[test]
fn supports_legacy_snake_case_migration_sidecars() {
    let fixture = Fixture::new();
    fixture.enable();
    fs::remove_dir_all(fixture.managed.join("mods").join(&fixture.id).join("links")).unwrap();
    fs::write(
        fixture
            .managed
            .join("mods")
            .join(&fixture.id)
            .join("links.json"),
        serde_json::to_vec(
            &serde_json::json!({"version": 1, "mod_id": fixture.id, "links": ["test.package"]}),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(
        apply_toggle(&fixture.managed, &fixture.game, &fixture.id, false)
            .unwrap()
            .applied
    );
    assert!(fs::symlink_metadata(fixture.game.join("test.package")).is_err());
}

#[test]
fn disabling_one_instance_keeps_the_other_instance_owned() {
    let fixture = Fixture::new();
    fixture.enable();
    let other = fixture.temp.path().join("other/Mods");
    fs::create_dir_all(&other).unwrap();
    assert!(
        apply_toggle(&fixture.managed, &other, &fixture.id, true)
            .unwrap()
            .applied
    );
    assert!(
        apply_toggle(&fixture.managed, &fixture.game, &fixture.id, false)
            .unwrap()
            .applied
    );
    assert_eq!(
        fs::read(other.join("test.package")).unwrap(),
        b"original bytes"
    );
    assert!(
        apply_toggle(&fixture.managed, &other, &fixture.id, false)
            .unwrap()
            .applied
    );
    assert!(fs::symlink_metadata(other.join("test.package")).is_err());
}

#[test]
fn read_rejects_a_mod_id_that_escapes_managed_storage() {
    let fixture = Fixture::new();
    assert_eq!(
        read_managed_mod(&fixture.managed, "../../outside")
            .unwrap_err()
            .code,
        ErrorCode::InvalidPath
    );
}

#[test]
fn toggle_rejects_a_symlink_parent_outside_the_game_folder() {
    let fixture = Fixture::new();
    let bundle = fixture.managed.join("mods").join(&fixture.id);
    let mut meta = read_managed_mod(&fixture.managed, &fixture.id).unwrap();
    fs::create_dir_all(bundle.join("files/Group")).unwrap();
    fs::rename(
        bundle.join("files/test.package"),
        bundle.join("files/Group/test.package"),
    )
    .unwrap();
    meta.files = vec!["Group/test.package".into()];
    plumcrate_core::managed_storage::write_managed_mod(&fixture.managed, &meta).unwrap();
    let outside = fixture.temp.path().join("user-folder");
    fs::create_dir_all(&outside).unwrap();
    symlink(&outside, fixture.game.join("Group")).unwrap();
    assert!(apply_toggle(&fixture.managed, &fixture.game, &fixture.id, true).is_err());
    assert!(fs::read_dir(outside).unwrap().next().is_none());
}

#[test]
fn failed_unlink_does_not_report_success_or_forget_ownership() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    fixture.enable();
    fs::set_permissions(&fixture.game, fs::Permissions::from_mode(0o500)).unwrap();
    let result = apply_toggle(&fixture.managed, &fixture.game, &fixture.id, false);
    fs::set_permissions(&fixture.game, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(result.is_err(), "an unlink failure must be visible");
    assert!(fs::symlink_metadata(fixture.game.join("test.package"))
        .unwrap()
        .file_type()
        .is_symlink());
    let retry = dry_run_toggle(&fixture.managed, &fixture.game, &fixture.id, false).unwrap();
    assert!(retry
        .operations
        .iter()
        .any(|op| op.action == "remove_symlink"));
}

#[test]
fn legacy_camel_sidecar_is_read_without_rewriting_or_removing_it() {
    let fixture = Fixture::new();
    let target = fs::canonicalize(fixture.managed.join("mods").join(&fixture.id))
        .unwrap()
        .join("files/test.package");
    symlink(target, fixture.game.join("test.package")).unwrap();
    let path = fixture
        .managed
        .join("mods")
        .join(&fixture.id)
        .join("links.json");
    let bytes = serde_json::to_vec(
        &serde_json::json!({"version":1,"modId":fixture.id,"links":["test.package"]}),
    )
    .unwrap();
    fs::write(&path, &bytes).unwrap();
    assert!(
        apply_toggle(&fixture.managed, &fixture.game, &fixture.id, false)
            .unwrap()
            .applied
    );
    assert_eq!(fs::read(path).unwrap(), bytes);
}

#[test]
fn metadata_edit_never_creates_an_external_bundle() {
    let fixture = Fixture::new();
    assert!(
        plumcrate_core::managed_storage::set_custom_display_name(
            &fixture.managed,
            "external",
            "Custom".into()
        )
        .is_err()
    );
    assert!(!fixture.managed.join("mods/external").exists());
}

#[test]
fn genuine_legacy_relative_link_with_parent_components_is_owned() {
    let fixture = Fixture::new();
    let relative = PathBuf::from("../../managed/mods")
        .join(&fixture.id)
        .join("files/test.package");
    symlink(&relative, fixture.game.join("test.package")).unwrap();
    let sidecar = fixture
        .managed
        .join("mods")
        .join(&fixture.id)
        .join("links.json");
    fs::write(
        &sidecar,
        serde_json::to_vec(
            &serde_json::json!({"version":1,"modId":fixture.id,"links":["test.package"]}),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(
        apply_toggle(&fixture.managed, &fixture.game, &fixture.id, false)
            .unwrap()
            .applied
    );
    assert!(!fixture.game.join("test.package").exists());
    assert!(sidecar.is_file());
}
