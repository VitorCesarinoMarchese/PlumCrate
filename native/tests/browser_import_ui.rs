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

#[test]
fn card_downloads_use_the_existing_selected_game_requirement() {
    use ts4_mod_manager_core::curseforge_browse::{BrowseQuery, parse_browse_response};
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
    let generation = view.discovery.browser.begin(BrowseQuery::default());
    view.discovery.browser.accept(generation, Ok(parse_browse_response(
        r#"{"data":[{"id":42,"gameId":78062,"name":"Synthetic card project","summary":"Summary"}],"pagination":{"index":0,"pageSize":24,"resultCount":1,"totalCount":1}}"#
    ).unwrap()));
    let ctx = egui::Context::default();
    ts4_mod_manager_native::ui::setup(&ctx);
    let mut render = |events| {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 1000.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                view.show_with_controls(ui, &mut catalog, &mut controls);
            },
        );
        output.textures_delta.clear();
        (output, view.discovery.stats.card_actions.first().copied())
    };
    for _ in 0..3 {
        render(vec![]);
    }
    let (_, action) = render(vec![]);
    render(click(action.unwrap()));
    for _ in 0..3 {
        render(vec![]);
    }
    let (output, _) = render(vec![]);
    assert!(output.shapes.iter().any(
        |shape| matches!(&shape.shape, egui::epaint::Shape::Text(text)
        if text.galley.text().contains("Choose a Sims 4 game folder before downloading a mod"))
    ));
    assert!(!controls.browser_import.running());
    assert!(!temp.path().join("managed/mods").exists());
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
fn a_dependency_download_starts_only_that_project_then_imports_and_enables_it() {
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
    assert_eq!(fixture.controls.browser_import.phase,Phase::Ready);
    assert_eq!(fixture.controls.browser_import.chosen,Some(200));
    assert!(!fixture.directory.path().join("game/Mods/core-library.package").exists());
    let output=frame(&ctx,&mut fixture,vec![]);
    assert!(!fixture.controls.review_open());
    assert!(output.platform_output.commands.iter().any(|c|matches!(c,egui::OutputCommand::OpenUrl(url) if url.url.ends_with("/synthetic-core/download/200"))));
    let deadline=std::time::Instant::now()+std::time::Duration::from_secs(5);
    while fixture.controls.browser_import.phase!=Phase::Complete {
        fixture.controls.poll(&mut fixture.catalog);
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
fn one_download_action_opens_the_download_page_then_imports_and_enables_without_more_clicks() {
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
    fixture.controls.browser_import.cancel();
    fixture.controls.prepare_browser_import(&ctx, &fixture.catalog, ts4_mod_manager_native::project_details::ReviewRequest::Loaded {
        project: fixture.project.clone(), file_id: 100,
    });
    let output = frame(&ctx, &mut fixture, vec![]);
    assert!(!fixture.controls.review_open());
    assert!(output.platform_output.commands.iter().any(|command|matches!(command,egui::OutputCommand::OpenUrl(url) if url.url==fixture.project.releases[0].navigation_url(&fixture.project))));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while fixture.controls.browser_import.phase
        != ts4_mod_manager_native::browser_import::Phase::Complete
    {
        fixture.controls.poll(&mut fixture.catalog);
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
fn cancelling_preparation_does_not_import_an_existing_matching_download() {
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
    let activity = fixture.controls.activity_button_rect.unwrap();
    frame(&ctx, &mut fixture, click(activity));
    frame(&ctx, &mut fixture, vec![]);
    let cancel = fixture.controls.browser_cancel_rect.unwrap();
    frame(&ctx, &mut fixture, click(cancel));
    fixture.controls.poll(&mut fixture.catalog);
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
fn direct_download_keeps_activity_reachable_in_a_small_window() {
    let mut fixture = support::fixture();
    fixture.controls.browser_import.cancel();
    let ctx = egui::Context::default();
    ts4_mod_manager_native::ui::setup(&ctx);
    fixture.controls.prepare_browser_import(&ctx, &fixture.catalog, ts4_mod_manager_native::project_details::ReviewRequest::Loaded {
        project: fixture.project.clone(), file_id: 100,
    });
    let size = egui::vec2(640.0, 480.0);
    sized_frame(&ctx, &mut fixture, vec![], size);
    sized_frame(&ctx, &mut fixture, vec![], size);
    assert!(!fixture.controls.review_open());
    assert!(egui::Rect::from_min_size(egui::Pos2::ZERO, size).contains_rect(fixture.controls.activity_button_rect.unwrap()));
}

#[test]
fn download_action_opens_the_browser_download_page_without_an_extra_popup() {
    use ts4_mod_manager_native::{browser_import::Phase, project_details::ReviewRequest};
    let mut fixture = support::fixture();
    fixture.controls.browser_import.cancel();
    let ctx = egui::Context::default();
    ts4_mod_manager_native::ui::setup(&ctx);
    fixture.controls.prepare_browser_import(&ctx, &fixture.catalog, ReviewRequest::Loaded {
        project: fixture.project.clone(), file_id: 100,
    });
    let output = frame(&ctx, &mut fixture, vec![]);
    assert!(!fixture.controls.review_open(), "the download action must not add a confirmation popup");
    assert_eq!(fixture.controls.browser_import.phase, Phase::Waiting);
    assert!(output.platform_output.commands.iter().any(|command| matches!(command,
        egui::OutputCommand::OpenUrl(url) if url.url == "https://www.curseforge.com/sims4/mods/synthetic-fixture/download/100")));
    assert!(!fixture.directory.path().join("managed/mods").exists(), "only a verified completed file may be imported");
    let repeated = frame(&ctx, &mut fixture, vec![]);
    assert!(!repeated.platform_output.commands.iter().any(|command| matches!(command, egui::OutputCommand::OpenUrl(_))), "one action must not open multiple browser tabs");
}

#[test]
fn changing_game_before_dispatch_does_not_open_or_install_the_download() {
    use ts4_mod_manager_native::{browser_import::Phase, project_details::ReviewRequest};
    let mut fixture = support::fixture();
    fixture.controls.browser_import.cancel();
    let ctx = egui::Context::default();
    ts4_mod_manager_native::ui::setup(&ctx);
    fixture.controls.prepare_browser_import(&ctx, &fixture.catalog, ReviewRequest::Loaded { project: fixture.project.clone(), file_id: 100 });
    let other = fixture.directory.path().join("other-game");
    std::fs::create_dir_all(other.join("Mods")).unwrap();
    let generation = fixture.catalog.begin_scan(other);
    fixture.catalog.accept_scan(generation, Ok(vec![]));
    let output = frame(&ctx, &mut fixture, vec![]);
    assert_eq!(fixture.controls.browser_import.phase, Phase::Cancelled);
    assert!(!output.platform_output.commands.iter().any(|command| matches!(command, egui::OutputCommand::OpenUrl(_))));
    assert!(!fixture.directory.path().join("managed/mods").exists());
}
