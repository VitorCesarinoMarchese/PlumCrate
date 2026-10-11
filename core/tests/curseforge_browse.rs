use std::cell::RefCell;
use ts4_mod_manager_core::{
    curseforge_browse::{
        parse_browse_response, parse_categories_response, BrowseQuery, BrowseSort,
    },
    curseforge_client::{
        CurseForgeClient, CurseForgeRequest, CurseForgeTransport, SourceLookupError,
    },
};

struct Transport {
    requests: RefCell<Vec<String>>,
    status: u16,
    body: String,
}
impl CurseForgeTransport for &Transport {
    fn get(&self, request: &CurseForgeRequest) -> Result<(u16, String), SourceLookupError> {
        assert_eq!(request.api_key, "fixture-only");
        self.requests.borrow_mut().push(request.url.clone());
        Ok((self.status, self.body.clone()))
    }
}

#[test]
fn browse_uses_sims4_encoded_search_sort_category_and_page() {
    let transport = Transport {
        requests: RefCell::default(),
        status: 200,
        body:
            r#"{"data":[],"pagination":{"index":24,"pageSize":24,"resultCount":0,"totalCount":24}}"#
                .into(),
    };
    let client = CurseForgeClient::new(Some("fixture-only"), &transport).unwrap();
    let query = BrowseQuery {
        search: " café & hair ".into(),
        sort: BrowseSort::Updated,
        category: Some(42),
        index: 24,
    };
    assert!(client.browse_mods(&query).unwrap().projects.is_empty());
    let urls = transport.requests.borrow();
    assert_eq!(urls[0], "https://api.curseforge.com/v1/mods/search?gameId=78062&searchFilter=caf%C3%A9+%26+hair&sortField=3&sortOrder=desc&index=24&pageSize=24&categoryId=42");
}

#[test]
fn browse_preserves_provider_metadata_and_pagination_without_inventing_links() {
    let page = parse_browse_response(r#"{"data":[
      {"id":1,"gameId":78062,"name":"Fixture hair","summary":"A synthetic test project.","authors":[{"name":"Fixture creator"}],"downloadCount":1200,"dateModified":"2026-10-01T10:00:00Z","categories":[{"id":42,"name":"Hair"}],"screenshots":[{"url":"https://media.forgecdn.net/fixture.png"}]},
      {"id":2,"gameId":432,"name":"Wrong game"}, {"id":3,"gameId":78062,"name":" "}
    ],"pagination":{"index":0,"pageSize":24,"resultCount":3,"totalCount":30}}"#).unwrap();
    assert_eq!(page.projects.len(), 1);
    let project = &page.projects[0];
    assert_eq!(project.name, "Fixture hair");
    assert_eq!(project.authors, ["Fixture creator"]);
    assert_eq!(project.summary, "A synthetic test project.");
    assert_eq!(project.downloads, 1200.0);
    assert_eq!(project.categories[0].name, "Hair");
    assert!(project.source_url.is_none());
    assert!(project.preview_url.is_none());
    assert_eq!(project.screenshots.len(), 1);
    assert_eq!(page.next_index(), Some(3));
}

#[test]
fn empty_and_invalid_provider_results_stay_distinct() {
    let empty = parse_browse_response(
        r#"{"data":[],"pagination":{"index":0,"pageSize":24,"resultCount":0,"totalCount":0}}"#,
    )
    .unwrap();
    assert!(empty.projects.is_empty());
    assert_eq!(empty.next_index(), None);
    assert_eq!(
        parse_browse_response("{}"),
        Err(SourceLookupError::InvalidResponse)
    );
    assert_eq!(parse_browse_response(r#"{"data":[],"pagination":{"index":9999,"pageSize":24,"resultCount":24,"totalCount":20000}}"#).unwrap().next_index(), None);
    let categories = parse_categories_response(r#"{"data":[{"id":42,"gameId":78062,"name":"Hair"},{"id":12,"gameId":432,"name":"Wrong game"}]}"#).unwrap();
    assert_eq!(categories.len(), 1);
    assert_eq!(categories[0].id, 42);
}

#[test]
fn provider_errors_do_not_become_empty_search_results() {
    for (status, expected) in [
        (403, SourceLookupError::Unauthorized),
        (429, SourceLookupError::RateLimited),
        (503, SourceLookupError::ProviderUnavailable),
    ] {
        let transport = Transport {
            requests: RefCell::default(),
            status,
            body: "{}".into(),
        };
        let client = CurseForgeClient::new(Some("fixture-only"), &transport).unwrap();
        assert_eq!(client.browse_mods(&BrowseQuery::default()), Err(expected));
    }
}
