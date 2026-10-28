use plumcrate_core::{
    curseforge_browse::{BrowseQuery, BrowseSort, parse_browse_response},
    curseforge_client::SourceLookupError,
};
use plumcrate::discovery::Browser;

fn page(index: u32, count: u32, total: u32) -> plumcrate_core::curseforge_browse::BrowsePage {
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
        class_id: Some(10),
        game_version: Some("1.128.90".into()),
        ..Default::default()
    };
    let generation = browser.begin(query.clone());
    browser.accept(generation, Ok(page(0, 24, 50)));
    let next = browser.next_query().unwrap();
    assert_eq!(next, BrowseQuery { index: 24, ..query });
    let generation = browser.navigate(next);
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
fn next_page_preloads_without_replacing_current_results_then_navigation_is_immediate() {
    let mut browser = Browser::default();
    let query = BrowseQuery {
        search: "garden".into(),
        class_id: Some(30),
        category: Some(42),
        game_version: Some("1.128".into()),
        sort: BrowseSort::Updated,
        index: 0,
    };
    let generation = browser.begin(query.clone());
    browser.accept(generation, Ok(page(0, 24, 90)));
    let (epoch, next) = browser.take_prefetch().unwrap();
    assert_eq!(
        next,
        BrowseQuery {
            index: 24,
            ..query.clone()
        }
    );
    assert!(
        browser.take_prefetch().is_none(),
        "only one speculative request per page"
    );
    assert!(browser.accept_prefetch(epoch, next.clone(), Ok(page(24, 24, 90))));
    assert_eq!(browser.page.as_ref().unwrap().pagination.index, 0);
    assert!(!browser.loading);
    browser.navigate(next);
    assert!(!browser.loading, "cached Next must skip the loading state");
    assert_eq!(browser.page.as_ref().unwrap().pagination.index, 24);
    assert_eq!(browser.take_prefetch().unwrap().1.index, 48);
    browser.navigate(browser.previous_query().unwrap());
    assert!(!browser.loading, "recent Previous is cached too");
    assert_eq!(browser.page.as_ref().unwrap().pagination.index, 0);
}

#[test]
fn clicking_next_adopts_the_running_prefetch_instead_of_requesting_it_again() {
    let mut browser = Browser::default();
    let generation = browser.begin(BrowseQuery::default());
    browser.accept(generation, Ok(page(0, 24, 50)));
    let (epoch, next) = browser.take_prefetch().unwrap();
    browser.navigate(next.clone());
    assert!(browser.loading);
    assert!(browser.awaiting_prefetch());
    assert!(browser.take_prefetch().is_none());
    assert!(browser.accept_prefetch(epoch, next, Ok(page(24, 24, 50))));
    assert!(!browser.loading);
    assert_eq!(browser.page.as_ref().unwrap().pagination.index, 24);
}

#[test]
fn refresh_and_every_filter_dimension_invalidate_old_prefetches() {
    for replacement in [
        BrowseQuery::default(),
        BrowseQuery {
            class_id: Some(30),
            ..Default::default()
        },
        BrowseQuery {
            category: Some(42),
            ..Default::default()
        },
        BrowseQuery {
            game_version: Some("1.128".into()),
            ..Default::default()
        },
        BrowseQuery {
            sort: BrowseSort::Name,
            ..Default::default()
        },
        BrowseQuery {
            search: "other".into(),
            ..Default::default()
        },
    ] {
        let mut browser = Browser::default();
        let generation = browser.begin(BrowseQuery::default());
        browser.accept(generation, Ok(page(0, 24, 50)));
        let (epoch, next) = browser.take_prefetch().unwrap();
        browser.begin(replacement.clone());
        assert!(!browser.accept_prefetch(epoch, next, Ok(page(24, 24, 50))));
        assert!(browser.page.is_none());
        assert!(browser.loading);
        assert_eq!(browser.active, replacement);
    }
}

#[test]
fn speculative_errors_do_not_hide_results_or_retry_in_a_loop() {
    let mut browser = Browser::default();
    let generation = browser.begin(BrowseQuery::default());
    browser.accept(generation, Ok(page(0, 24, 50)));
    let (epoch, next) = browser.take_prefetch().unwrap();
    browser.accept_prefetch(epoch, next.clone(), Err(SourceLookupError::RateLimited));
    assert!(browser.error.is_none());
    assert_eq!(browser.page.as_ref().unwrap().pagination.index, 0);
    assert!(browser.take_prefetch().is_none());
    browser.navigate(next);
    assert!(browser.loading);
    assert!(
        !browser.awaiting_prefetch(),
        "explicit navigation can retry normally"
    );
}

#[test]
fn preloading_uses_provider_page_boundaries_and_stops_at_the_last_page() {
    let mut browser = Browser::default();
    let generation = browser.begin(BrowseQuery::default());
    browser.accept(generation, Ok(page(0, 17, 19)));
    let (epoch, next) = browser.take_prefetch().unwrap();
    assert_eq!(next.index, 17);
    browser.accept_prefetch(epoch, next.clone(), Ok(page(17, 2, 19)));
    browser.navigate(next);
    assert!(browser.next_query().is_none());
    assert!(browser.take_prefetch().is_none());
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
