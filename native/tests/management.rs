use ts4_mod_manager_native::settings::Settings;
#[test]
fn settings_transfer_omits_key_and_rejects_unknown_schema() {
    let settings=Settings::parse(r#"{"version":1,"theme":"system","gameRoots":["/game"],"curseforgeApiKey":"fixture-secret"}"#).unwrap();
    assert!(!settings.export(false).unwrap().contains("fixture-secret"));
    assert!(settings.export(true).unwrap().contains("fixture-secret"));
    assert!(Settings::parse(r#"{"version":2,"theme":"system","gameRoots":[]}"#).is_err());
    assert!(Settings::parse(r#"{"version":1,"theme":"custom","gameRoots":[]}"#).is_err());
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    settings.save(&path).unwrap();
    assert_eq!(Settings::load(&path).unwrap().game_roots, vec!["/game"]);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
}
#[test]
fn approved_queue_survives_worker_drop() {
    use ts4_mod_manager_native::management::{Action, Identity, Job, Mutations};
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    std::fs::create_dir(&source).unwrap();
    std::fs::write(source.join("a.package"), b"fixture").unwrap();
    let managed = dir.path().join("managed");
    let root = dir.path().join("game");
    std::fs::create_dir_all(root.join("Mods")).unwrap();
    let worker = Mutations::start(|| {}).unwrap();
    for operation in 1..=2 {
        worker
            .submit(Job {
                identity: Identity {
                    operation,
                    root: root.clone(),
                    entry: None,
                    generation: 1,
                },
                managed: managed.clone(),
                trash: dir.path().join("trash"),
                action: Action::ImportFolder {
                    path: source.clone(),
                    name: format!("Mod {operation}"),
                },
            })
            .unwrap();
    }
    drop(worker);
    assert_eq!(std::fs::read_dir(managed.join("mods")).unwrap().count(), 2);
}
#[test]
fn settings_normalize_roots_and_reject_relative_paths() {
    let settings=Settings::parse(r#"{"version":1,"theme":"system","gameRoots":["/game/./folder/","/game/folder"],"selectedRoot":"/game/folder/"}"#).unwrap();
    assert_eq!(settings.game_roots, vec!["/game/folder"]);
    assert_eq!(settings.selected_root.as_deref(), Some("/game/folder"));
    assert!(Settings::parse(r#"{"version":1,"theme":"light","gameRoots":["relative"]}"#).is_err());
}
#[test]
fn scoped_identity_requires_same_root_entry_and_generation() {
    use ts4_mod_manager_native::{catalog::EntryId, management::Identity};
    let root: std::path::PathBuf = "/a".into();
    let entry = EntryId::Managed("same".into());
    let id = Identity {
        operation: 1,
        root: root.clone(),
        entry: Some(entry.clone()),
        generation: 3,
    };
    assert!(id.current(Some(&root), Some(&entry), 3));
    assert!(!id.current(Some(&"/b".into()), Some(&entry), 3));
    assert!(!id.current(Some(&root), Some(&EntryId::External("same".into())), 3));
    assert!(!id.current(Some(&root), Some(&entry), 4));
}
#[test]
fn manual_source_rejects_credentials_before_metadata_write() {
    use ts4_mod_manager_native::catalog::EntryId;
    use ts4_mod_manager_native::management::{Action, Identity, Job, execute};
    let dir = tempfile::tempdir().unwrap();
    let job = Job {
        identity: Identity {
            operation: 1,
            root: dir.path().join("game"),
            entry: Some(EntryId::Managed("fixture".into())),
            generation: 1,
        },
        managed: dir.path().join("managed"),
        trash: dir.path().join("trash"),
        action: Action::ManualSource("https://user:secret@example.org/test".into()),
    };
    let Err(error) = execute(&job) else {
        panic!("credential URL accepted")
    };
    assert!(error.contains("credentials"));
    assert!(!job.managed.exists());
}
#[test]
fn malformed_settings_remain_unchanged_and_errors_hide_secrets() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    let raw = b"{ secret-fixture-key malformed";
    std::fs::write(&path, raw).unwrap();
    let Err(error) = Settings::load(&path) else {
        panic!("invalid settings accepted")
    };
    assert!(!error.contains("secret-fixture-key"));
    assert_eq!(std::fs::read(path).unwrap(), raw);
}
#[test]
fn settings_save_rejects_invalid_in_memory_roots_before_touching_disk() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("new/settings.json");
    let settings = Settings {
        game_roots: vec!["relative".into()],
        ..Default::default()
    };
    assert!(settings.save(&path).is_err());
    assert!(!path.parent().unwrap().exists());
}
#[test]
fn settings_reject_nul_and_bounded_transfer_reads() {
    assert!(
        Settings::parse(r#"{"version":1,"theme":"light","gameRoots":["/game\u0000folder"]}"#)
            .is_err()
    );
    let directory = tempfile::tempdir().unwrap();
    let file = directory.path().join("large.json");
    std::fs::write(&file, vec![b' '; 1024 * 1024 + 1]).unwrap();
    let Err(error) = Settings::read_transfer(&file) else {
        panic!("oversized import accepted")
    };
    assert!(error.contains("1 MiB"));
}
#[test]
fn settings_import_preserves_existing_key_when_omitted() {
    let local = Settings {
        curseforge_api_key: Some("fixture-key".into()),
        ..Default::default()
    };
    let mut incoming = Settings::default();
    incoming.preserve_omitted_key(&local);
    assert_eq!(incoming.curseforge_api_key, local.curseforge_api_key);
    let mut replacement = Settings {
        curseforge_api_key: Some("replacement-fixture".into()),
        ..Default::default()
    };
    replacement.preserve_omitted_key(&local);
    assert_eq!(
        replacement.curseforge_api_key.as_deref(),
        Some("replacement-fixture")
    );
}
#[test]
fn settings_export_never_overwrites_an_existing_user_file() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("existing.json");
    std::fs::write(&path, b"user original").unwrap();
    assert!(Settings::default().save_new(&path).is_err());
    assert_eq!(std::fs::read(path).unwrap(), b"user original");
}

#[test]
fn imported_folders_and_archives_start_enabled_in_the_selected_game() {
    use std::io::Write;
    use ts4_mod_manager_native::management::{Action, Identity, Job, execute};
    for archive in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("source");
        let game = dir.path().join("game");
        let managed = dir.path().join("managed");
        std::fs::create_dir(&source).unwrap();
        std::fs::create_dir_all(game.join("Mods")).unwrap();
        std::fs::write(source.join("fixture.package"), b"original").unwrap();
        let zip_path = dir.path().join("fixture.zip");
        let action = if archive {
            let mut zip = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
            zip.start_file("fixture.package", zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(b"original").unwrap();
            zip.finish().unwrap();
            Action::ImportZip { path: zip_path.clone(), name: "Fixture".into() }
        } else { Action::ImportFolder { path: source.clone(), name: "Fixture".into() } };
        let archive_before = archive.then(|| std::fs::read(&zip_path).unwrap());
        execute(&Job { identity: Identity {operation:1, root:game.clone(), entry:None, generation:1}, managed:managed.clone(),trash:dir.path().join("trash"),action }).unwrap();
        let mods = ts4_mod_manager_native::worker::scan(&game, &managed).unwrap().mods;
        assert_eq!(mods.len(), 1);
        assert!(mods[0].enabled, "new imports should already be enabled");
        assert_eq!(std::fs::read(source.join("fixture.package")).unwrap(), b"original");
        if let Some(before) = archive_before { assert_eq!(std::fs::read(zip_path).unwrap(), before); }
    }
}

#[test]
fn a_blocked_auto_enable_keeps_the_import_and_unmanaged_file() {
    use ts4_mod_manager_native::management::{Action, Identity, Job, Output, execute};
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("source");
    let game = dir.path().join("game");
    let managed = dir.path().join("managed");
    std::fs::create_dir(&source).unwrap();
    std::fs::create_dir_all(game.join("Mods")).unwrap();
    std::fs::write(source.join("fixture.package"), b"download").unwrap();
    let existing = game.join("Mods/fixture.package");
    std::fs::write(&existing, b"unmanaged original").unwrap();
    let result = execute(&Job { identity: Identity {operation:1, root:game.clone(), entry:None, generation:1}, managed:managed.clone(),trash:dir.path().join("trash"),action: Action::ImportFolder {path:source,name:"Fixture".into()} }).unwrap();
    let Output::Imported {enable_error:Some(error), ..} = result else {panic!("import must report why it stayed disabled")};
    assert!(error.contains("collision"));
    assert_eq!(std::fs::read(&existing).unwrap(), b"unmanaged original");
    let entry = std::fs::read_dir(managed.join("mods")).unwrap().next().unwrap().unwrap();
    assert_eq!(std::fs::read(entry.path().join("files/fixture.package")).unwrap(), b"download");
    let id = entry.file_name().to_string_lossy().into_owned();
    let job = Job {identity:Identity {operation:2,root:game.clone(),entry:Some(ts4_mod_manager_native::catalog::EntryId::Managed(id)),generation:1},managed,trash:dir.path().join("trash"),action:Action::Toggle {enabled:true}};
    assert!(matches!(execute(&job), Err(error) if error.contains("collision")));
    assert_eq!(std::fs::read(&existing).unwrap(), b"unmanaged original");
}
