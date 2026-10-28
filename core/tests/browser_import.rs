use serde_json::{json, Value};
use std::{fs, io::Write, path::Path};
use plumcrate_core::{
    browser_import::{self, DownloadWatch, ImportPermit},
    curseforge_client::{
        CurseForgeClient, CurseForgeRequest, CurseForgeTransport, SourceLookupError,
    },
    managed_storage,
};
struct Provider {
    project: Value,
    files: Vec<Value>,
}
impl CurseForgeTransport for Provider {
    fn get(&self, r: &CurseForgeRequest) -> Result<(u16, String), SourceLookupError> {
        Ok((200, if r.url.ends_with("/42") { json!({"data":self.project}) } else if r.url.ends_with("/files/100") { json!({"data":self.files[0]}) } else {json!({"data":self.files,"pagination":{"index":0,"pageSize":50,"resultCount":self.files.len(),"totalCount":self.files.len()}})}.to_string()))
    }
}
fn provider(bytes: &[u8], filename: &str) -> Provider {
    use md5::Md5;
    use sha1::{Digest, Sha1};
    Provider {
        project: json!({"id":42,"gameId":78062,"name":"Fixture","authors":[{"name":"Author"}],"links":{"websiteUrl":"https://www.curseforge.com/sims4/mods/fixture"},"allowModDistribution":false}),
        files: vec![
            json!({"id":100,"modId":42,"fileName":filename,"displayName":"Version 1","fileDate":"2026-10-01T00:00:00Z","releaseType":1,"isAvailable":true,"fileStatus":4,"gameVersions":["1.0"],"fileLength":bytes.len(),"hashes":[{"algo":1,"value":format!("{:x}",Sha1::digest(bytes))},{"algo":2,"value":format!("{:x}",Md5::digest(bytes))}]}),
        ],
    }
}
fn project(p: Provider) -> browser_import::BrowserProject {
    browser_import::load_browser_project(&CurseForgeClient::new(Some("fixture"), p).unwrap(), 42)
        .unwrap()
}

#[test]
fn dependencies_belong_to_each_file_and_distinguish_empty_partial_and_missing() {
    use browser_import::{DependencyCompleteness, RelationKind};
    let mut p=provider(b"package", "mod.package");
    let mut older=p.files[0].clone();
    older["id"]=json!(101);
    older["dependencies"]=json!([]);
    p.files[0]["dependencies"]=json!([
        {"modId":9,"relationType":3},
        {"modId":10,"relationType":2},
        {"modId":11,"relationType":5},
        {"modId":12,"relationType":99},
        {"modId":0,"relationType":3}
    ]);
    p.files.push(older);
    let result=project(p);
    let current=&result.releases.iter().find(|r|r.release.file_id==100).unwrap().dependencies;
    assert_eq!(current.relations.len(),4);
    assert_eq!(current.completeness,DependencyCompleteness::Partial);
    assert_eq!(current.relations[0].kind,RelationKind::Required);
    assert!(!current.relations[2].kind.can_install());
    assert!(!current.relations[3].kind.can_install());
    let old=&result.releases.iter().find(|r|r.release.file_id==101).unwrap().dependencies;
    assert!(old.relations.is_empty());
    assert_eq!(old.completeness,DependencyCompleteness::Complete);
    let missing=project(provider(b"package", "mod.package"));
    assert_eq!(missing.releases[0].dependencies.completeness,DependencyCompleteness::Unknown);
}
fn match_download(
    dir: &Path,
    p: &browser_import::BrowserProject,
) -> browser_import::MatchedDownload {
    let mut watch = DownloadWatch::new(dir, &p.releases[0]).unwrap();
    let permit = ImportPermit::default();
    assert!(watch.poll(&permit).unwrap().is_none());
    watch.poll(&permit).unwrap().unwrap()
}
#[test]
fn provider_releases_require_matching_identity_public_status_supported_content_and_hashes() {
    let mut p = provider(b"package", "mod.package");
    let valid = p.files[0].clone();
    for (index, (key, value)) in [
        ("modId", json!(43)),
        ("releaseType", json!(2)),
        ("isAvailable", json!(false)),
        ("fileStatus", json!(1)),
        ("fileName", json!("mod.rar")),
        ("fileLength", json!(0)),
        ("hashes", json!([])),
    ]
    .into_iter()
    .enumerate()
    {
        let mut invalid = valid.clone();
        invalid["id"] = json!(200 + index);
        invalid[key] = value;
        p.files.push(invalid);
    }
    let p = project(p);
    assert_eq!(p.releases.len(), 1);
    assert_eq!(
        p.source_url,
        "https://www.curseforge.com/sims4/mods/fixture"
    );
    assert_eq!(
        p.releases[0].navigation_url(&p),
        "https://www.curseforge.com/sims4/mods/fixture/download/100"
    );
}
#[test]
fn wrong_game_and_forged_project_page_reject_the_project() {
    for (key, value) in [
        ("gameId", json!(1)),
        ("id", json!(43)),
        (
            "links",
            json!({"websiteUrl":"https://www.curseforge.com.attacker/sims4/mods/fixture"}),
        ),
    ] {
        let mut p = provider(b"x", "mod.package");
        p.project[key] = value;
        assert!(browser_import::load_browser_project(
            &CurseForgeClient::new(Some("fixture"), p).unwrap(),
            42
        )
        .is_err());
    }
}
#[test]
fn watches_renamed_completed_regular_files_and_ignores_partial_symlink_and_wrong_hash() {
    let t = tempfile::tempdir().unwrap();
    let p = project(provider(b"right", "mod.package"));
    fs::write(t.path().join("mod.package.part"), b"right").unwrap();
    fs::write(t.path().join("wrong.package"), b"wrong").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        t.path().join("mod.package.part"),
        t.path().join("linked.package"),
    )
    .unwrap();
    let mut watch = DownloadWatch::new(t.path(), &p.releases[0]).unwrap();
    let permit = ImportPermit::default();
    for _ in 0..3 {
        assert!(watch.poll(&permit).unwrap().is_none());
    }
    fs::write(t.path().join("renamed (1).package"), b"right").unwrap();
    assert!(watch.poll(&permit).unwrap().is_none());
    assert!(watch.poll(&permit).unwrap().is_some());
}
#[test]
fn verified_raw_import_attaches_baseline_enables_preserves_original_and_deduplicates() {
    let t = tempfile::tempdir().unwrap();
    let downloads = t.path().join("Downloads");
    let managed = t.path().join("managed");
    let game = t.path().join("Mods");
    fs::create_dir_all(&downloads).unwrap();
    fs::create_dir_all(&game).unwrap();
    fs::write(downloads.join("renamed.package"), b"package").unwrap();
    let p = project(provider(b"package", "mod.package"));
    let matched = match_download(&downloads, &p);
    let meta = browser_import::import_browser_download(
        &managed,
        &game,
        &p,
        &p.releases[0],
        &matched,
        &ImportPermit::default(),
    )
    .unwrap();
    assert_eq!(
        fs::read(downloads.join("renamed.package")).unwrap(),
        b"package"
    );
    assert_eq!(fs::read(game.join("mod.package")).unwrap(), b"package");
    let attachment = meta.source_attachment.as_ref().unwrap();
    assert_eq!(attachment.attached_by, "user");
    assert_eq!(attachment.confidence, Some(100));
    assert_eq!(attachment.installed_release.as_ref().unwrap().file_id, 100);
    assert_eq!(
        managed_storage::read_managed_mod(&managed, &meta.mod_id).unwrap(),
        meta
    );
    let client =
        CurseForgeClient::new(Some("fixture"), provider(b"package", "mod.package")).unwrap();
    let report =
        plumcrate_core::mod_updates::check_mod_updates(&managed, &meta.mod_id, &client)
            .unwrap();
    assert_eq!(
        report.status,
        plumcrate_core::mod_updates::UpdateStatus::NoNewerRelease
    );

    let second = match_download(&downloads, &p);
    let again = browser_import::import_browser_download(
        &managed,
        &game,
        &p,
        &p.releases[0],
        &second,
        &ImportPermit::default(),
    )
    .unwrap();
    assert_eq!(meta.mod_id, again.mod_id);
    assert_eq!(fs::read_dir(managed.join("mods")).unwrap().count(), 1);
    assert!(
        browser_import::import_browser_download(
            &managed,
            &game,
            &p,
            &p.releases[0],
            &second,
            &ImportPermit::default()
        )
        .is_err(),
        "consumed descriptor cannot be used again"
    );
}
#[test]
fn collision_leaves_unmanaged_file_and_no_published_mod() {
    let t = tempfile::tempdir().unwrap();
    let d = t.path().join("Downloads");
    let m = t.path().join("managed");
    let g = t.path().join("Mods");
    fs::create_dir_all(&d).unwrap();
    fs::create_dir_all(&g).unwrap();
    fs::write(d.join("mod.package"), b"package").unwrap();
    fs::write(g.join("mod.package"), b"user-owned").unwrap();
    let p = project(provider(b"package", "mod.package"));
    let matched = match_download(&d, &p);
    assert!(browser_import::import_browser_download(
        &m,
        &g,
        &p,
        &p.releases[0],
        &matched,
        &ImportPermit::default()
    )
    .is_err());
    assert_eq!(fs::read(g.join("mod.package")).unwrap(), b"user-owned");
    assert_eq!(fs::read(d.join("mod.package")).unwrap(), b"package");
    assert!(fs::read_dir(m.join("mods"))
        .map(|e| e.count() == 0)
        .unwrap_or(true));
}
#[test]
fn cancellation_prevents_watch_and_publication() {
    let t = tempfile::tempdir().unwrap();
    fs::write(t.path().join("mod.package"), b"package").unwrap();
    let p = project(provider(b"package", "mod.package"));
    let matched = match_download(t.path(), &p);
    let permit = ImportPermit::default();
    assert!(permit.cancel());
    let mut watch = DownloadWatch::new(t.path(), &p.releases[0]).unwrap();
    assert!(watch.poll(&permit).is_err());
    let game = t.path().join("Mods");
    fs::create_dir_all(&game).unwrap();
    assert!(browser_import::import_browser_download(
        &t.path().join("managed"),
        &game,
        &p,
        &p.releases[0],
        &matched,
        &permit
    )
    .is_err());
    assert!(
        browser_import::import_browser_download(
            &t.path().join("managed"),
            &game,
            &p,
            &p.releases[0],
            &matched,
            &ImportPermit::default()
        )
        .is_err(),
        "cancelled attempts must release the descriptor held by operation history"
    );
}
#[test]
fn source_path_replacement_keeps_verified_descriptor_and_mutated_bytes_reject() {
    let t = tempfile::tempdir().unwrap();
    let d = t.path().join("Downloads");
    let m = t.path().join("managed");
    let g = t.path().join("Mods");
    fs::create_dir_all(&d).unwrap();
    fs::create_dir_all(&g).unwrap();
    let path = d.join("mod.package");
    fs::write(&path, b"package").unwrap();
    let p = project(provider(b"package", "mod.package"));
    let matched = match_download(&d, &p);
    fs::rename(&path, d.join("original.package")).unwrap();
    fs::write(&path, b"attacker").unwrap();
    let meta = browser_import::import_browser_download(
        &m,
        &g,
        &p,
        &p.releases[0],
        &matched,
        &ImportPermit::default(),
    )
    .unwrap();
    assert_eq!(
        fs::read(m.join("mods").join(meta.mod_id).join("files/mod.package")).unwrap(),
        b"package"
    );
    assert_eq!(fs::read(&path).unwrap(), b"attacker");
    let matched = match_download(&d, &p);
    fs::write(d.join("original.package"), b"mutated").unwrap();
    assert!(browser_import::import_browser_download(
        &m,
        &g,
        &p,
        &p.releases[0],
        &matched,
        &ImportPermit::default()
    )
    .is_err());
}
fn zip(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        writer
            .start_file(*name, zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}
#[test]
fn zip_import_preserves_script_archive_and_rejects_no_mod_content_and_traversal() {
    for (entries, ok) in [
        (
            vec![
                ("mod.package", b"package".as_slice()),
                ("mod.ts4script", b"script".as_slice()),
            ],
            true,
        ),
        (vec![("readme.txt", b"text".as_slice())], false),
        (vec![("../escaped.package", b"package".as_slice())], false),
    ] {
        let t = tempfile::tempdir().unwrap();
        let d = t.path().join("Downloads");
        let m = t.path().join("managed");
        let g = t.path().join("Mods");
        fs::create_dir_all(&d).unwrap();
        fs::create_dir_all(&g).unwrap();
        let bytes = zip(&entries);
        fs::write(d.join("mod.zip"), &bytes).unwrap();
        let p = project(provider(&bytes, "mod.zip"));
        let matched = match_download(&d, &p);
        let result = browser_import::import_browser_download(
            &m,
            &g,
            &p,
            &p.releases[0],
            &matched,
            &ImportPermit::default(),
        );
        assert_eq!(result.is_ok(), ok);
        assert_eq!(fs::read(d.join("mod.zip")).unwrap(), bytes);
        if ok {
            assert_eq!(fs::read(g.join("mod.ts4script")).unwrap(), b"script");
        }
    }
}
#[test]
fn every_published_hash_must_match_and_invalid_duplicate_ids_are_not_selectable() {
    let mut p = provider(b"package", "mod.package");
    p.files[0]["hashes"][1]["value"] = json!("00000000000000000000000000000000");
    let p = project(p);
    let t = tempfile::tempdir().unwrap();
    fs::write(t.path().join("mod.package"), b"package").unwrap();
    let mut watch = DownloadWatch::new(t.path(), &p.releases[0]).unwrap();
    for _ in 0..3 {
        assert!(watch.poll(&ImportPermit::default()).unwrap().is_none());
    }
    let mut p = provider(b"package", "mod.package");
    let mut duplicate = p.files[0].clone();
    duplicate["hashes"] = json!([]);
    p.files.push(duplicate);
    let p = project(p);
    assert!(
        p.releases.is_empty(),
        "a repeated ID with incomplete metadata must not leave an earlier version selectable"
    );
    assert!(!p.complete);
}
#[test]
fn identical_release_reenables_without_duplicate_and_changed_baseline_is_preserved() {
    let t = tempfile::tempdir().unwrap();
    let d = t.path().join("Downloads");
    let m = t.path().join("managed");
    let g = t.path().join("Mods");
    fs::create_dir_all(&d).unwrap();
    fs::create_dir_all(&g).unwrap();
    fs::write(d.join("mod.package"), b"package").unwrap();
    let p = project(provider(b"package", "mod.package"));
    let meta = browser_import::import_browser_download(
        &m,
        &g,
        &p,
        &p.releases[0],
        &match_download(&d, &p),
        &ImportPermit::default(),
    )
    .unwrap();
    plumcrate_core::toggle::apply_toggle(&m, &g, &meta.mod_id, false).unwrap();
    assert!(!g.join("mod.package").exists());
    let again = browser_import::import_browser_download(
        &m,
        &g,
        &p,
        &p.releases[0],
        &match_download(&d, &p),
        &ImportPermit::default(),
    )
    .unwrap();
    assert_eq!(again.mod_id, meta.mod_id);
    assert!(g.join("mod.package").is_file());
    fs::write(
        m.join("mods").join(&meta.mod_id).join("files/mod.package"),
        b"user edit",
    )
    .unwrap();
    assert!(browser_import::import_browser_download(
        &m,
        &g,
        &p,
        &p.releases[0],
        &match_download(&d, &p),
        &ImportPermit::default()
    )
    .is_err());
    assert_eq!(fs::read(g.join("mod.package")).unwrap(), b"user edit");
    assert_eq!(fs::read_dir(m.join("mods")).unwrap().count(), 1);
}
#[test]
fn publication_gate_finishes_after_late_cancel_and_raw_script_remains_a_script() {
    let t = tempfile::tempdir().unwrap();
    let d = t.path().join("Downloads");
    let m = t.path().join("managed");
    let g = t.path().join("Mods");
    fs::create_dir_all(&d).unwrap();
    fs::create_dir_all(&g).unwrap();
    fs::write(d.join("script.ts4script"), b"script archive bytes").unwrap();
    let p = project(provider(b"script archive bytes", "script.ts4script"));
    let permit = ImportPermit::default();
    let meta = browser_import::import_browser_download(
        &m,
        &g,
        &p,
        &p.releases[0],
        &match_download(&d, &p),
        &permit,
    )
    .unwrap();
    assert!(permit.is_publishing());
    assert!(!permit.cancel());
    assert!(!permit.is_cancelled());
    assert_eq!(
        fs::read(g.join("script.ts4script")).unwrap(),
        b"script archive bytes"
    );
    assert_eq!(meta.files, vec!["script.ts4script"]);
}
#[test]
fn large_managed_library_does_not_inherit_the_download_watch_entry_limit() {
    let t = tempfile::tempdir().unwrap();
    let downloads = t.path().join("Downloads");
    let managed = t.path().join("managed");
    let game = t.path().join("Mods");
    fs::create_dir_all(&downloads).unwrap();
    fs::create_dir_all(&game).unwrap();
    for id in 0..1025 {
        fs::create_dir_all(managed.join("mods").join(format!("existing-{id}"))).unwrap();
    }
    fs::write(downloads.join("mod.package"), b"package").unwrap();
    let p = project(provider(b"package", "mod.package"));
    assert!(browser_import::import_browser_download(
        &managed,
        &game,
        &p,
        &p.releases[0],
        &match_download(&downloads, &p),
        &ImportPermit::default()
    )
    .is_ok());
}
