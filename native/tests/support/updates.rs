use plumcrate_core::{
    curseforge_client::{
        CurseForgeClient, CurseForgeRequest, CurseForgeTransport, SourceLookupError,
    },
    managed_storage::{self, ImportRequest},
    mod_updates::{self, UpdateReport},
};
use plumcrate::{catalog::Catalog, controls::Controls, settings::Settings, ui::View};

struct Provider;
impl CurseForgeTransport for Provider {
    fn get(&self, request: &CurseForgeRequest) -> Result<(u16, String), SourceLookupError> {
        let file = |id, name, date| serde_json::json!({"id":id,"modId":42,"gameId":78062,"isAvailable":true,"releaseType":1,"fileStatus":4,"fileName":name,"displayName":name,"fileDate":date,"gameVersions":["1.128.90"]});
        let body = if request.url.ends_with("/mods/42") {
            serde_json::json!({"data":{"id":42,"gameId":78062,"links":{"websiteUrl":"https://www.curseforge.com/sims4/mods/synthetic-fixture"}}})
        } else {
            serde_json::json!({"data":[file(100,"Synthetic mod 1.0.zip","2026-09-01T00:00:00Z"),file(200,"Synthetic mod 2.0.zip","2026-10-01T00:00:00Z")],"pagination":{"index":0,"pageSize":50,"resultCount":2,"totalCount":2}})
        };
        Ok((200, body.to_string()))
    }
}

pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub controls: Controls,
    pub view: View,
    pub catalog: Catalog,
    pub report: UpdateReport,
}

pub fn fixture() -> Fixture {
    let directory = tempfile::tempdir().unwrap();
    let source = directory.path().join("source");
    let root = directory.path().join("game");
    let managed = directory.path().join("managed");
    std::fs::create_dir(&source).unwrap();
    std::fs::create_dir_all(root.join("Mods")).unwrap();
    std::fs::write(source.join("fixture.package"), b"synthetic fixture bytes").unwrap();
    let meta = managed_storage::create_managed_mod(
        &managed,
        ImportRequest {
            name: "Synthetic mod collection".into(),
            slug: Some("fixture".into()),
            source_dir: source,
        },
    )
    .unwrap();
    let url = "https://www.curseforge.com/sims4/mods/synthetic-fixture";
    let attachment = serde_json::from_value(serde_json::json!({"providerId":"curseforge","projectId":42,"fileId":900,"sourceUrl":url,"title":"Synthetic mod collection","confidence":95,"attachedBy":"user","attachedAt":"2026-10-01T00:00:00Z"})).unwrap();
    managed_storage::set_source_url(
        &managed,
        &meta.mod_id,
        url.into(),
        Some("curseforge".into()),
        None,
        None,
        Some(attachment),
    )
    .unwrap();
    let client = CurseForgeClient::new(Some("fixture-only"), Provider).unwrap();
    let report = mod_updates::check_mod_updates(&managed, &meta.mod_id, &client).unwrap();
    let mut catalog = Catalog::default();
    let generation = catalog.begin_scan(root.clone());
    catalog.accept_scan(
        generation,
        Ok(plumcrate::worker::scan(&root, &managed)
            .unwrap()
            .mods),
    );
    catalog.select(0);
    let mut controls =
        Controls::new(managed, directory.path().into(), Settings::default(), || {}).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while controls.busy() {
        controls.poll(&mut catalog);
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    controls.updates.sync(&catalog, None);
    let token = controls.updates.begin();
    controls
        .updates
        .accept(token, vec![(meta.mod_id, Ok(report.clone()))]);
    Fixture {
        directory,
        controls,
        view: View::default(),
        catalog,
        report,
    }
}
