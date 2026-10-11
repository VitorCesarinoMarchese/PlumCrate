use ts4_mod_manager_core::{
    curseforge_browse::{BrowseQuery, BrowseSort, parse_browse_response},
    curseforge_client::SourceLookupError,
};
use ts4_mod_manager_native::discovery::Browser;

fn page(index: u32, count: u32, total: u32) -> ts4_mod_manager_core::curseforge_browse::BrowsePage {
    parse_browse_response(&serde_json::json!({"data":[{"id":9,"gameId":78062,"name":"Synthetic project","summary":"Test summary"}],"pagination":{"index":index,"pageSize":24,"resultCount":count,"totalCount":total}}).to_string()).unwrap()
}

#[test]
fn superseded_search_cannot_replace_results_or_errors() {
    let mut browser = Browser::default();
    let a = browser.begin(BrowseQuery {
        search: "first".into(),
        ..Default::default()
    });
    let b = browser.begin(BrowseQuery {
        search: "second".into(),
        sort: BrowseSort::Name,
        ..Default::default()
    });
    assert!(!browser.accept(a, Err(SourceLookupError::Network)));
    assert!(browser.loading);
    assert!(browser.accept(b, Ok(page(0, 24, 50))));
    assert!(!browser.accept(a, Ok(page(0, 1, 1))));
    assert_eq!(browser.active.search, "second");
    assert_eq!(browser.active.sort, BrowseSort::Name);
    assert!(browser.error.is_none());
}

#[test]
fn pagination_uses_committed_search_and_filter_not_unsent_draft() {
    let mut browser = Browser::default();
    let query = BrowseQuery {
        search: "hair".into(),
        category: Some(42),
        ..Default::default()
    };
    let generation = browser.begin(query.clone());
    browser.accept(generation, Ok(page(0, 24, 50)));
    let next = browser.next_query().unwrap();
    assert_eq!(next, BrowseQuery { index: 24, ..query });
    let generation = browser.begin(next);
    assert!(
        browser.page.is_none(),
        "old cards must not appear under a new page label"
    );
    browser.accept(generation, Ok(page(24, 24, 50)));
    assert_eq!(browser.previous_query().unwrap().index, 0);
    assert_eq!(browser.next_query().unwrap().index, 48);
    browser.begin(BrowseQuery::default());
    assert!(browser.next_query().is_none());
}

#[test]
fn missing_key_and_provider_failures_are_actionable_not_no_matches() {
    let mut browser = Browser::default();
    let generation = browser.begin(BrowseQuery::default());
    browser.accept(generation, Err(SourceLookupError::MissingApiKey));
    assert!(!browser.loading);
    assert!(browser.page.is_none());
    assert!(browser.error_message().unwrap().contains("Settings"));
    let generation = browser.begin(BrowseQuery::default());
    browser.accept(generation, Err(SourceLookupError::RateLimited));
    assert!(browser.error_message().unwrap().contains("wait"));
    let generation = browser.begin(BrowseQuery::default());
    browser.accept(
        generation,
        Ok(parse_browse_response(
            r#"{"data":[],"pagination":{"index":0,"pageSize":24,"resultCount":0,"totalCount":0}}"#,
        )
        .unwrap()),
    );
    assert!(browser.error.is_none());
    assert!(browser.page.unwrap().projects.is_empty());
}
