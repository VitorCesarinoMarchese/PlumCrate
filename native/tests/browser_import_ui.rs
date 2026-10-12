use ts4_mod_manager_native::{catalog::Catalog, controls::Controls, settings::Settings, ui::View};

#[test]
fn browsing_offers_browser_import_and_requires_a_selected_game() {
    let temp = tempfile::tempdir().unwrap();
    let mut controls = Controls::new(
        temp.path().join("managed"),
        temp.path().into(),
        Settings::default(),
        || {},
    )
    .unwrap();
    let mut catalog = Catalog::default();
    let mut view = View::default();
    view.browse_page = true;
    view.discovery.browser.begin(Default::default());
    view.discovery.selected = Some(ts4_mod_manager_core::curseforge_browse::BrowseProject {
        id: 42,
        name: "Synthetic project".into(),
        authors: vec!["Synthetic author".into()],
        summary: "Synthetic browser import fixture".into(),
        source_url: Some("https://www.curseforge.com/sims4/mods/synthetic-fixture".into()),
        preview_url: None,
        screenshots: vec![],
        downloads: 0.0,
        updated: None,
        categories: vec![],
    });
    let ctx = egui::Context::default();
    ts4_mod_manager_native::ui::setup(&ctx);
    for _ in 0..3 {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 800.0),
                )),
                ..Default::default()
            },
            |ui| {
                view.show_with_controls(ui, &mut catalog, &mut controls);
            },
        );
        output.textures_delta.clear();
    }
    assert!(
        view.discovery.stats.browser_import.is_some(),
        "browser import action in project details"
    );
    assert!(!controls.browser_import.running());
}

#[path = "support/browser_import.rs"]
mod support;
#[allow(dead_code)]
#[path="support/project_details.rs"] mod details_support;
fn click(rect: egui::Rect) -> Vec<egui::Event> {
    let pos = rect.center();
    vec![
        egui::Event::PointerMoved(pos),
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        },
        egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        },
    ]
}
fn frame(
    ctx: &egui::Context,
    fixture: &mut support::Fixture,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    sized_frame(ctx, fixture, events, egui::vec2(1200.0, 800.0))
}

#[test]
fn a_dependency_review_confirms_only_that_project_then_imports_and_enables_it() {
    use ts4_mod_manager_core::{browser_import::load_browser_project,curseforge_client::CurseForgeClient};
    use ts4_mod_manager_native::{browser_import::Phase,project_details::ReviewRequest};
    let mut fixture=support::fixture();
    fixture.controls.browser_import.cancel();
    let ctx=egui::Context::default();ts4_mod_manager_native::ui::setup(&ctx);
    let client=CurseForgeClient::new(Some("synthetic"),details_support::Provider).unwrap();
    let dependency=load_browser_project(&client,9).unwrap();
    let original=fixture.directory.path().join("Downloads/core-library.package");
    std::fs::write(&original,support::BYTES).unwrap();
    fixture.controls.prepare_browser_import(&ctx,&fixture.catalog,ReviewRequest::Loaded {project:dependency,file_id:200});
    assert_eq!(fixture.controls.browser_import.phase,Phase::Review);
    assert_eq!(fixture.controls.browser_import.chosen,Some(200));
    for _ in 0..3 {frame(&ctx,&mut fixture,vec![]);}
    assert!(!fixture.directory.path().join("game/Mods/core-library.package").exists());
    let start=fixture.controls.browser_start_rect.unwrap();
    let output=frame(&ctx,&mut fixture,click(start));
    assert!(output.platform_output.commands.iter().any(|c|matches!(c,egui::OutputCommand::OpenUrl(url) if url.url.ends_with("/synthetic-core/files/200"))));
    let deadline=std::time::Instant::now()+std::time::Duration::from_secs(5);
    while fixture.controls.browser_import.phase!=Phase::Complete {
        fixture.controls.poll(&fixture.catalog);
        assert!(std::time::Instant::now()<deadline,"{:?}",fixture.controls.browser_import.error);
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert_eq!(std::fs::read(&original).unwrap(),support::BYTES);
    assert_eq!(std::fs::read(fixture.directory.path().join("game/Mods/core-library.package")).unwrap(),support::BYTES);
    assert!(!fixture.directory.path().join("game/Mods/garden.package").exists());
}
fn sized_frame(ctx: &egui::Context, fixture: &mut support::Fixture, events: Vec<egui::Event>, size: egui::Vec2) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
            events,
            ..Default::default()
        },
        |ui| {
            fixture
                .view
                .show_with_controls(ui, &mut fixture.catalog, &mut fixture.controls);
            fixture.controls.show_dialogs(ui.ctx(), &fixture.catalog);
        },
    );
    output.textures_delta.clear();
    output
}
#[test]
fn one_approval_opens_the_release_page_then_imports_and_enables_without_more_clicks() {
    let mut fixture = support::fixture();
    let ctx = egui::Context::default();
    ts4_mod_manager_native::ui::setup(&ctx);
    let downloads = fixture
        .directory
        .path()
        .join("Downloads/renamed (1).package");
    std::fs::write(&downloads, support::BYTES).unwrap();
    for _ in 0..3 {
        frame(&ctx, &mut fixture, vec![]);
    }
    assert_eq!(
        std::fs::read_dir(fixture.directory.path().join("game/Mods"))
            .unwrap()
            .count(),
        0
    );
    assert!(!fixture.directory.path().join("managed/mods").exists());
    let start = fixture.controls.browser_start_rect.unwrap();
    let output = frame(&ctx, &mut fixture, click(start));
    assert!(output.platform_output.commands.iter().any(|command|matches!(command,egui::OutputCommand::OpenUrl(url) if url.url==fixture.project.releases[0].navigation_url(&fixture.project))));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while fixture.controls.browser_import.phase
        != ts4_mod_manager_native::browser_import::Phase::Complete
    {
        fixture.controls.poll(&fixture.catalog);
        frame(&ctx, &mut fixture, vec![]);
        assert!(
            std::time::Instant::now() < deadline,
            "{:?}: {:?}",
            fixture.controls.browser_import.phase,
            fixture.controls.browser_import.error
        );
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(std::fs::read(&downloads).unwrap(), support::BYTES);
    assert_eq!(
        std::fs::read(fixture.directory.path().join("game/Mods/fixture.package")).unwrap(),
        support::BYTES
    );
    let mods: Vec<_> = std::fs::read_dir(fixture.directory.path().join("managed/mods"))
        .unwrap()
        .collect();
    assert_eq!(mods.len(), 1);
    let metadata: serde_json::Value = serde_json::from_slice(
        &std::fs::read(mods[0].as_ref().unwrap().path().join("meta.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(metadata["sourceAttachment"]["attachedBy"], "user");
    assert_eq!(
        metadata["sourceAttachment"]["installedRelease"]["fileId"],
        100
    );
    assert_eq!(metadata["sourceUrl"], fixture.project.source_url);
}
#[test]
fn cancelling_the_review_does_not_import_an_existing_matching_download() {
    let mut fixture = support::fixture();
    let ctx = egui::Context::default();
    ts4_mod_manager_native::ui::setup(&ctx);
    std::fs::write(
        fixture.directory.path().join("Downloads/fixture.package"),
        support::BYTES,
    )
    .unwrap();
    for _ in 0..3 {
        frame(&ctx, &mut fixture, vec![]);
    }
    let cancel = fixture.controls.browser_cancel_rect.unwrap();
    frame(&ctx, &mut fixture, click(cancel));
    fixture.controls.poll(&fixture.catalog);
    assert_eq!(
        fixture.controls.browser_import.phase,
        ts4_mod_manager_native::browser_import::Phase::Cancelled
    );
    assert!(!fixture.directory.path().join("managed/mods").exists());
    assert_eq!(
        std::fs::read_dir(fixture.directory.path().join("game/Mods"))
            .unwrap()
            .count(),
        0
    );
}

#[test]
fn approval_keeps_the_game_destination_visible_in_a_small_window() {
    let mut fixture = support::fixture();
    let ctx = egui::Context::default();
    ts4_mod_manager_native::ui::setup(&ctx);
    let size = egui::vec2(640.0, 480.0);
    let path = fixture.catalog.root.as_ref().unwrap().join("Mods").display().to_string();
    let mut output = sized_frame(&ctx, &mut fixture, vec![], size);
    for _ in 0..3 { output = sized_frame(&ctx, &mut fixture, vec![], size); }
    assert!(output.shapes.iter().any(|clipped| match &clipped.shape {
        egui::epaint::Shape::Text(text) => text.galley.text() == path && clipped.clip_rect.contains_rect(text.galley.rect.translate(text.pos.to_vec2())),
        _ => false,
    }), "approved game destination must be visible without scrolling");
    assert!(egui::Rect::from_min_size(egui::Pos2::ZERO, size).contains_rect(fixture.controls.browser_start_rect.unwrap()));
}
