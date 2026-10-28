use plumcrate_core::{browser_import::{BrowserProject, load_browser_project}, curseforge_client::CurseForgeClient};
use plumcrate::{browser_import::Phase, project_details::ReviewRequest};

#[path = "support/browser_import.rs"]
mod support;
#[allow(dead_code)]
#[path = "support/project_details.rs"]
mod details_support;

fn frame(ctx: &egui::Context, fixture: &mut support::Fixture, events: Vec<egui::Event>) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1200.0, 800.0))),
            events,
            ..Default::default()
        },
        |ui| {
            fixture.view.show_with_controls(ui, &mut fixture.catalog, &mut fixture.controls);
            fixture.controls.show_dialogs(ui.ctx(), &fixture.catalog);
        },
    );
    output.textures_delta.clear();
    output
}
fn click(rect: egui::Rect) -> Vec<egui::Event> {
    let pos = rect.center();
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() },
        egui::Event::PointerButton { pos, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() },
    ]
}
fn text(output: &egui::FullOutput, value: &str) -> bool {
    output.shapes.iter().any(|shape| matches!(&shape.shape, egui::epaint::Shape::Text(text) if text.galley.text().contains(value)))
}
fn opened(output: &egui::FullOutput, suffix: &str) -> bool {
    output.platform_output.commands.iter().any(|command| matches!(command, egui::OutputCommand::OpenUrl(url) if url.url.ends_with(suffix)))
}
fn dependency() -> BrowserProject {
    load_browser_project(&CurseForgeClient::new(Some("synthetic"), details_support::Provider).unwrap(), 9).unwrap()
}
fn start_two(ctx: &egui::Context) -> support::Fixture {
    let mut fixture = support::fixture();
    fixture.controls.browser_import.cancel();
    fixture.controls.prepare_browser_import(ctx, &fixture.catalog, ReviewRequest::Loaded { project: fixture.project.clone(), file_id: 100 });
    let first = frame(ctx, &mut fixture, vec![]);
    assert!(opened(&first, "/synthetic-fixture/download/100"));
    assert_eq!(fixture.controls.browser_import.phase, Phase::Waiting);
    fixture.controls.prepare_browser_import(ctx, &fixture.catalog, ReviewRequest::Loaded { project: dependency(), file_id: 200 });
    let second = frame(ctx, &mut fixture, vec![]);
    assert!(!opened(&second, "/download/200"), "a queued download must wait for the active one");
    fixture
}
fn wait_for(ctx: &egui::Context, fixture: &mut support::Fixture, project_id: u64, phase: Phase) -> Vec<egui::FullOutput> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    let mut outputs = vec![];
    while !(fixture.controls.browser_import.phase == phase
        && fixture.controls.browser_import.project.as_ref().is_some_and(|project| project.project_id == project_id))
    {
        fixture.controls.poll(&mut fixture.catalog);
        outputs.push(frame(ctx, fixture, vec![]));
        assert!(std::time::Instant::now() < deadline, "{:?}: {:?}", fixture.controls.browser_import.phase, fixture.controls.browser_import.error);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    outputs
}

#[test]
fn a_second_download_waits_in_the_queue_then_starts_after_the_first_installs() {
    let ctx = egui::Context::default();
    plumcrate::ui::setup(&ctx);
    let mut fixture = start_two(&ctx);
    assert_eq!(fixture.controls.browser_import.project.as_ref().unwrap().project_id, 42);
    assert_eq!(fixture.controls.download_queue(), vec![(9, "Synthetic Core Library".to_owned())]);
    assert_eq!(fixture.controls.download_positions(), vec![42, 9]);
    std::fs::write(fixture.directory.path().join("Downloads/fixture.package"), support::BYTES).unwrap();
    let outputs = wait_for(&ctx, &mut fixture, 9, Phase::Waiting);
    assert!(outputs.iter().any(|output| opened(output, "/synthetic-core/download/200")), "the next queued download opens on its own");
    assert!(fixture.controls.download_queue().is_empty());
    assert_eq!(std::fs::read(fixture.directory.path().join("game/Mods/fixture.package")).unwrap(), support::BYTES);
    std::fs::write(fixture.directory.path().join("Downloads/core-library.package"), support::BYTES).unwrap();
    wait_for(&ctx, &mut fixture, 9, Phase::Complete);
    assert_eq!(std::fs::read(fixture.directory.path().join("game/Mods/core-library.package")).unwrap(), support::BYTES);
}

#[test]
fn activity_marks_the_waiting_mod_as_next_and_can_remove_it() {
    let ctx = egui::Context::default();
    plumcrate::ui::setup(&ctx);
    let mut fixture = start_two(&ctx);
    let activity = fixture.controls.activity_button_rect.unwrap();
    frame(&ctx, &mut fixture, click(activity));
    let output = frame(&ctx, &mut fixture, vec![]);
    assert!(text(&output, "Waiting for browser download"));
    assert!(text(&output, "Synthetic Core Library"));
    assert!(text(&output, "Next in queue"));
    let remove = fixture.controls.queue_remove_rects[0];
    frame(&ctx, &mut fixture, click(remove));
    assert!(fixture.controls.download_queue().is_empty());
    assert_eq!(fixture.controls.browser_import.phase, Phase::Waiting, "removing a queued mod keeps the active download");
}

#[test]
fn queueing_the_same_mod_twice_keeps_one_entry() {
    let ctx = egui::Context::default();
    plumcrate::ui::setup(&ctx);
    let mut fixture = start_two(&ctx);
    fixture.controls.prepare_browser_import(&ctx, &fixture.catalog, ReviewRequest::Loaded { project: dependency(), file_id: 200 });
    fixture.controls.prepare_browser_import(&ctx, &fixture.catalog, ReviewRequest::ProjectId(42));
    frame(&ctx, &mut fixture, vec![]);
    assert_eq!(fixture.controls.download_positions(), vec![42, 9]);
}

#[test]
fn a_failed_download_pauses_the_queue_until_the_user_skips_it() {
    let ctx = egui::Context::default();
    plumcrate::ui::setup(&ctx);
    let mut fixture = start_two(&ctx);
    let token = fixture.controls.browser_import.token();
    fixture.controls.browser_import.finish(token, Err("Synthetic failure".into()));
    for _ in 0..3 {
        let output = frame(&ctx, &mut fixture, vec![]);
        assert!(!opened(&output, "/download/200"));
    }
    assert_eq!(fixture.controls.browser_import.phase, Phase::Failed);
    assert_eq!(fixture.controls.download_queue().len(), 1);
    let activity = fixture.controls.activity_button_rect.unwrap();
    frame(&ctx, &mut fixture, click(activity));
    let output = frame(&ctx, &mut fixture, vec![]);
    assert!(text(&output, "Download queue paused"));
    let skip = fixture.controls.queue_skip_rect.unwrap();
    let mut outputs = vec![frame(&ctx, &mut fixture, click(skip))];
    outputs.extend(wait_for(&ctx, &mut fixture, 9, Phase::Waiting));
    assert!(outputs.iter().any(|output| opened(output, "/synthetic-core/download/200")));
}

#[test]
fn changing_the_game_folder_clears_queued_downloads() {
    let ctx = egui::Context::default();
    plumcrate::ui::setup(&ctx);
    let mut fixture = start_two(&ctx);
    let other = fixture.directory.path().join("other-game");
    std::fs::create_dir_all(other.join("Mods")).unwrap();
    let generation = fixture.catalog.begin_scan(other);
    fixture.catalog.accept_scan(generation, Ok(vec![]));
    for _ in 0..3 {
        let output = frame(&ctx, &mut fixture, vec![]);
        assert!(!opened(&output, "/download/200"));
    }
    assert!(fixture.controls.download_queue().is_empty());
}

#[test]
fn browse_cards_show_which_mod_is_downloading_and_which_is_next() {
    use plumcrate_core::curseforge_browse::parse_browse_response;
    let ctx = egui::Context::default();
    plumcrate::ui::setup(&ctx);
    let mut fixture = start_two(&ctx);
    fixture.view.browse_page = true;
    frame(&ctx, &mut fixture, vec![]);
    let generation = fixture.view.discovery.browser.begin(Default::default());
    fixture.view.discovery.browser.accept(generation, Ok(parse_browse_response(
        r#"{"data":[{"id":42,"gameId":78062,"name":"Active card","summary":"Summary"},{"id":9,"gameId":78062,"name":"Queued card","summary":"Summary"}],"pagination":{"index":0,"pageSize":24,"resultCount":2,"totalCount":2}}"#
    ).unwrap()));
    let mut output = frame(&ctx, &mut fixture, vec![]);
    for _ in 0..3 {
        output = frame(&ctx, &mut fixture, vec![]);
    }
    assert!(text(&output, "Downloading…"), "cards={} skeleton={}", fixture.view.discovery.stats.cards, fixture.view.discovery.stats.skeleton_cards);
    assert!(text(&output, "Next in queue"));
}
