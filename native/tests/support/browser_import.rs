use ts4_mod_manager_core::{
    browser_import::{BrowserProject, load_browser_project},
    curseforge_client::{
        CurseForgeClient, CurseForgeRequest, CurseForgeTransport, SourceLookupError,
    },
};
use ts4_mod_manager_native::{catalog::Catalog, controls::Controls, settings::Settings, ui::View};
pub const BYTES: &[u8] = b"synthetic browser package";
pub const KEY: &str = "fixture-only";
struct Provider;
impl CurseForgeTransport for Provider {
    fn get(&self, request: &CurseForgeRequest) -> Result<(u16, String), SourceLookupError> {
        let body = if request.url.ends_with("/42") {
            serde_json::json!({"data":{"id":42,"gameId":78062,"name":"Synthetic browser collection","authors":[{"name":"Synthetic creator"}],"links":{"websiteUrl":"https://www.curseforge.com/sims4/mods/synthetic-fixture"},"allowModDistribution":false}})
        } else {
            serde_json::json!({"data":[{"id":100,"modId":42,"fileName":"fixture.package","displayName":"Synthetic release 1.0","fileDate":"2026-10-01T00:00:00Z","releaseType":1,"fileStatus":4,"isAvailable":true,"gameVersions":["1.128.90"],"fileLength":BYTES.len(),"hashes":[{"algo":1,"value":"d09fd66eb35c050ebfbe72a191db3cc8cbc42d91"}]}],"pagination":{"index":0,"pageSize":50,"resultCount":1,"totalCount":1}})
        };
        Ok((200, body.to_string()))
    }
}
pub struct Fixture {
    pub directory: tempfile::TempDir,
    pub controls: Controls,
    pub catalog: Catalog,
    pub view: View,
    pub project: BrowserProject,
}
pub fn fixture() -> Fixture {
    fixture_with_wake(|| {})
}
pub fn fixture_with_wake(wake: impl Fn() + Send + Sync + 'static) -> Fixture {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("game");
    std::fs::create_dir_all(root.join("Mods")).unwrap();
    let downloads = directory.path().join("Downloads");
    std::fs::create_dir_all(&downloads).unwrap();
    let mut catalog = Catalog::default();
    let generation = catalog.begin_scan(root.clone());
    catalog.accept_scan(generation, Ok(vec![]));
    let mut controls = Controls::new(
        directory.path().join("managed"),
        directory.path().into(),
        Settings {
            curseforge_api_key: Some(KEY.into()),
            ..Default::default()
        },
        wake,
    )
    .unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while controls.busy() {
        controls.poll(&catalog);
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let project =
        load_browser_project(&CurseForgeClient::new(Some(KEY), Provider).unwrap(), 42).unwrap();
    controls
        .browser_import
        .prepare_loaded(root, KEY, project.clone());
    controls.browser_import.download_dir = downloads;
    Fixture {
        directory,
        controls,
        catalog,
        view: View::default(),
        project,
    }
}
