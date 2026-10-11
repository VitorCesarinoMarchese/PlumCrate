use ts4_mod_manager_native::{catalog::Catalog, updates::UpdateState};

#[test]
fn update_results_follow_the_game_scan_and_saved_key_but_not_selection() {
    let mut catalog = Catalog::default();
    let generation = catalog.begin_scan("/game-a".into());
    catalog.accept_scan(generation, Ok(vec![]));
    let mut updates = UpdateState::default();
    updates.sync(&catalog, Some("fixture-key"));
    let token = updates.begin();
    assert!(updates.accept(token, vec![("mod".into(), Err("fixture failure".into()))]));
    assert!(updates.reports.contains_key("mod"));
    updates.sync(&catalog, Some("fixture-key"));
    assert!(updates.reports.contains_key("mod"));
    let stale = updates.begin();
    catalog.begin_scan("/game-a".into());
    updates.sync(&catalog, Some("fixture-key"));
    assert!(!updates.accept(stale, vec![]));
    assert!(updates.reports.is_empty());
    let stale = updates.begin();
    updates.sync(&catalog, Some("replacement-fixture"));
    assert!(!updates.accept(stale, vec![]));
    let stale = updates.begin();
    catalog.begin_scan("/game-b".into());
    updates.sync(&catalog, Some("replacement-fixture"));
    assert!(!updates.accept(stale, vec![]));
}

#[test]
fn metadata_mutations_cancel_pending_checks() {
    let mut updates = UpdateState::default();
    let token = updates.begin();
    let cancellation = updates.epoch.clone();
    updates.invalidate();
    assert_ne!(
        cancellation.load(std::sync::atomic::Ordering::Acquire),
        token
    );
    assert!(!updates.accept(token, vec![]));
    assert!(!updates.pending);
}
