use ts4_mod_manager_core::curseforge_browse::{BrowseQuery, parse_browse_response};
use ts4_mod_manager_native::{
    catalog::Catalog,
    ui::{self, View},
};
fn frame(
    ctx: &egui::Context,
    view: &mut View,
    catalog: &mut Catalog,
    size: [f32; 2],
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size.into())),
            events,
            ..Default::default()
        },
        |ui| {
            view.show(ui, catalog);
        },
    );
    output.textures_delta.clear();
    output
}
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
fn text(output: &egui::FullOutput, value: &str) -> bool {
    output.shapes.iter().any(
        |s| matches!(&s.shape, egui::epaint::Shape::Text(t) if t.galley.text().contains(value)),
    )
}
#[test]
fn browse_is_available_without_game_folder_and_preserves_library_search() {
    for size in [[1200.0, 800.0], [640.0, 480.0]] {
        let ctx = egui::Context::default();
        ui::setup(&ctx);
        let mut view = View::default();
        let mut catalog = Catalog::default();
        catalog.search("local draft".into());
        for _ in 0..3 {
            frame(&ctx, &mut view, &mut catalog, size, vec![]);
        }
        let tab = view.stats.browse_tab.unwrap();
        frame(&ctx, &mut view, &mut catalog, size, click(tab));
        let output = frame(&ctx, &mut view, &mut catalog, size, vec![]);
        assert!(view.browse_page);
        assert!(text(&output, "Connect to CurseForge"));
        assert!(text(&output, "Settings"));
        assert!(!text(&output, "No matching mods"));
        assert_eq!(catalog.query, "local draft");
        assert!(catalog.root.is_none());
        let tab = view.stats.library_tab.unwrap();
        frame(&ctx, &mut view, &mut catalog, size, click(tab));
        assert!(!view.browse_page);
        assert_eq!(catalog.query, "local draft");
    }
}
#[test]
fn projects_open_inside_app_and_escape_restores_results_at_all_sizes() {
    for size in [[1200.0, 800.0], [680.0, 760.0], [640.0, 480.0]] {
        let ctx = egui::Context::default();
        ui::setup(&ctx);
        let mut view = View::default();
        view.browse_page = true;
        let generation = view.discovery.browser.begin(BrowseQuery::default());
        view.discovery.browser.accept(generation,Ok(parse_browse_response(r#"{"data":[{"id":9,"gameId":78062,"name":"Synthetic project","summary":"This is a synthetic summary.","authors":[{"name":"Fixture creator"}]}],"pagination":{"index":0,"pageSize":24,"resultCount":1,"totalCount":1}}"#).unwrap()));
        let mut catalog = Catalog::default();
        for _ in 0..3 {
            frame(&ctx, &mut view, &mut catalog, size, vec![]);
        }
        let card = view.discovery.stats.first_card.unwrap();
        frame(&ctx, &mut view, &mut catalog, size, click(card));
        let output = frame(&ctx, &mut view, &mut catalog, size, vec![]);
        assert!(
            view.discovery.selected.is_some(),
            "size {size:?}, card {card:?}"
        );
        assert!(text(&output, "Back to results"));
        assert!(text(&output, "This is a synthetic summary."));
        assert!(
            output.platform_output.commands.is_empty(),
            "opening details must not open a website or copy anything"
        );
        frame(
            &ctx,
            &mut view,
            &mut catalog,
            size,
            vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            }],
        );
        assert!(view.discovery.selected.is_none());
        assert_eq!(
            view.discovery.browser.page.as_ref().unwrap().projects.len(),
            1
        );
    }
}

#[test]
fn control_f_and_enter_submit_browser_query_without_changing_local_search() {
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    let mut view = View::default();
    view.browse_page = true;
    let mut catalog = Catalog::default();
    catalog.search("local query".into());
    let size = [1200.0, 800.0];
    for _ in 0..3 {
        frame(&ctx, &mut view, &mut catalog, size, vec![]);
    }
    frame(
        &ctx,
        &mut view,
        &mut catalog,
        size,
        vec![egui::Event::Key {
            key: egui::Key::F,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::CTRL,
        }],
    );
    frame(
        &ctx,
        &mut view,
        &mut catalog,
        size,
        vec![egui::Event::Text("café hair".into())],
    );
    frame(
        &ctx,
        &mut view,
        &mut catalog,
        size,
        vec![egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }],
    );
    assert_eq!(view.discovery.browser.active.search, "café hair");
    assert_eq!(catalog.query, "local query");
    assert_eq!(
        view.discovery.browser.error,
        Some(ts4_mod_manager_core::curseforge_client::SourceLookupError::MissingApiKey)
    );
}

#[test]
fn browse_cards_fit_columns_without_overflow_or_squeezed_text() {
    for size in [[1200.0, 800.0], [680.0, 760.0], [640.0, 480.0]] {
        let ctx = egui::Context::default();
        ui::setup(&ctx);
        let mut view = View::default();
        view.browse_page = true;
        let data=(0..24).map(|id|serde_json::json!({"id":id,"gameId":78062,"name":"A long synthetic project title to test wrapping","summary":"This readable summary belongs below the title, rather than in a narrow column beside it."})).collect::<Vec<_>>();
        let generation = view.discovery.browser.begin(BrowseQuery::default());
        view.discovery.browser.accept(generation,Ok(parse_browse_response(&serde_json::json!({"data":data,"pagination":{"index":0,"pageSize":24,"resultCount":24,"totalCount":24}}).to_string()).unwrap()));
        let mut catalog = Catalog::default();
        for _ in 0..3 {
            frame(&ctx, &mut view, &mut catalog, size, vec![]);
        }
        let available = size[0] - 48.0;
        let columns = ((available + 16.0) / 280.0).floor().max(1.0);
        let width = (available - (columns - 1.0) * 16.0) / columns;
        assert!(
            view.discovery.stats.first_card.unwrap().width() <= width + 2.0,
            "cards must fit {columns} columns at {size:?}"
        );
    }
}

#[test]
fn browser_settings_returns_to_browsing_and_uses_only_saved_credentials() {
    let fixture = tempfile::tempdir().unwrap();
    let mut controls = ts4_mod_manager_native::controls::Controls::new(
        fixture.path().join("managed"),
        fixture.path().into(),
        Default::default(),
        || {},
    )
    .unwrap();
    controls.settings.curseforge_api_key = Some("synthetic-unsaved-key".into());
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    let mut view = View::default();
    view.browse_page = true;
    let mut catalog = Catalog::default();
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
    assert!(
        text(&output, "Connect to CurseForge"),
        "unsaved credentials must not reach the provider"
    );
    controls.settings_page = true;
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
    assert!(text(&output, "Back to browsing"));
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            events: vec![egui::Event::Key {
                key: egui::Key::Escape,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: Default::default(),
            }],
            ..Default::default()
        },
        |ui| {
            view.show_with_controls(ui, &mut catalog, &mut controls);
        },
    );
    output.textures_delta.clear();
    assert!(!controls.settings_page);
    assert!(view.browse_page);
    assert_eq!(
        view.discovery.browser.error,
        Some(ts4_mod_manager_core::curseforge_client::SourceLookupError::MissingApiKey)
    );
    assert_eq!(
        controls.settings.curseforge_api_key.as_deref(),
        Some("synthetic-unsaved-key")
    );
    assert!(
        !fixture
            .path()
            .join(".config/ts4-mod-manager/settings.json")
            .exists()
    );
}

#[test]
fn long_and_short_project_headers_keep_actions_aligned() {
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    let mut view = View::default();
    view.browse_page = true;
    let generation = view.discovery.browser.begin(BrowseQuery::default());
    view.discovery.browser.accept(generation,Ok(parse_browse_response(r#"{"data":[{"id":1,"gameId":78062,"name":"A long synthetic project title that wraps across several words","summary":"Summary"},{"id":2,"gameId":78062,"name":"Short title","summary":"Summary"}],"pagination":{"index":0,"pageSize":24,"resultCount":2,"totalCount":2}}"#).unwrap()));
    let mut catalog = Catalog::default();
    for _ in 0..3 {
        frame(&ctx, &mut view, &mut catalog, [1200.0, 800.0], vec![]);
    }
    let actions = &view.discovery.stats.card_actions;
    assert_eq!(actions.len(), 2);
    assert!(
        (actions[0].top() - actions[1].top()).abs() < 1.0,
        "long titles must not shift summary, metadata or actions: {actions:?}"
    );
}

#[test]
fn tab_focus_paints_card_outline_and_enter_opens_project() {
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    let mut view = View::default();
    view.browse_page = true;
    let generation = view.discovery.browser.begin(BrowseQuery::default());
    view.discovery.browser.accept(generation,Ok(parse_browse_response(r#"{"data":[{"id":1,"gameId":78062,"name":"Synthetic project","summary":"Summary"}],"pagination":{"index":0,"pageSize":24,"resultCount":1,"totalCount":1}}"#).unwrap()));
    let mut catalog = Catalog::default();
    let size = [1200.0, 800.0];
    for _ in 0..3 {
        frame(&ctx, &mut view, &mut catalog, size, vec![]);
    }
    let id = view.discovery.stats.first_card_id.unwrap();
    let key = |key| {
        vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }]
    };
    let mut output = frame(
        &ctx,
        &mut view,
        &mut catalog,
        size,
        vec![egui::Event::PointerMoved(egui::pos2(1190.0, 10.0))],
    );
    for _ in 0..20 {
        if ctx.memory(|memory| memory.has_focus(id)) {
            break;
        }
        output = frame(&ctx, &mut view, &mut catalog, size, key(egui::Key::Tab));
    }
    assert!(
        ctx.memory(|memory| memory.has_focus(id)),
        "Tab must reach the card"
    );
    let card = view.discovery.stats.first_card.unwrap();
    let color = ctx.style_of(ctx.theme()).visuals.selection.stroke.color;
    assert!(output.shapes.iter().any(|shape|matches!(&shape.shape,egui::epaint::Shape::Rect(rect) if rect.stroke.color==color && (rect.rect.left()-card.left()).abs()<2.0 && rect.rect.width()>200.0)),"focused card must paint an outline with pointer elsewhere");
    frame(&ctx, &mut view, &mut catalog, size, key(egui::Key::Enter));
    assert_eq!(view.discovery.selected.as_ref().unwrap().id, 1);
}

#[test]
fn escape_closes_help_before_leaving_a_browse_project() {
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    let mut view = View::default();
    view.browse_page = true;
    let generation = view.discovery.browser.begin(BrowseQuery::default());
    view.discovery.browser.accept(generation,Ok(parse_browse_response(r#"{"data":[{"id":1,"gameId":78062,"name":"Synthetic project","summary":"Summary"}],"pagination":{"index":0,"pageSize":24,"resultCount":1,"totalCount":1}}"#).unwrap()));
    view.discovery.selected =
        Some(view.discovery.browser.page.as_ref().unwrap().projects[0].clone());
    let mut catalog = Catalog::default();
    let size = [1200.0, 800.0];
    let key = |key| {
        vec![egui::Event::Key {
            key,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Default::default(),
        }]
    };
    for _ in 0..3 {
        frame(&ctx, &mut view, &mut catalog, size, vec![]);
    }
    frame(&ctx, &mut view, &mut catalog, size, key(egui::Key::F1));
    frame(&ctx, &mut view, &mut catalog, size, vec![]);
    frame(&ctx, &mut view, &mut catalog, size, key(egui::Key::Escape));
    let output = frame(&ctx, &mut view, &mut catalog, size, vec![]);
    assert!(
        view.discovery.selected.is_some(),
        "Help must receive Escape before project navigation"
    );
    assert!(!text(&output, "Using your mod library"));
}
