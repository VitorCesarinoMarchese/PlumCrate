use std::path::Path;
use ts4_mod_manager_native::browser_import::{BrowserImport, Phase};

#[test]
fn browser_visit_does_not_authorize_an_import() {
    let home = tempfile::tempdir().unwrap();
    let mut import = BrowserImport::new(home.path());
    assert_eq!(import.phase, Phase::Idle);
    assert!(!import.running());
    assert!(!import.is_review_open());
    assert!(import.start(&egui::Context::default()).is_err());
    assert!(!import.running());
}

#[test]
fn stale_completion_cannot_change_the_session() {
    let home = tempfile::tempdir().unwrap();
    let mut import = BrowserImport::new(home.path());
    import.finish(import.token().wrapping_add(1), Ok(()));
    assert_eq!(import.phase, Phase::Idle);
}

#[test]
fn a_directory_choice_is_not_an_import_approval() {
    let mut import = BrowserImport::new(Path::new("/tmp/browser-import-fixture"));
    import.download_dir = Path::new("/tmp").to_owned();
    assert!(import.retry(&egui::Context::default()).is_err());
    assert_eq!(import.phase, Phase::Idle);
}

#[test]
fn a_release_lookup_failure_does_not_offer_an_unapproved_watch_retry() {
    let home = tempfile::tempdir().unwrap();
    let mut import = BrowserImport::new(home.path());
    import.phase = Phase::Failed;
    assert!(!import.can_retry());
}
