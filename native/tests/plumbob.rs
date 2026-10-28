use plumcrate::{catalog::Catalog, ui::{self, View}};

fn frame(ctx: &egui::Context, view: &mut View, catalog: &mut Catalog, width: f32) -> egui::FullOutput {
    let mut output = ctx.run_ui(egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(width, 800.0))),
        ..Default::default()
    }, |ui| { view.show(ui, catalog); });
    output.textures_delta.clear();
    output
}

#[test]
fn plumcrate_is_the_visible_application_brand_at_both_navigation_sizes() {
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    let mut catalog = Catalog::default();
    let mut view = View::default();
    for width in [1200.0, 640.0] {
        frame(&ctx, &mut view, &mut catalog, width);
        let output = frame(&ctx, &mut view, &mut catalog, width);
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text) if text.galley.text() == "PlumCrate")), "brand missing at {width}px");
    }
}

#[test]
fn plumbob_navigation_stays_in_the_blue_rail_on_library_and_discovery() {
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    let mut catalog = Catalog::default();
    let mut view = View::default();
    for browse in [false, true] {
        view.browse_page = browse;
        frame(&ctx, &mut view, &mut catalog, 1200.0);
        let output = frame(&ctx, &mut view, &mut catalog, 1200.0);
        let library = view.stats.library_tab.unwrap();
        let discover = view.stats.browse_tab.unwrap();
        assert!(library.right() <= 240.0 && discover.right() <= 240.0, "primary navigation belongs in the rail");
        assert!(discover.top() > library.bottom(), "destinations stack vertically");
        assert!(output.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text() == "Discover")));
    }
}

#[test]
fn plumbob_light_theme_uses_the_approved_cool_surfaces_and_blue_links() {
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    let v = ctx.style_of(egui::Theme::Light);
    assert_eq!(v.visuals.panel_fill, egui::Color32::from_rgb(237, 244, 248));
    assert_eq!(v.visuals.hyperlink_color, egui::Color32::from_rgb(18, 99, 202));
}

#[test]
fn compact_plumbob_navigation_and_search_fit_small_desktop_windows() {
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    let mut catalog = Catalog::default();
    let mut view = View::default();
    for width in [640.0, 680.0] {
        frame(&ctx, &mut view, &mut catalog, width);
        frame(&ctx, &mut view, &mut catalog, width);
        for rect in [view.stats.library_tab.unwrap(), view.stats.browse_tab.unwrap(), view.stats.search.unwrap()] {
            assert!(rect.left() >= 0.0 && rect.right() <= width);
        }
    }
}

#[test]
fn short_windows_keep_mod_names_visible_in_compact_rows() {
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    let mut view = View::default();
    let mut catalog = Catalog::default();
    let generation = catalog.begin_scan("/fixture/game".into());
    catalog.accept_scan(generation, Ok(vec![serde_json::from_value(serde_json::json!({
        "key":"fixture", "name":"Fixture mod", "files":["a.package"], "mod_files":["a.package"],
        "source":"external", "enabled":true, "group_path":[]
    })).unwrap()]));
    let mut visible_name = false;
    for _ in 0..2 {
        let mut output = ctx.run_ui(egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(640.0, 480.0))),
            ..Default::default()
        }, |ui| { view.show(ui, &mut catalog); });
        visible_name = output.shapes.iter().any(|shape| matches!(&shape.shape,
            egui::Shape::Text(text) if text.galley.text() == "Fixture mod" && shape.clip_rect.contains_rect(egui::Rect::from_min_size(text.pos, text.galley.size()))));
        output.textures_delta.clear();
    }
    assert!(visible_name, "the mod name must be inside the visible scroll region");
    let row = view.stats.first_row.expect("a mod remains reachable");
    assert!(row.height() <= 80.0, "short windows use compact rows rather than clipping card titles");
}
