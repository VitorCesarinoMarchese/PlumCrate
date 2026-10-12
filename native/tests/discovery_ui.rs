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
fn pending_search_reserves_cards_without_exposing_download_actions() {
    for size in [[1200.0, 800.0], [640.0, 480.0]] {
        let ctx = egui::Context::default();
        ui::setup(&ctx);
        let mut view = View::default();
        filter_fixture(&mut view);
        view.discovery.browser.begin(BrowseQuery::default());
        let mut catalog = Catalog::default();
        let output = frame(&ctx, &mut view, &mut catalog, size, vec![]);
        assert!(text(&output, "Searching CurseForge"));
        assert!(view.discovery.stats.skeleton_cards > 0);
        assert!(view.discovery.stats.card_actions.is_empty());
        assert!(view.discovery.stats.browser_import.is_none());
        assert!(view.discovery.selected.is_none());
        let cards=output.shapes.iter().filter_map(|shape|match &shape.shape {
            egui::epaint::Shape::Rect(rect) if rect.rect.height()>250.0 && rect.rect.height()<300.0 && rect.stroke.width>0.0 && rect.fill==ctx.style_of(ctx.theme()).visuals.extreme_bg_color=>Some(rect.rect),
            _=>None,
        }).collect::<Vec<_>>();
        assert!(!cards.is_empty());
        for rect in cards {assert!(rect.left()>=24.0 && rect.right()<=size[0]-23.0,"placeholder card overflow: {rect:?}");}
    }
}

#[test]
fn initial_catalog_scan_reserves_library_cards() {
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    let mut view = View::default();
    let mut catalog = Catalog::default();
    catalog.begin_scan(std::path::PathBuf::from("/synthetic/game"));
    let output = frame(&ctx, &mut view, &mut catalog, [1200.0, 800.0], vec![]);
    assert!(text(&output, "Reading your catalog"));
    assert!(view.stats.skeleton_cards > 0);
    assert_eq!(view.stats.rows, 0);
}

fn filter_fixture(view: &mut View) {
    view.browse_page = true;
    view.discovery.categories = ts4_mod_manager_core::curseforge_browse::parse_categories_response(
        r#"{"data":[
        {"id":10,"gameId":78062,"name":"Create a Sim","isClass":true},
        {"id":11,"gameId":78062,"name":"Clothing","parentCategoryId":10,"classId":10},
        {"id":13,"gameId":78062,"name":"Adult","parentCategoryId":11,"classId":10},
        {"id":20,"gameId":78062,"name":"Build / Buy","isClass":true},
        {"id":21,"gameId":78062,"name":"Lighting","parentCategoryId":20,"classId":20}
    ]}"#,
    )
    .unwrap();
    view.discovery.game_versions = vec!["1.128.90".into(), "1.127.41".into()];
    view.discovery.class_id = Some(10);
    view.discovery.category = Some(13);
    view.discovery.game_version = Some("1.128.90".into());
    let generation = view.discovery.browser.begin(BrowseQuery {
        class_id: Some(10),
        category: Some(13),
        game_version: Some("1.128.90".into()),
        index: 24,
        ..Default::default()
    });
    view.discovery.browser.accept(generation, Ok(parse_browse_response(r#"{"data":[],"pagination":{"index":24,"pageSize":24,"resultCount":0,"totalCount":24}}"#).unwrap()));
}

#[test]
fn curseforge_filters_fit_and_show_the_category_parent_at_every_size() {
    for size in [
        [1870.0, 1006.0],
        [1200.0, 800.0],
        [680.0, 760.0],
        [640.0, 480.0],
    ] {
        let ctx = egui::Context::default();
        ui::setup(&ctx);
        let mut view = View::default();
        filter_fixture(&mut view);
        let mut catalog = Catalog::default();
        for _ in 0..3 {
            frame(&ctx, &mut view, &mut catalog, size, vec![]);
        }
        if let Some(toggle) = view.discovery.stats.filters_toggle {
            frame(&ctx, &mut view, &mut catalog, size, click(toggle));
            for _ in 0..4 {
                frame(&ctx, &mut view, &mut catalog, size, vec![]);
            }
        }
        let output = frame(&ctx, &mut view, &mut catalog, size, vec![]);
        assert!(text(&output, "Browse by"));
        assert!(text(&output, "Game Version"));
        assert!(text(&output, "Clothing / Adult"));
        for rect in [
            view.discovery.stats.browse_by,
            view.discovery.stats.category,
            view.discovery.stats.game_version,
            view.discovery.stats.sort,
            view.discovery.stats.clear_filters,
        ]
        .into_iter()
        .flatten()
        {
            assert!(
                rect.left() >= 0.0 && rect.right() <= size[0],
                "filter overflow at {size:?}: {rect:?}"
            );
        }
    }
}

#[test]
fn switching_content_type_clears_category_keeps_version_and_restarts_pagination() {
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    let mut view = View::default();
    filter_fixture(&mut view);
    let mut catalog = Catalog::default();
    let size = [1200.0, 800.0];
    for _ in 0..3 {
        frame(&ctx, &mut view, &mut catalog, size, vec![]);
    }
    let rect = view.discovery.stats.browse_by.unwrap();
    frame(&ctx, &mut view, &mut catalog, size, click(rect));
    frame(&ctx, &mut view, &mut catalog, size, vec![]);
    let target = view
        .discovery
        .stats
        .content_types
        .iter()
        .find(|(id, _)| *id == 20)
        .unwrap()
        .1;
    frame(&ctx, &mut view, &mut catalog, size, click(target));
    assert_eq!(view.discovery.class_id, Some(20));
    assert_eq!(view.discovery.category, None);
    assert_eq!(view.discovery.browser.active.class_id, Some(20));
    assert_eq!(view.discovery.browser.active.category, None);
    assert_eq!(
        view.discovery.browser.active.game_version.as_deref(),
        Some("1.128.90")
    );
    assert_eq!(view.discovery.browser.active.index, 0);
}

#[test]
fn clearing_filters_keeps_search_text_and_resets_all_filter_dimensions() {
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    let mut view = View::default();
    filter_fixture(&mut view);
    view.discovery.query = "hair".into();
    let mut catalog = Catalog::default();
    let size = [1200.0, 800.0];
    for _ in 0..3 {
        frame(&ctx, &mut view, &mut catalog, size, vec![]);
    }
    let rect = view.discovery.stats.clear_filters.unwrap();
    frame(&ctx, &mut view, &mut catalog, size, click(rect));
    assert_eq!(view.discovery.class_id, None);
    assert_eq!(view.discovery.category, None);
    assert_eq!(view.discovery.game_version, None);
    assert_eq!(view.discovery.browser.active.search, "hair");
    assert_eq!(view.discovery.browser.active.index, 0);
}

#[test]
fn selecting_category_version_and_sort_submits_combined_filters() {
    fn text_rect(output: &egui::FullOutput, value: &str) -> egui::Rect {
        output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) if text.galley.text() == value => {
                    Some(egui::Rect::from_min_size(text.pos, text.galley.size()))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing selectable option {value}"))
    }
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    let mut view = View::default();
    filter_fixture(&mut view);
    view.discovery.category = None;
    view.discovery.query = "collection".into();
    let mut catalog = Catalog::default();
    let size = [1200.0, 800.0];
    for _ in 0..3 {
        frame(&ctx, &mut view, &mut catalog, size, vec![]);
    }
    for (control, label) in [
        (0, "Clothing / Adult"),
        (1, "1.127.41"),
        (2, "Date released"),
    ] {
        let rect = match control {
            0 => view.discovery.stats.category,
            1 => view.discovery.stats.game_version,
            _ => view.discovery.stats.sort,
        }
        .unwrap();
        frame(&ctx, &mut view, &mut catalog, size, click(rect));
        let output = frame(&ctx, &mut view, &mut catalog, size, vec![]);
        let option = text_rect(&output, label);
        frame(&ctx, &mut view, &mut catalog, size, click(option));
        frame(&ctx, &mut view, &mut catalog, size, vec![]);
        assert_eq!(view.discovery.browser.active.index, 0);
    }
    let query = &view.discovery.browser.active;
    assert_eq!(query.search, "collection");
    assert_eq!(query.class_id, Some(10));
    assert_eq!(query.category, Some(13));
    assert_eq!(query.game_version.as_deref(), Some("1.127.41"));
    assert_eq!(
        query.sort,
        ts4_mod_manager_core::curseforge_browse::BrowseSort::Created
    );
}

#[test]
fn short_window_filter_disclosure_scrolls_to_clear_action() {
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    let mut view = View::default();
    filter_fixture(&mut view);
    let mut catalog = Catalog::default();
    let size = [640.0, 480.0];
    for _ in 0..3 {
        frame(&ctx, &mut view, &mut catalog, size, vec![]);
    }
    let toggle = view.discovery.stats.filters_toggle.unwrap();
    frame(&ctx, &mut view, &mut catalog, size, click(toggle));
    for _ in 0..4 {
        frame(&ctx, &mut view, &mut catalog, size, vec![]);
    }
    frame(
        &ctx,
        &mut view,
        &mut catalog,
        size,
        vec![
            egui::Event::PointerMoved(egui::pos2(320.0, 330.0)),
            egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -180.0),
                modifiers: Default::default(),
                phase: egui::TouchPhase::Move,
            },
        ],
    );
    for _ in 0..5 {
        frame(&ctx, &mut view, &mut catalog, size, vec![]);
    }
    let clear = view.discovery.stats.clear_filters.unwrap();
    assert!(
        clear.center().y > 105.0 && clear.center().y < 409.0,
        "clear action must be reachable after scrolling: {clear:?}"
    );
    frame(&ctx, &mut view, &mut catalog, size, click(clear));
    assert_eq!(view.discovery.browser.active.class_id, None);
    assert_eq!(view.discovery.browser.active.category, None);
    assert_eq!(view.discovery.browser.active.game_version, None);
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
