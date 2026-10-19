use ts4_mod_manager_native::{
    catalog::Catalog,
    controls::Controls,
    settings::{CustomTheme, Settings, Theme},
    ui::{self, View},
};
struct Fixture {
    _dir: tempfile::TempDir,
    path: std::path::PathBuf,
    controls: Controls,
    catalog: Catalog,
    view: View,
    ctx: egui::Context,
}
impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let theme = CustomTheme::base();
        let settings = Settings {
            theme: Theme::Light,
            active_custom_theme: Some(theme.name.clone()),
            custom_themes: vec![theme],
            ..Default::default()
        };
        let path = dir.path().join(".config/ts4-mod-manager/settings.json");
        settings.save(&path).unwrap();
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
        controls.settings_page = true;
        Self {
            _dir: dir,
            path,
            controls,
            catalog: Catalog::default(),
            view: View::default(),
            ctx,
        }
    }
    fn frame(&mut self, events: Vec<egui::Event>) -> egui::FullOutput {
        let mut out = self.ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 800.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                self.view
                    .show_with_controls(ui, &mut self.catalog, &mut self.controls);
            },
        );
        out.textures_delta.clear();
        out
    }
    fn settle(&mut self) -> egui::FullOutput {
        self.frame(vec![]);
        self.frame(vec![])
    }
    fn click(&mut self, name: &str) {
        let out = self.settle();
        let pos = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == name => {
                    Some(t.pos + t.galley.size() / 2.0)
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("missing {name}"));
        self.frame(vec![
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
        ]);
    }
}
#[test]
fn settings_changes_are_staged_until_the_single_save_action() {
    let mut f = Fixture::new();
    f.click("Dark");
    assert!(f.controls.settings.theme == Theme::Dark);
    assert_eq!(
        f.ctx.theme(),
        egui::Theme::Light,
        "choosing a draft must not apply it to the app"
    );
    assert!(Settings::load(&f.path).unwrap().theme == Theme::Light);
    f.click("Save changes");
    assert!(Settings::load(&f.path).unwrap().theme == Theme::Dark);
    assert_eq!(f.ctx.theme(), egui::Theme::Dark);
    assert!(!f.controls.settings_dirty());
}
#[test]
fn removing_a_theme_can_be_discarded_or_saved() {
    let mut f = Fixture::new();
    f.click("Remove theme");
    assert!(f.controls.settings.custom_themes.is_empty());
    assert_eq!(
        Settings::load(&f.path).unwrap().custom_themes.len(),
        1,
        "removal remains a draft"
    );
    f.click("Discard changes");
    assert_eq!(f.controls.settings.custom_themes.len(), 1);
    f.click("Remove theme");
    f.click("Save changes");
    let saved = Settings::load(&f.path).unwrap();
    assert!(saved.custom_themes.is_empty());
    assert!(saved.active_custom_theme.is_none());
}
#[test]
fn editing_a_color_version_keeps_the_window_mode_and_fits_the_hex_value() {
    let mut f = Fixture::new();
    f.click("Edit colors");
    f.click("Dark colors");
    assert!(f.controls.settings.theme == Theme::Light);
    assert_eq!(f.ctx.theme(), egui::Theme::Light);
    let out = f.settle();
    let hex = out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == "#9BC7FF" => Some((
                s.clip_rect,
                egui::Rect::from_min_size(t.pos, t.galley.size()),
            )),
            _ => None,
        })
        .expect("the complete hex value is visible");
    assert!(hex.0.contains_rect(hex.1), "hex values must not be clipped");
    assert!(!f.controls.settings_dirty());
}
#[test]
fn quick_appearance_switch_does_not_apply_a_theme_draft() {
    let mut f = Fixture::new();
    let saved_surface = f.ctx.style_of(egui::Theme::Dark).visuals.panel_fill;
    f.controls.settings.custom_themes[0].dark.background = "#123456".into();
    f.controls.set_color_mode(&f.ctx, Theme::Dark);
    assert_eq!(
        f.ctx.style_of(egui::Theme::Dark).visuals.panel_fill,
        saved_surface
    );
    assert!(f.controls.settings_dirty());
    assert_ne!(
        Settings::load(&f.path).unwrap().custom_themes[0]
            .dark
            .background,
        "#123456"
    );
}
#[test]
fn transparent_choices_keep_readable_ink_on_a_black_page() {
    let mut f = Fixture::new();
    let mut theme = f.controls.settings.custom_themes[0].clone();
    theme.light.background = "#000000".into();
    theme.light.surface = "#FFFFFF".into();
    theme.light.text = "#000000".into();
    f.controls.settings.custom_themes = vec![theme];
    ui::apply_settings_theme(&f.ctx, &f.controls.settings);
    let out = f.settle();
    for label in ["Folders", "Connections", "Dark", "System", "Built-in"] {
        let ink = out
            .shapes
            .iter()
            .find_map(|s| match &s.shape {
                egui::Shape::Text(t) if t.galley.text() == label => Some(t.fallback_color),
                _ => None,
            })
            .unwrap();
        assert_ne!(
            ink,
            egui::Color32::BLACK,
            "{label} must remain visible on black"
        );
    }
}
