use plumcrate_core::{
    curseforge_client::SourceLookupError,
    curseforge_details::{DescriptionDocument, ProjectReference, SafeUrl},
};
use plumcrate::project_details::{Details, Load};

#[test]
fn description_and_release_outcomes_are_independent_and_selection_scoped() {
    let mut state = Details::default();
    let old = state.begin(1, Some("synthetic-a"));
    let current = state.begin(2, Some("synthetic-a"));
    assert!(!state.accept_description(old, Ok(DescriptionDocument::default())));
    assert!(matches!(state.description, Load::Loading));
    assert!(state.accept_description(current, Ok(DescriptionDocument::default())));
    assert!(matches!(state.description, Load::Ready(_)));
    assert!(matches!(state.project, Load::Loading));
    assert!(state.accept_project(current, Err("Unavailable releases".into())));
    assert!(matches!(state.description, Load::Ready(_)));
    state.clear();
    assert!(!state.accept_description(current, Ok(DescriptionDocument::default())));
}

#[test]
fn saved_key_and_file_changes_reject_old_related_project_results() {
    let mut state = Details::default();
    let old = state.begin(1, Some("synthetic-a"));
    let current = state.begin(1, Some("synthetic-b"));
    state.chosen = Some(100);
    let reference = ProjectReference {
        project_id: 9,
        title: "Real provider project".into(),
        source_url: SafeUrl::parse("https://www.curseforge.com/sims4/mods/library").unwrap(),
    };
    assert!(!state.accept_reference(old, 100, 9, Ok(reference.clone())));
    state.chosen = Some(101);
    assert!(!state.accept_reference(current, 100, 9, Ok(reference.clone())));
    assert!(state.accept_reference(current, 101, 9, Err(SourceLookupError::Network)));
    assert!(matches!(state.references.get(&9), Some(Load::Failed(_))));
    assert!(state.accept_reference(current, 101, 9, Ok(reference)));
    assert!(matches!(state.references.get(&9), Some(Load::Ready(_))));
}
