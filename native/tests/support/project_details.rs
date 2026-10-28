use serde_json::json;
use plumcrate_core::{
    browser_import::{BrowserProject, load_browser_project},
    curseforge_browse::{BrowseQuery, parse_browse_response},
    curseforge_client::{
        CurseForgeClient, CurseForgeRequest, CurseForgeTransport, SourceLookupError,
    },
    curseforge_details::{ProjectReference, SafeUrl, parse_description_response},
};
use plumcrate::discovery::DiscoverView;
pub struct Provider;
impl CurseForgeTransport for Provider {
    fn get(&self, request: &CurseForgeRequest) -> Result<(u16, String), SourceLookupError> {
        let body = if request.url.ends_with("/9") {
            json!({"data":{"id":9,"gameId":78062,"name":"Synthetic Core Library","authors":[{"name":"Fixture creator"}],"links":{"websiteUrl":"https://www.curseforge.com/sims4/mods/synthetic-core"}}})
        } else if request.url.contains("/9/files?") {
            json!({"data":[{"id":200,"modId":9,"fileName":"core-library.package","displayName":"Core Library 1.0","fileDate":"2026-10-10T00:00:00Z","releaseType":1,"isAvailable":true,"fileStatus":4,"gameVersions":["1.128"],"fileLength":25,"hashes":[{"algo":1,"value":"d09fd66eb35c050ebfbe72a191db3cc8cbc42d91"}],"dependencies":[]}],"pagination":{"index":0,"pageSize":50,"resultCount":1,"totalCount":1}})
        } else if request.url.ends_with("/42") {
            json!({"data":{"id":42,"gameId":78062,"name":"Synthetic garden interactions","authors":[{"name":"Fixture creator"}],"links":{"websiteUrl":"https://www.curseforge.com/sims4/mods/synthetic-garden"}}})
        } else {
            json!({"data":[
                {"id":100,"modId":42,"fileName":"garden.package","displayName":"Garden interactions 2.0","fileDate":"2026-10-10T00:00:00Z","releaseType":1,"isAvailable":true,"fileStatus":4,"gameVersions":["1.128"],"fileLength":25,"hashes":[{"algo":1,"value":"d09fd66eb35c050ebfbe72a191db3cc8cbc42d91"}],"dependencies":[{"modId":9,"relationType":3},{"modId":10,"relationType":5}]},
                {"id":101,"modId":42,"fileName":"garden-old.package","displayName":"Garden interactions 1.0","fileDate":"2026-10-01T00:00:00Z","releaseType":1,"isAvailable":true,"fileStatus":4,"gameVersions":["1.127"],"fileLength":25,"hashes":[{"algo":1,"value":"d09fd66eb35c050ebfbe72a191db3cc8cbc42d91"}],"dependencies":[]}
            ],"pagination":{"index":0,"pageSize":50,"resultCount":2,"totalCount":2}})
        };
        Ok((200, body.to_string()))
    }
}
pub fn project() -> BrowserProject {
    load_browser_project(
        &CurseForgeClient::new(Some("synthetic"), Provider).unwrap(),
        42,
    )
    .unwrap()
}
pub fn seed(ctx: &egui::Context, view: &mut DiscoverView) -> u64 {
    let page=parse_browse_response(&json!({"data":[{"id":42,"gameId":78062,"name":"Synthetic garden interactions","summary":"More ways to care for your Sims' garden.","authors":[{"name":"Fixture creator"}],"links":{"websiteUrl":"https://www.curseforge.com/sims4/mods/synthetic-garden"},"downloadCount":124000,"dateModified":"2026-10-10T00:00:00Z"}],"pagination":{"index":0,"pageSize":24,"resultCount":1,"totalCount":1}}).to_string()).unwrap();
    let generation = view.browser.begin(BrowseQuery::default());
    view.browser.accept(generation, Ok(page.clone()));
    view.selected = Some(page.projects[0].clone());
    let token = view.details.begin(42, None);
    view.details.accept_project(token, Ok(project()));
    let description=parse_description_response(&json!({"data":"<h2>A livelier garden</h2><p>Synthetic fixture content. Sims can tend seedlings, share gardening tips, and harvest together. The full description explains installation and requirements.</p><h3>Installation</h3><ol><li>Close The Sims 4.</li><li>Install the required core library separately, then install this mod.</li><li>Enable script mods in Game Options and restart the game.</li></ol><h3>Requirements</h3><p>These instructions also mention an author-linked add-on: <a href='https://www.curseforge.com/sims4/mods/synthetic-addon'>Optional garden add-on</a>. This link is not a declared dependency.</p><h3>Support the creator</h3><p><a href='https://example.org/support'>Author support page</a></p>"}).to_string(),"https://www.curseforge.com/sims4/mods/synthetic-garden").unwrap();
    view.details.accept_description(token, Ok(description));
    view.details.sync(ctx, view.selected.as_ref(), None);
    for (id, title, slug) in [
        (9, "Synthetic Core Library", "synthetic-core"),
        (10, "Older garden overhaul", "synthetic-overhaul"),
    ] {
        view.details.accept_reference(
            token,
            100,
            id,
            Ok(ProjectReference {
                project_id: id,
                title: title.into(),
                source_url: SafeUrl::parse(&format!(
                    "https://www.curseforge.com/sims4/mods/{slug}"
                ))
                .unwrap(),
            }),
        );
    }
    token
}
