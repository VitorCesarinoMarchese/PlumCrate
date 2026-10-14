use super::*;
use serde_json::json;
use std::io::Cursor;
use std::os::unix::fs::PermissionsExt;

pub(super) fn image() -> Vec<u8> {
    let mut bytes = vec![0; 128];
    bytes[..4].copy_from_slice(b"\x7fELF");
    bytes[4] = 2;
    bytes[5] = 1;
    bytes[8..11].copy_from_slice(b"AI\x02");
    bytes[18..20].copy_from_slice(&62u16.to_le_bytes());
    bytes
}

fn release(version: &str, arch: &str) -> serde_json::Value {
    let name = format!("ts4-mod-manager-native-{version}-{arch}.AppImage");
    let base = format!("https://github.com/{REPOSITORY}/releases");
    json!({"tag_name": format!("v{version}"), "html_url": format!("{base}/tag/v{version}"),
        "draft": false, "prerelease": version.contains('-'),
        "assets": [{"name": name, "size": 128,
            "browser_download_url": format!("{base}/download/v{version}/{name}"),
            "digest": format!("sha256:{:x}", Sha256::digest(image()))}]})
}

pub(super) fn selected() -> Release {
    select_release(
        &json!([release("0.0.1-alpha.2", "x86_64")]).to_string(),
        "0.0.1-alpha.1",
        "x86_64",
    )
    .unwrap()
    .unwrap()
}

pub(super) fn target(directory: &std::path::Path) -> Target {
    let path = directory.join("My manager.AppImage");
    std::fs::write(&path, image()).unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    Target::open(path).unwrap()
}

#[test]
fn selects_newest_matching_architecture_with_semver_prereleases() {
    let feed = json!([
        release("0.0.1-alpha.2", "x86_64"),
        release("0.0.1-alpha.10", "x86_64"),
        release("2.0.0", "aarch64")
    ]);
    let update = select_release(&feed.to_string(), "0.0.1-alpha.1", "x86_64")
        .unwrap()
        .unwrap();
    assert_eq!(update.version, "0.0.1-alpha.10");
    assert!(
        select_release(&feed.to_string(), "0.0.1", "x86_64")
            .unwrap()
            .is_none()
    );
}

#[test]
fn ignores_drafts_current_versions_and_empty_releases() {
    let mut draft = release("3.0.0", "x86_64");
    draft["draft"] = json!(true);
    let feed = json!([draft, release("0.0.1-alpha.1", "x86_64")]);
    assert!(
        select_release(&feed.to_string(), "0.0.1-alpha.1", "x86_64")
            .unwrap()
            .is_none()
    );
    assert!(
        select_release("[]", "0.0.1-alpha.1", "x86_64")
            .unwrap()
            .is_none()
    );
    assert!(select_release("not JSON", "0.0.1", "x86_64").is_err());
}

#[test]
fn refuses_untrusted_asset_urls_and_missing_verification() {
    for url in [
        "http://github.com/asset",
        "https://example.com/asset",
        "https://github.com/other/repository/releases/download/v1/asset",
    ] {
        let mut item = release("1.0.0", "x86_64");
        item["assets"][0]["browser_download_url"] = json!(url);
        assert!(select_release(&json!([item]).to_string(), "0.0.1", "x86_64").is_err());
    }
    let mut item = release("1.0.0", "x86_64");
    item["assets"][0]["digest"] = serde_json::Value::Null;
    assert!(select_release(&json!([item.clone()]).to_string(), "0.0.1", "x86_64").is_err());
    let name = item["assets"][0]["name"].as_str().unwrap().to_string() + ".sha256";
    item["assets"].as_array_mut().unwrap().push(json!({"name": name, "size": 200,
        "browser_download_url": format!("https://github.com/{REPOSITORY}/releases/download/v1.0.0/{name}")}));
    assert!(
        select_release(&json!([item]).to_string(), "0.0.1", "x86_64")
            .unwrap()
            .is_some()
    );
}

#[test]
fn checksum_must_name_the_exact_asset() {
    let update = selected();
    let digest = format!("{:x}", Sha256::digest(image()));
    assert_eq!(
        parse_checksum(
            &format!("{digest}  {}\n", update.asset.name),
            &update.asset.name
        )
        .unwrap(),
        digest
    );
    assert!(parse_checksum(&format!("{digest}  other.AppImage\n"), &update.asset.name).is_err());
    assert!(parse_checksum("bad  app.AppImage", "app.AppImage").is_err());
}

#[test]
fn verified_download_does_not_replace_app_until_install_and_keeps_backup() {
    let directory = tempfile::tempdir().unwrap();
    let target = target(directory.path());
    let original = std::fs::read(&target.path).unwrap();
    let mut payload = image();
    payload[100] = 42;
    let mut update = selected();
    update.sha256 = Some(format!("{:x}", Sha256::digest(&payload)));
    let staged = stage_download(
        target.clone(),
        &update,
        &update.sha256.clone().unwrap(),
        Cursor::new(&payload),
        |_, _| {},
    )
    .unwrap();
    assert_eq!(std::fs::read(&target.path).unwrap(), original);
    let installed = staged.install().unwrap();
    assert_eq!(
        std::fs::metadata(installed.backup.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(std::fs::read(&target.path).unwrap(), payload);
    assert_eq!(std::fs::read(&installed.backup).unwrap(), original);
    assert_eq!(
        std::fs::metadata(&target.path)
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o755
    );
    installed.rollback().unwrap();
    assert_eq!(std::fs::read(&target.path).unwrap(), original);
}

#[test]
fn corrupt_truncated_oversized_or_non_appimage_downloads_leave_original_untouched() {
    let directory = tempfile::tempdir().unwrap();
    let target = target(directory.path());
    let original = std::fs::read(&target.path).unwrap();
    let update = selected();
    for payload in [vec![0; 128], image()[..64].to_vec(), vec![0; 129]] {
        assert!(
            stage_download(
                target.clone(),
                &update,
                &update.sha256.clone().unwrap(),
                Cursor::new(payload),
                |_, _| {}
            )
            .is_err()
        );
        assert_eq!(std::fs::read(&target.path).unwrap(), original);
    }
    let bytes = vec![0; 128];
    assert!(
        stage_download(
            target.clone(),
            &update,
            &format!("{:x}", Sha256::digest(&bytes)),
            Cursor::new(bytes),
            |_, _| {}
        )
        .is_err()
    );
    assert_eq!(std::fs::read_dir(directory.path()).unwrap().count(), 1);
}

#[test]
fn install_refuses_an_appimage_changed_after_download() {
    let directory = tempfile::tempdir().unwrap();
    let target = target(directory.path());
    let update = selected();
    let staged = stage_download(
        target.clone(),
        &update,
        &update.sha256.clone().unwrap(),
        Cursor::new(image()),
        |_, _| {},
    )
    .unwrap();
    std::fs::write(&target.path, b"a different user file").unwrap();
    assert!(staged.install().is_err());
    assert_eq!(
        std::fs::read(&target.path).unwrap(),
        b"a different user file"
    );
}

#[test]
fn target_refuses_symlinks_and_non_appimages() {
    let directory = tempfile::tempdir().unwrap();
    let target = target(directory.path());
    let link = directory.path().join("linked.AppImage");
    std::os::unix::fs::symlink(&target.path, &link).unwrap();
    assert!(Target::open(link).is_err());
    let other = directory.path().join("unmanaged");
    std::fs::write(&other, b"user file").unwrap();
    assert!(Target::open(other).is_err());
}

#[test]
fn old_settings_enable_checks_and_opt_out_survives_save() {
    let directory = tempfile::tempdir().unwrap();
    let mut settings =
        crate::settings::Settings::parse(r#"{"version":1,"theme":"system","gameRoots":[]}"#)
            .unwrap();
    assert!(settings.check_app_updates);
    settings.check_app_updates = false;
    let path = directory.path().join("settings.json");
    settings.save(&path).unwrap();
    assert!(
        !crate::settings::Settings::load(&path)
            .unwrap()
            .check_app_updates
    );
}

#[test]
fn restart_rolls_back_when_the_new_process_exits_before_showing_a_window() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("new.AppImage");
    let backup = directory.path().join("previous.AppImage");
    std::fs::write(&path, b"#!/bin/sh\nexit 1\n").unwrap();
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::write(&backup, b"previous app").unwrap();
    let installed = Installed {
        path: path.clone(),
        replacement: stamp(&path).unwrap(),
        previous: stamp(&backup).unwrap(),
        backup,
    };
    assert!(installed.restart(vec![]).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), b"previous app");
}

#[test]
fn rollback_preserves_a_user_file_replaced_during_restart() {
    let directory = tempfile::tempdir().unwrap();
    let target = target(directory.path());
    let release = selected();
    let installed = stage_download(
        target.clone(),
        &release,
        release.sha256.as_ref().unwrap(),
        Cursor::new(image()),
        |_, _| {},
    )
    .unwrap()
    .install()
    .unwrap();
    std::fs::write(&target.path, b"new user file").unwrap();
    assert!(installed.rollback().is_err());
    assert_eq!(std::fs::read(target.path).unwrap(), b"new user file");
    assert!(installed.backup.is_file());
}

#[test]
fn startup_receipt_cannot_overwrite_user_files_or_escape_the_backup_folder() {
    let root = tempfile::tempdir().unwrap();
    let appimage = root.path().join("app.AppImage");
    let directory = tempfile::Builder::new()
        .prefix(".ts4-previous-")
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir_in(root.path())
        .unwrap();
    std::fs::write(directory.path().join("previous.AppImage"), image()).unwrap();
    let receipt = directory.path().join("started");
    acknowledge_at(&receipt, &appimage).unwrap();
    assert_eq!(std::fs::read(&receipt).unwrap(), b"started");
    std::fs::write(&receipt, b"user data").unwrap();
    assert!(acknowledge_at(&receipt, &appimage).is_err());
    assert_eq!(std::fs::read(&receipt).unwrap(), b"user data");
    assert!(acknowledge_at(&root.path().join("started"), &appimage).is_err());
    assert!(!root.path().join("started").exists());
}

#[test]
fn redirect_hosts_and_response_sizes_are_bounded() {
    for uri in [
        "https://evil.example/asset",
        "http://github.com/asset",
        "https://github.com:8443/asset",
        "https://user:password@github.com/asset",
    ] {
        assert!(!allowed_host(&url::Url::parse(uri).unwrap()));
    }
    assert!(allowed_host(
        &url::Url::parse("https://release-assets.githubusercontent.com/asset").unwrap()
    ));
    assert!(bounded_text(Cursor::new(vec![0; 5]), 4).is_err());
}

#[test]
#[ignore = "queries GitHub's public release API"]
fn live_github_release_check_uses_public_feed() {
    if let Some(release) = check().unwrap() {
        assert!(
            Version::parse(&release.version)
                .unwrap()
                .cmp_precedence(&Version::parse(env!("CARGO_PKG_VERSION")).unwrap())
                .is_gt()
        );
        assert!(release.asset.browser_download_url.starts_with(&format!(
            "https://github.com/{REPOSITORY}/releases/download/"
        )));
    }
}

#[test]
#[ignore = "requires TS4_TEST_APPIMAGE pointing to a packaged build"]
fn packaged_appimage_can_be_verified_installed_and_restored() {
    let source = std::path::PathBuf::from(
        std::env::var_os("TS4_TEST_APPIMAGE").expect("Set TS4_TEST_APPIMAGE"),
    );
    let hash_file = |path: &std::path::Path| {
        let mut hash = Sha256::new();
        std::io::copy(&mut std::fs::File::open(path).unwrap(), &mut hash).unwrap();
        format!("{:x}", hash.finalize())
    };
    let digest = hash_file(&source);
    let directory = tempfile::tempdir().unwrap();
    let installed_path = directory.path().join("manager with spaces.AppImage");
    std::fs::copy(&source, &installed_path).unwrap();
    std::fs::OpenOptions::new()
        .append(true)
        .open(&installed_path)
        .unwrap()
        .write_all(b"old fixture")
        .unwrap();
    let previous_digest = hash_file(&installed_path);
    let target = Target::open(installed_path.clone()).unwrap();
    let mut release = selected();
    release.version = env!("CARGO_PKG_VERSION").into();
    release.asset.size = std::fs::metadata(&source).unwrap().len();
    let staged = stage_download(
        target,
        &release,
        &digest,
        std::fs::File::open(&source).unwrap(),
        |_, _| {},
    )
    .unwrap();
    probe_version(&staged.payload, &release.version).unwrap();
    let installed = staged.install().unwrap();
    assert_eq!(hash_file(&installed.path), digest);
    assert_eq!(hash_file(&installed.backup), previous_digest);
    probe_version(&installed.path, &release.version).unwrap();
    installed.rollback().unwrap();
    assert_eq!(hash_file(&installed_path), previous_digest);
    assert_eq!(hash_file(&source), digest);
}

#[test]
#[ignore = "requires TS4_TEST_APPIMAGE and a graphical Linux session"]
fn packaged_appimage_restart_acknowledges_a_rendered_window() {
    let source = std::path::PathBuf::from(
        std::env::var_os("TS4_TEST_APPIMAGE").expect("Set TS4_TEST_APPIMAGE"),
    );
    let directory = tempfile::tempdir().unwrap();
    let installed_path = directory.path().join("manager with spaces.AppImage");
    std::fs::copy(&source, &installed_path).unwrap();
    let target = Target::open(installed_path).unwrap();
    let mut release = selected();
    release.asset.size = std::fs::metadata(&source).unwrap().len();
    let mut hash = Sha256::new();
    std::io::copy(&mut std::fs::File::open(&source).unwrap(), &mut hash).unwrap();
    let staged = stage_download(
        target,
        &release,
        &format!("{:x}", hash.finalize()),
        std::fs::File::open(&source).unwrap(),
        |_, _| {},
    )
    .unwrap();
    let installed = staged.install().unwrap();
    let game = directory.path().join("The Sims 4");
    std::fs::create_dir_all(game.join("Mods")).unwrap();
    std::fs::write(game.join("Mods/fixture.package"), b"fixture package").unwrap();
    let screenshot = directory.path().join("started.png");
    installed
        .restart(vec![
            "--home".into(),
            directory.path().join("home").into_os_string(),
            "--root".into(),
            game.clone().into_os_string(),
            "--screenshot".into(),
            screenshot.clone().into_os_string(),
        ])
        .unwrap();
    assert_eq!(
        std::fs::read(installed.backup.with_file_name("started")).unwrap(),
        b"started"
    );
    let start = Instant::now();
    while !screenshot.exists() {
        assert!(
            start.elapsed() < Duration::from_secs(30),
            "updated window did not finish rendering"
        );
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        std::fs::read(screenshot)
            .unwrap()
            .starts_with(b"\x89PNG\r\n\x1a\n")
    );
    assert_eq!(
        std::fs::read(game.join("Mods/fixture.package")).unwrap(),
        b"fixture package"
    );
    assert!(installed.backup.is_file());
    installed.rollback().unwrap();
}
