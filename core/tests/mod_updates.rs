use plumcrate_core::managed_storage::SourceAttachmentMetadata;

use serde_json::{json, Value};
use std::{collections::HashMap, fs, path::PathBuf};
use plumcrate_core::{
    curseforge_client::{
        CurseForgeClient, CurseForgeRequest, CurseForgeTransport, SourceLookupError,
    },
    managed_storage::{self, ImportRequest},
    mod_updates::{check_mod_updates, confirm_installed_release, UpdateStatus},
};

struct Provider {
    pages: Vec<Value>,
    exact: HashMap<u64, Value>,
    project: Value,
    status: u16,
}
impl Provider {
    fn new(files: Vec<Value>) -> Self {
        let exact = files
            .iter()
            .map(|f| (f["id"].as_u64().unwrap(), f.clone()))
            .collect();
        let count = files.len();
        Self {
            pages: vec![
                json!({"data": files, "pagination": {"index": 0, "pageSize": 50, "resultCount": count, "totalCount": count}}),
            ],
            exact,
            project: json!({"data":{"id":42,"gameId":78062,"links":{"websiteUrl":"https://www.curseforge.com/sims4/mods/fixture"}}}),
            status: 200,
        }
    }
    fn client(self) -> CurseForgeClient<Self> {
        CurseForgeClient::new(Some("fixture"), self).unwrap()
    }
}
impl CurseForgeTransport for Provider {
    fn get(&self, request: &CurseForgeRequest) -> Result<(u16, String), SourceLookupError> {
        assert!(request
            .url
            .starts_with("https://api.curseforge.com/v1/mods/42"));
        let suffix = request
            .url
            .strip_prefix("https://api.curseforge.com/v1/mods/42")
            .unwrap();
        let value = if suffix.is_empty() {
            self.project.clone()
        } else if let Some(id) = suffix.strip_prefix("/files/") {
            let Some(file) = self.exact.get(&id.parse().unwrap()) else {
                return Ok((404, "{}".into()));
            };
            json!({"data":file})
        } else {
            assert!(suffix.starts_with("/files?"));
            let index: usize = suffix
                .split('&')
                .find_map(|p| p.strip_prefix("index="))
                .unwrap()
                .parse()
                .unwrap();
            self.pages
                .get(index / 50)
                .cloned()
                .unwrap_or_else(|| json!({}))
        };
        Ok((self.status, value.to_string()))
    }
}
fn file(id: u64, date: &str) -> Value {
    json!({"id":id,"modId":42,"displayName":format!("Release {id}"),"fileName":format!("mod-{id}.zip"),"fileDate":date,"releaseType":1,"isAvailable":true,"fileStatus":4,"isEarlyAccessContent":false,"gameVersions":["1.0"]})
}
fn fixture() -> (tempfile::TempDir, PathBuf, String) {
    let tmp = tempfile::tempdir().unwrap();
    let source = tmp.path().join("source");
    let root = tmp.path().join("managed");
    fs::create_dir_all(&source).unwrap();
    fs::write(source.join("a.package"), b"original").unwrap();
    let meta = managed_storage::create_managed_mod(
        &root,
        ImportRequest {
            name: "Fixture".into(),
            slug: None,
            source_dir: source,
        },
    )
    .unwrap();
    let attachment: SourceAttachmentMetadata = serde_json::from_value(json!({"providerId":"curseforge","projectId":42,"fileId":900,"sourceUrl":"https://www.curseforge.com/sims4/mods/fixture","title":"Fixture","attachedBy":"user","attachedAt":"2026-10-01T00:00:00Z"})).unwrap();
    managed_storage::set_source_url(
        &root,
        &meta.mod_id,
        attachment.source_url.clone(),
        Some("curseforge".into()),
        None,
        None,
        Some(attachment),
    )
    .unwrap();
    (tmp, root, meta.mod_id)
}
fn baseline(root: &std::path::Path, id: &str) {
    let report = check_mod_updates(
        root,
        id,
        &Provider::new(vec![file(100, "2026-01-01T00:00:00Z")]).client(),
    )
    .unwrap();
    confirm_installed_release(root, id, &report, &report.releases[0]).unwrap();
}

#[test]
fn legacy_id_is_unknown_and_checks_are_read_only() {
    let (_tmp, root, id) = fixture();
    let before = fs::read(root.join("mods").join(&id).join("meta.json")).unwrap();
    let report = check_mod_updates(
        &root,
        &id,
        &Provider::new(vec![file(900, "2026-01-01T00:00:00Z")]).client(),
    )
    .unwrap();
    assert_eq!(report.status, UpdateStatus::Untracked);
    assert!(report.installed.is_none());
    assert!(report.complete);
    assert_eq!(
        before,
        fs::read(root.join("mods").join(id).join("meta.json")).unwrap()
    );
}

#[test]
fn dates_not_numeric_ids_determine_newer_releases_and_ties_do_not() {
    let (_tmp, root, id) = fixture();
    baseline(&root, &id);
    let report = check_mod_updates(
        &root,
        &id,
        &Provider::new(vec![
            file(100, "2026-01-01T00:00:00Z"),
            file(2, "2026-01-02T01:00:00+01:00"),
            file(900, "2025-12-01T00:00:00Z"),
        ])
        .client(),
    )
    .unwrap();
    assert!(matches!(report.status,UpdateStatus::Available(ref release) if release.file_id==2));
    assert_eq!(report.installed.unwrap().file_id, 100);
    let tied = check_mod_updates(
        &root,
        &id,
        &Provider::new(vec![
            file(100, "2026-01-01T00:00:00Z"),
            file(900, "2026-01-01T00:00:00Z"),
        ])
        .client(),
    )
    .unwrap();
    assert_eq!(tied.status, UpdateStatus::NoNewerRelease);
}

#[test]
fn installed_file_is_fetched_separately_and_missing_identity_is_unknown() {
    let (_tmp, root, id) = fixture();
    baseline(&root, &id);
    let mut provider = Provider::new(vec![file(2, "2025-01-01T00:00:00Z")]);
    provider
        .exact
        .insert(100, file(100, "2026-01-01T00:00:00Z"));
    let report = check_mod_updates(&root, &id, &provider.client()).unwrap();
    assert_eq!(report.status, UpdateStatus::NoNewerRelease);
    assert_eq!(report.installed.unwrap().file_id, 100);
    let report = check_mod_updates(&root, &id, &Provider::new(vec![]).client()).unwrap();
    assert_eq!(report.status, UpdateStatus::InstalledReleaseUnavailable);
}

#[test]
fn nonpublic_beta_early_access_files_are_not_release_choices() {
    let (_tmp, root, id) = fixture();
    let mut beta = file(2, "2026-01-02T00:00:00Z");
    beta["releaseType"] = json!(2);
    let mut unavailable = file(3, "2026-01-03T00:00:00Z");
    unavailable["isAvailable"] = json!(false);
    let mut early = file(4, "2026-01-04T00:00:00Z");
    early["isEarlyAccessContent"] = json!(true);
    let mut pending = file(5, "2026-01-05T00:00:00Z");
    pending["fileStatus"] = json!(1);
    let report = check_mod_updates(
        &root,
        &id,
        &Provider::new(vec![
            file(1, "2026-01-01T00:00:00Z"),
            beta,
            unavailable,
            early,
            pending,
        ])
        .client(),
    )
    .unwrap();
    assert_eq!(report.releases.len(), 1);
}

#[test]
fn incomplete_listing_can_prove_available_but_never_no_newer_release() {
    let (_tmp, root, id) = fixture();
    baseline(&root, &id);
    let mut missing_date = file(3, "2026-01-03T00:00:00Z");
    missing_date["fileDate"] = Value::Null;
    let report = check_mod_updates(
        &root,
        &id,
        &Provider::new(vec![
            file(100, "2026-01-01T00:00:00Z"),
            missing_date.clone(),
        ])
        .client(),
    )
    .unwrap();
    assert_eq!(report.status, UpdateStatus::Incomplete);
    assert!(!report.complete);
    let report = check_mod_updates(
        &root,
        &id,
        &Provider::new(vec![
            file(100, "2026-01-01T00:00:00Z"),
            file(2, "2026-01-02T00:00:00Z"),
            missing_date,
        ])
        .client(),
    )
    .unwrap();
    assert!(matches!(report.status, UpdateStatus::Available(_)));
    assert!(!report.complete);
}

#[test]
fn local_changes_additions_and_removals_invalidate_confirmation() {
    let (_tmp, root, id) = fixture();
    baseline(&root, &id);
    let dir = root.join("mods").join(&id).join("files");
    let provider = Provider::new(vec![file(100, "2026-01-01T00:00:00Z")]).client();
    let report = check_mod_updates(&root, &id, &provider).unwrap();
    fs::write(dir.join("extra.package"), b"added").unwrap();
    assert!(confirm_installed_release(&root, &id, &report, &report.releases[0]).is_err());
    assert_eq!(
        check_mod_updates(&root, &id, &provider).unwrap().status,
        UpdateStatus::LocalContentChanged
    );
    fs::remove_file(dir.join("extra.package")).unwrap();
    fs::write(dir.join("a.package"), b"changed").unwrap();
    assert_eq!(
        check_mod_updates(&root, &id, &provider).unwrap().status,
        UpdateStatus::LocalContentChanged
    );
    fs::remove_file(dir.join("a.package")).unwrap();
    assert_eq!(
        check_mod_updates(&root, &id, &provider).unwrap().status,
        UpdateStatus::LocalContentChanged
    );
}

#[test]
fn source_change_rejects_stale_confirmation_and_clears_saved_baseline() {
    let (_tmp, root, id) = fixture();
    baseline(&root, &id);
    let report = check_mod_updates(
        &root,
        &id,
        &Provider::new(vec![file(100, "2026-01-01T00:00:00Z")]).client(),
    )
    .unwrap();
    let mut attachment = managed_storage::read_managed_mod(&root, &id)
        .unwrap()
        .source_attachment
        .unwrap();
    attachment.source_url = "https://www.curseforge.com/sims4/mods/replacement".into();
    let changed = managed_storage::set_source_url(
        &root,
        &id,
        attachment.source_url.clone(),
        Some("curseforge".into()),
        None,
        None,
        Some(attachment),
    )
    .unwrap();
    assert!(changed
        .source_attachment
        .unwrap()
        .installed_release
        .is_none());
    assert!(confirm_installed_release(&root, &id, &report, &report.releases[0]).is_err());
}

#[test]
fn only_provider_sims4_project_https_pages_are_usable() {
    let (_tmp, root, id) = fixture();
    for project in [
        json!({"data":{"id":42,"gameId":1,"links":{"websiteUrl":"https://www.curseforge.com/sims4/mods/fixture"}}}),
        json!({"data":{"id":42,"gameId":78062,"links":{"websiteUrl":"https://evil.example/sims4/mods/fixture"}}}),
    ] {
        let mut provider = Provider::new(vec![]);
        provider.project = project;
        assert!(check_mod_updates(&root, &id, &provider.client()).is_err());
    }
    let mut provider = Provider::new(vec![]);
    provider.status = 429;
    assert_eq!(
        check_mod_updates(&root, &id, &provider.client())
            .unwrap_err()
            .code
            .as_str(),
        "SOURCE_RATE_LIMITED"
    );
}

#[cfg(unix)]
#[test]
fn symlinks_and_special_files_inside_managed_content_are_rejected() {
    let (tmp, root, id) = fixture();
    std::os::unix::fs::symlink(
        tmp.path().join("source/a.package"),
        root.join("mods").join(&id).join("files/linked.package"),
    )
    .unwrap();
    assert!(check_mod_updates(&root, &id, &Provider::new(vec![]).client()).is_err());
}

#[test]
fn duplicate_wrong_project_and_inconsistent_pagination_do_not_claim_complete() {
    let (_tmp, root, id) = fixture();
    baseline(&root, &id);
    let mut wrong = file(2, "2025-01-01T00:00:00Z");
    wrong["modId"] = json!(99);
    for records in [
        vec![
            file(100, "2026-01-01T00:00:00Z"),
            file(100, "2026-01-01T00:00:00Z"),
        ],
        vec![file(100, "2026-01-01T00:00:00Z"), wrong],
    ] {
        let report = check_mod_updates(&root, &id, &Provider::new(records).client()).unwrap();
        assert_eq!(report.status, UpdateStatus::Incomplete);
        assert!(!report.complete);
    }
    let mut provider = Provider::new(vec![file(100, "2026-01-01T00:00:00Z")]);
    provider.pages[0]["pagination"]["totalCount"] = json!(0);
    assert!(
        !check_mod_updates(&root, &id, &provider.client())
            .unwrap()
            .complete
    );
}

fn paged_provider(count: usize) -> Provider {
    let mut provider = Provider::new(vec![]);
    provider
        .exact
        .insert(100, file(100, "2026-01-01T00:00:00Z"));
    provider.pages=(0..count).collect::<Vec<_>>().chunks(50).enumerate().map(|(page, ids)| {
        let records:Vec<_>=ids.iter().map(|id|file(*id as u64+1000,"2025-01-01T00:00:00Z")).collect();
        json!({"data":records,"pagination":{"index":page*50,"pageSize":50,"resultCount":ids.len(),"totalCount":count}})
    }).collect();
    provider
}

#[test]
fn follows_indexed_pages_and_marks_bounded_history_incomplete() {
    let (_tmp, root, id) = fixture();
    baseline(&root, &id);
    let report = check_mod_updates(&root, &id, &paged_provider(51).client()).unwrap();
    assert!(report.complete);
    assert_eq!(report.releases.len(), 51);
    assert_eq!(report.status, UpdateStatus::NoNewerRelease);
    let report = check_mod_updates(&root, &id, &paged_provider(501).client()).unwrap();
    assert!(!report.complete);
    assert_eq!(report.releases.len(), 500);
    assert_eq!(report.status, UpdateStatus::Incomplete);
    let mut repeated = paged_provider(51);
    repeated.pages[1] = repeated.pages[0].clone();
    assert!(
        !check_mod_updates(&root, &id, &repeated.client())
            .unwrap()
            .complete
    );
}

#[test]
fn confirming_twice_is_idempotent_and_never_replaces_attachment_suggestion() {
    let (_tmp, root, id) = fixture();
    let report = check_mod_updates(
        &root,
        &id,
        &Provider::new(vec![file(100, "2026-01-01T00:00:00Z")]).client(),
    )
    .unwrap();
    let first = confirm_installed_release(&root, &id, &report, &report.releases[0]).unwrap();
    let raw = fs::read(root.join("mods").join(&id).join("meta.json")).unwrap();
    let second = confirm_installed_release(&root, &id, &report, &report.releases[0]).unwrap();
    assert_eq!(first.source_attachment, second.source_attachment);
    assert_eq!(
        raw,
        fs::read(root.join("mods").join(&id).join("meta.json")).unwrap()
    );
    let source = second.source_attachment.unwrap();
    assert_eq!(source.file_id, Some(900));
    assert_eq!(source.installed_release.unwrap().file_id, 100);
    let mut invented = report.releases[0].clone();
    invented.file_id = 999;
    assert!(confirm_installed_release(&root, &id, &report, &invented).is_err());
}

#[cfg(unix)]
#[test]
fn fifo_is_rejected_without_blocking_and_enablement_links_do_not_change_digest() {
    let (_tmp, root, id) = fixture();
    baseline(&root, &id);
    let provider = Provider::new(vec![file(100, "2026-01-01T00:00:00Z")]).client();
    let dir = root.join("mods").join(&id);
    std::os::unix::fs::symlink("/absent", dir.join("links/enabled.package")).unwrap();
    assert_eq!(
        check_mod_updates(&root, &id, &provider).unwrap().status,
        UpdateStatus::NoNewerRelease
    );
    use std::os::unix::ffi::OsStrExt;
    let path =
        std::ffi::CString::new(dir.join("files/fifo.package").as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(path.as_ptr(), 0o600) }, 0);
    assert!(check_mod_updates(&root, &id, &provider).is_err());
}

struct ChangeContentDuringCheck {
    provider: Provider,
    path: PathBuf,
}
impl CurseForgeTransport for ChangeContentDuringCheck {
    fn get(&self, request: &CurseForgeRequest) -> Result<(u16, String), SourceLookupError> {
        if request.url.ends_with("/mods/42") {
            fs::write(&self.path, b"replacement").unwrap();
        }
        self.provider.get(request)
    }
}
#[test]
fn content_changed_during_provider_check_cannot_keep_no_newer_release_status() {
    let (_tmp, root, id) = fixture();
    baseline(&root, &id);
    let transport = ChangeContentDuringCheck {
        provider: Provider::new(vec![file(100, "2026-01-01T00:00:00Z")]),
        path: root.join("mods").join(&id).join("files/a.package"),
    };
    let client = CurseForgeClient::new(Some("fixture"), transport).unwrap();
    assert_eq!(
        check_mod_updates(&root, &id, &client).unwrap().status,
        UpdateStatus::LocalContentChanged
    );
}

#[test]
fn contradictory_duplicate_and_installed_ids_cannot_prove_available() {
    let (_tmp, root, id) = fixture();
    baseline(&root, &id);
    let report = check_mod_updates(
        &root,
        &id,
        &Provider::new(vec![
            file(100, "2026-01-01T00:00:00Z"),
            file(2, "2026-02-01T00:00:00Z"),
            file(2, "2025-01-01T00:00:00Z"),
        ])
        .client(),
    )
    .unwrap();
    assert_eq!(report.status, UpdateStatus::Incomplete);
    let mut provider = Provider::new(vec![file(100, "2026-02-01T00:00:00Z")]);
    provider
        .exact
        .insert(100, file(100, "2026-01-01T00:00:00Z"));
    assert_eq!(
        check_mod_updates(&root, &id, &provider.client())
            .unwrap()
            .status,
        UpdateStatus::Incomplete
    );
}

struct MalformedExact(Provider);
impl CurseForgeTransport for MalformedExact {
    fn get(&self, request: &CurseForgeRequest) -> Result<(u16, String), SourceLookupError> {
        if request.url.ends_with("/files/100") {
            return Ok((200, "{broken".into()));
        }
        self.0.get(request)
    }
}
#[test]
fn unusable_exact_installed_file_is_unknown_instead_of_a_comparison_failure() {
    let (_tmp, root, id) = fixture();
    baseline(&root, &id);
    let client =
        CurseForgeClient::new(Some("fixture"), MalformedExact(Provider::new(vec![]))).unwrap();
    assert_eq!(
        check_mod_updates(&root, &id, &client).unwrap().status,
        UpdateStatus::InstalledReleaseUnavailable
    );
}

#[test]
fn confirmed_installed_release_survives_metadata_roundtrip_without_promoting_legacy_file_id() {
    let raw = serde_json::json!({
        "providerId": "curseforge", "projectId": 42, "fileId": 900,
        "sourceUrl": "https://www.curseforge.com/sims4/mods/fixture",
        "title": "Fixture", "attachedBy": "user", "attachedAt": "2026-10-01T00:00:00Z",
        "installedRelease": {"projectId": 42, "fileId": 100, "contentDigest": "abc", "confirmedAt": "2026-10-02T00:00:00Z"}
    });
    let attachment: SourceAttachmentMetadata = serde_json::from_value(raw.clone()).unwrap();
    let saved = serde_json::to_value(attachment).unwrap();
    assert_eq!(saved["installedRelease"], raw["installedRelease"]);
    assert_eq!(saved["fileId"], 900);
    let mut legacy = raw;
    legacy.as_object_mut().unwrap().remove("installedRelease");
    let old: SourceAttachmentMetadata = serde_json::from_value(legacy).unwrap();
    assert!(serde_json::to_value(old)
        .unwrap()
        .get("installedRelease")
        .is_none());
}
