use plumcrate::{
    catalog::Catalog,
    controls::Controls,
    settings::{CustomTheme, Settings, Theme},
    ui::{self, View},
};

#[test]
fn custom_palettes_reach_sidebar_primary_actions_and_mod_switches() {
    let theme = CustomTheme::parse(r##"{"name":"Plum","light":{"accent":"#64387A","background":"#F8F3FA","surface":"#FFFFFF","text":"#251B2C","mutedText":"#67566F","border":"#DACDE1"},"dark":{"accent":"#B18BD0","background":"#19151F","surface":"#251E30","text":"#F6F0FF","mutedText":"#C7B8D8","border":"#53445F"}}"##).unwrap();
    for (mode, dark) in [(Theme::Light, false), (Theme::Dark, true)] {
        let dir = tempfile::tempdir().unwrap();
        let settings = Settings {
            theme: mode,
            active_custom_theme: Some(theme.name.clone()),
            custom_themes: vec![theme.clone()],
            ..Default::default()
        };
        let ctx = egui::Context::default();
        ui::setup(&ctx);
        ui::apply_settings_theme(&ctx, &settings);
        let mut controls = Controls::new(
            dir.path().join("managed"),
            dir.path().into(),
            settings,
            || {},
        )
        .unwrap();
        let mut catalog = Catalog::default();
        let generation = catalog.begin_scan(dir.path().join("game"));
        catalog.accept_scan(
            generation,
            Ok(vec![
                serde_json::from_value(serde_json::json!({
                    "key":"fixture", "id":"fixture", "name":"Fixture", "files":["a.package"],
                    "mod_files":["a.package"], "source":"managed", "enabled":true, "group_path":[]
                }))
                .unwrap(),
            ]),
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while controls.busy() {
            controls.poll(&mut catalog);
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let mut view = View::default();
        let mut output = None;
        for _ in 0..2 {
            let mut frame = ctx.run_ui(
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
            frame.textures_delta.clear();
            output = Some(frame);
        }
        let output = output.unwrap();
        let colors = theme.palette(dark);
        let surface = ui::hex_color(&colors.surface).unwrap();
        let accent = ui::hex_color(&colors.accent).unwrap();
        let rects: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::Shape::Rect(r) => Some(r),
                _ => None,
            })
            .collect();
        let rail = rects
            .iter()
            .find(|r| r.rect.width() >= 239.0 && r.rect.width() <= 241.0 && r.rect.height() > 700.0)
            .expect("sidebar");
        assert_eq!(
            rail.fill, surface,
            "sidebar must use the selected custom palette"
        );
        let switch = view.stats.first_toggle.unwrap();
        assert!(
            rects
                .iter()
                .any(|r| switch.contains_rect(r.rect) && r.fill == accent),
            "enabled switch must use custom accent"
        );
        let add = output
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == "Add mods" => Some(t.pos),
                _ => None,
            })
            .unwrap();
        assert!(
            rects
                .iter()
                .any(|r| r.rect.contains(add) && r.fill == accent),
            "Add mods must use custom accent"
        );
        let builtin = Settings {
            theme: if dark { Theme::Dark } else { Theme::Light },
            ..Default::default()
        };
        ui::apply_settings_theme(&ctx, &builtin);
        let mut reset = ctx.run_ui(
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
        reset.textures_delta.clear();
        let builtin_rail = if dark { egui::Color32::from_rgb(28, 31, 38) } else { egui::Color32::from_rgb(8, 99, 185) };
        assert!(reset.shapes.iter().any(|s| matches!(&s.shape, egui::Shape::Rect(r) if r.rect.width() >= 239.0 && r.rect.width() <= 241.0 && r.rect.height() > 700.0 && r.fill == builtin_rail)), "switching back must restore the built-in rail for the current mode");
    }
}

#[test]
fn a_valid_earth_palette_keeps_its_colors_even_when_text_needs_adjustment() {
    let mut theme = CustomTheme::base();
    theme.light.accent = "#8A6F3E".into();
    theme.light.background = "#F3F0E8".into();
    theme.light.surface = "#FFFFFF".into();
    theme.light.text = "#282B29".into();
    theme.light.muted_text = "#6F7774".into();
    theme.light.border = "#A38E5A".into();
    assert!(!theme.light.contrast_warnings().is_empty());
    let settings = Settings { theme: Theme::Light, active_custom_theme: Some(theme.name.clone()), custom_themes: vec![theme.clone()], ..Default::default() };
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    ui::apply_settings_theme(&ctx, &settings);
    let v = ctx.style_of(egui::Theme::Light);
    assert_eq!(v.visuals.panel_fill, ui::hex_color(&theme.light.background).unwrap(), "a contrast warning must not discard the entire theme");
    assert!(settings.custom_themes[0] == theme, "readability adjustments must not overwrite saved colors");
}
