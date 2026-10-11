use std::cell::RefCell;
use ts4_mod_manager_core::{
    curseforge_browse::{
        category_options, parse_browse_response, parse_categories_response,
        parse_game_versions_response, BrowseQuery, BrowseSort,
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
        ..Default::default()
    };
    assert!(client.browse_mods(&query).unwrap().projects.is_empty());
    let urls = transport.requests.borrow();
    assert_eq!(urls[0], "https://api.curseforge.com/v1/mods/search?gameId=78062&searchFilter=caf%C3%A9+%26+hair&sortField=3&sortOrder=desc&index=24&pageSize=24&categoryId=42");
}

#[test]
fn provider_taxonomy_keeps_content_types_and_disambiguates_duplicate_categories() {
    let categories = parse_categories_response(
        r#"{"data":[
      {"id":10,"gameId":78062,"name":"Create a Sim","isClass":true},
      {"id":11,"gameId":78062,"name":"Clothing","classId":10,"parentCategoryId":10},
      {"id":12,"gameId":78062,"name":"Body","classId":10,"parentCategoryId":10},
      {"id":13,"gameId":78062,"name":"Feminine","classId":10,"parentCategoryId":11},
      {"id":14,"gameId":78062,"name":"Feminine","classId":10,"parentCategoryId":12},
      {"id":20,"gameId":78062,"name":"Build / Buy","isClass":true},
      {"id":21,"gameId":78062,"name":"Lighting","classId":20,"parentCategoryId":20}
    ]}"#,
    )
    .unwrap();
    assert_eq!(
        categories
            .iter()
            .filter(|c| c.is_class == Some(true))
            .count(),
        2
    );
    let options = category_options(&categories, 10);
    assert_eq!(
        options,
        vec![
            (12, "Body".into()),
            (14, "Body / Feminine".into()),
            (11, "Clothing".into()),
            (13, "Clothing / Feminine".into())
        ]
    );
    assert_eq!(
        category_options(&categories, 20),
        vec![(21, "Lighting".into())]
    );
    assert!(category_options(&categories, 999).is_empty());
}

#[test]
fn invalid_parent_chains_are_not_presented_as_scoped_categories() {
    let categories = parse_categories_response(
        r#"{"data":[
      {"id":10,"gameId":78062,"name":"Mods","isClass":true},
      {"id":11,"gameId":78062,"name":"Cycle one","classId":10,"parentCategoryId":12},
      {"id":12,"gameId":78062,"name":"Cycle two","classId":10,"parentCategoryId":11},
      {"id":13,"gameId":78062,"name":"Orphan","classId":10,"parentCategoryId":999}
    ]}"#,
    )
    .unwrap();
    assert!(category_options(&categories, 10).is_empty());
}

#[test]
fn browse_combines_content_type_category_and_encoded_game_version() {
    let transport = Transport {
        requests: RefCell::default(),
        status: 200,
        body:
            r#"{"data":[],"pagination":{"index":0,"pageSize":24,"resultCount":0,"totalCount":0}}"#
                .into(),
    };
    let client = CurseForgeClient::new(Some("fixture-only"), &transport).unwrap();
    let query = BrowseQuery {
        class_id: Some(10),
        category: Some(13),
        game_version: Some("1.128.90 & fixture".into()),
        ..Default::default()
    };
    client.browse_mods(&query).unwrap();
    let requests = transport.requests.borrow();
    assert!(requests[0].contains("&classId=10"));
    assert!(requests[0].contains("&categoryId=13"));
    assert!(requests[0].contains("&gameVersion=1.128.90+%26+fixture"));
    assert!(!requests[0].contains("gameVersion=1.128.90 &"));
}

#[test]
fn game_versions_are_numeric_latest_first_without_pack_names_or_duplicates() {
    let versions = parse_game_versions_response(r#"{"data":[{"type":1,"versions":["1.99.305","1.128.90","1.9.100","1.128.90"," ","bad.version"]},{"type":2,"versions":["Seasons","Castle Estate"]}]}"#).unwrap();
    assert_eq!(versions, ["1.128.90", "1.99.305", "1.9.100"]);
    assert_eq!(
        parse_game_versions_response("{}"),
        Err(SourceLookupError::InvalidResponse)
    );
    let transport = Transport {
        requests: RefCell::default(),
        status: 200,
        body: r#"{"data":[{"type":1,"versions":["1.128.90"]}]}"#.into(),
    };
    let client = CurseForgeClient::new(Some("fixture-only"), &transport).unwrap();
    assert_eq!(client.browse_game_versions().unwrap(), ["1.128.90"]);
    assert_eq!(
        transport.requests.borrow()[0],
        "https://api.curseforge.com/v1/games/78062/versions"
    );
}

#[test]
fn curseforge_sort_options_use_the_documented_fields_and_order() {
    let expected = [
        (BrowseSort::Relevance, "Relevancy", None, "desc"),
        (BrowseSort::Popular, "Popularity", Some(2), "desc"),
        (BrowseSort::Updated, "Latest update", Some(3), "desc"),
        (BrowseSort::Created, "Date released", Some(11), "desc"),
        (BrowseSort::Downloads, "Total Downloads", Some(6), "desc"),
        (BrowseSort::Name, "A-Z", Some(4), "asc"),
    ];
    for (sort, label, field, order) in expected {
        assert_eq!(sort.label(), label);
        assert_eq!(sort.field(), field);
        assert_eq!(sort.order(), order);
    }
    let transport = Transport {
        requests: RefCell::default(),
        status: 200,
        body:
            r#"{"data":[],"pagination":{"index":0,"pageSize":24,"resultCount":0,"totalCount":0}}"#
                .into(),
    };
    let client = CurseForgeClient::new(Some("fixture-only"), &transport).unwrap();
    client
        .browse_mods(&BrowseQuery {
            sort: BrowseSort::Relevance,
            ..Default::default()
        })
        .unwrap();
    assert!(!transport.requests.borrow()[0].contains("sortField="));
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
