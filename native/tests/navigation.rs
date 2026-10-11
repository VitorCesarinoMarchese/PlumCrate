use ts4_mod_manager_native::{
    catalog::{Catalog, EntryId},
    controls::Controls,
    settings::Settings,
    ui::{self, View},
};

fn frame(
    ctx: &egui::Context,
    view: &mut View,
    controls: &mut Controls,
    catalog: &mut Catalog,
    events: Vec<egui::Event>,
) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            events,
            ..Default::default()
        },
        |ui| {
            view.show_with_controls(ui, catalog, controls);
            controls.show_dialogs(ui.ctx(), catalog);
        },
    );
    output.textures_delta.clear();
    output
}
fn has_text(output: &egui::FullOutput, value: &str) -> bool {
    output
        .shapes
        .iter()
        .any(|s| matches!(&s.shape, egui::epaint::Shape::Text(t) if t.galley.text() == value))
}
#[test]
fn settings_occupies_page_without_library_or_floating_window() {
    let dir = tempfile::tempdir().unwrap();
    let mut controls = Controls::new(
        dir.path().join("managed"),
        dir.path().into(),
        Settings::default(),
        || {},
    )
    .unwrap();
    controls.settings_page = true;
    let mut catalog = Catalog::default();
    let mut view = View::default();
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
    let output = frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
    assert!(has_text(&output, "Appearance"));
    assert!(has_text(&output, "Create custom theme"));
    assert!(!has_text(&output, "Mod library"));
    assert!(!has_text(&output, "Local settings"));
    assert!(view.stats.search.is_none());
}
#[test]
fn right_click_card_selects_target_and_opens_actions_for_it() {
    for list in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let mut controls = Controls::new(
            dir.path().join("managed"),
            dir.path().into(),
            Settings::default(),
            || {},
        )
        .unwrap();
        let mut catalog = Catalog::default();
        let generation = catalog.begin_scan(dir.path().join("game"));
        let mods = ["A target", "Z other"].into_iter().map(|name| serde_json::from_value(serde_json::json!({"key":name,"id":name,"name":name,"files":["a.package"],"mod_files":["a.package"],"source":"managed","enabled":false,"group_path":[]})).unwrap()).collect();
        catalog.accept_scan(generation, Ok(mods));
        catalog.select(1);
        let mut view = View::default();
        let ctx = egui::Context::default();
        ui::setup(&ctx);
        frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
        frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
        if list {
            let pos = view.stats.list_switch.unwrap().center();
            let events = vec![
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
            ];
            frame(&ctx, &mut view, &mut controls, &mut catalog, events);
            frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
        }
        let pos = view.stats.first_row.unwrap().center();
        let events = vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Secondary,
                pressed: true,
                modifiers: Default::default(),
            },
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Secondary,
                pressed: false,
                modifiers: Default::default(),
            },
        ];
        frame(&ctx, &mut view, &mut controls, &mut catalog, events);
        let output = frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
        assert_eq!(catalog.selected, Some(EntryId::Managed("A target".into())));
        assert!(egui::Popup::is_any_open(&ctx));
        assert!(has_text(&output, "Rename"));
        assert!(has_text(&output, "Move to trash"));
    }
}

#[test]
fn custom_theme_roundtrips_and_rejects_bad_colors_or_missing_selection() {
    let raw = r##"{"version":1,"theme":"dark","gameRoots":[],"activeCustomTheme":"Plum","customThemes":[{"name":"Plum","colors":{"accent":"#b18bd0","background":"#19151f","surface":"#251e30","text":"#f6f0ff","mutedText":"#c7b8d8","border":"#53445f"}}]}"##;
    let settings = Settings::parse(raw).expect("legacy-compatible custom theme settings");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("settings.json");
    settings.save(&path).unwrap();
    let loaded = Settings::load(&path).unwrap().export(false).unwrap();
    assert!(loaded.contains("Plum"));
    assert!(loaded.contains("#19151f"));
    assert!(Settings::parse(&raw.replace("#b18bd0", "red")).is_err());
    assert!(
        Settings::parse(&raw.replace(
            "\"activeCustomTheme\":\"Plum\"",
            "\"activeCustomTheme\":\"Missing\""
        ))
        .is_err()
    );
}

fn paired_theme_json() -> serde_json::Value {
    serde_json::json!({
        "name": "Plum",
        "light": {"accent": "#64387A", "background": "#F8F3FA", "surface": "#FFFFFF", "text": "#251B2C", "mutedText": "#67566F", "border": "#DACDE1"},
        "dark": {"accent": "#B18BD0", "background": "#19151F", "surface": "#251E30", "text": "#F6F0FF", "mutedText": "#C7B8D8", "border": "#53445F"}
    })
}

#[test]
fn paired_theme_roundtrips_and_applies_a_different_palette_for_each_mode() {
    let theme =
        ts4_mod_manager_native::settings::CustomTheme::parse(&paired_theme_json().to_string())
            .expect("paired light and dark theme");
    let settings = Settings {
        active_custom_theme: Some(theme.name.clone()),
        custom_themes: vec![theme],
        ..Settings::default()
    };
    let fixture = tempfile::tempdir().unwrap();
    let path = fixture.path().join("settings.json");
    settings.save(&path).unwrap();
    let loaded = Settings::load(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&loaded.export(false).unwrap()).unwrap();
    assert_eq!(json["customThemes"][0], paired_theme_json());
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    ui::apply_settings_theme(&ctx, &loaded);
    for (mode, background, accent) in [
        (egui::Theme::Light, [0xF8, 0xF3, 0xFA], [0x64, 0x38, 0x7A]),
        (egui::Theme::Dark, [0x19, 0x15, 0x1F], [0xB1, 0x8B, 0xD0]),
    ] {
        ctx.set_theme(mode);
        let visuals = &ctx.style_of(mode).visuals;
        assert_eq!(
            visuals.panel_fill,
            egui::Color32::from_rgb(background[0], background[1], background[2])
        );
        assert_eq!(
            visuals.hyperlink_color,
            egui::Color32::from_rgb(accent[0], accent[1], accent[2])
        );
    }
}

#[test]
fn legacy_theme_keeps_its_palette_and_gets_a_companion_version() {
    for mode in ["light", "dark"] {
        let paired = paired_theme_json();
        let legacy = serde_json::json!({"name": "Plum", "colors": paired[mode]});
        let theme =
            ts4_mod_manager_native::settings::CustomTheme::parse(&legacy.to_string()).unwrap();
        let exported = serde_json::to_value(theme).unwrap();
        assert_eq!(exported[mode], paired[mode], "preserve all existing colors");
        assert!(exported[if mode == "dark" { "light" } else { "dark" }].is_object());
        assert!(exported.get("colors").is_none());
    }
}

#[test]
fn paired_theme_requires_two_valid_palettes_and_rejects_mixed_formats() {
    use ts4_mod_manager_native::settings::CustomTheme;
    assert!(CustomTheme::parse(&paired_theme_json().to_string()).is_ok());
    for mode in ["light", "dark"] {
        let mut json = paired_theme_json();
        json[mode]["accent"] = "invalid".into();
        assert!(CustomTheme::parse(&json.to_string()).is_err());
        let mut json = paired_theme_json();
        json.as_object_mut().unwrap().remove(mode);
        assert!(CustomTheme::parse(&json.to_string()).is_err());
    }
    let mut json = paired_theme_json();
    json["colors"] = json["dark"].clone();
    assert!(CustomTheme::parse(&json.to_string()).is_err());
    let mut json = paired_theme_json();
    json["unknown"] = true.into();
    assert!(CustomTheme::parse(&json.to_string()).is_err());
}

#[test]
fn editor_edits_and_saves_both_versions_without_losing_the_other_draft() {
    let dir = tempfile::tempdir().unwrap();
    let theme =
        ts4_mod_manager_native::settings::CustomTheme::parse(&paired_theme_json().to_string())
            .unwrap();
    let settings = Settings {
        theme: ts4_mod_manager_native::settings::Theme::Light,
        active_custom_theme: Some(theme.name.clone()),
        custom_themes: vec![theme],
        ..Settings::default()
    };
    let mut controls = Controls::new(
        dir.path().join("managed"),
        dir.path().into(),
        settings.clone(),
        || {},
    )
    .unwrap();
    controls.settings_page = true;
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    ui::apply_settings_theme(&ctx, &settings);
    let mut catalog = Catalog::default();
    let mut view = View::default();
    for (mode, heading, old, new) in [
        ("Light", "Light colors", "#64387A", "#554466"),
        ("Dark", "Dark colors", "#B18BD0", "#CCAADD"),
    ] {
        frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
        let output = frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
        frame(
            &ctx,
            &mut view,
            &mut controls,
            &mut catalog,
            click_text(&output, mode),
        );
        let output = frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
        assert!(has_text(&output, heading));
        frame(
            &ctx,
            &mut view,
            &mut controls,
            &mut catalog,
            click_text(&output, old),
        );
        let modifiers = egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        };
        frame(
            &ctx,
            &mut view,
            &mut controls,
            &mut catalog,
            vec![
                egui::Event::Key {
                    key: egui::Key::A,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                },
                egui::Event::Text(new.into()),
            ],
        );
        let output = frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
        assert!(has_text(&output, new));
    }
    let output = frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
    frame(
        &ctx,
        &mut view,
        &mut controls,
        &mut catalog,
        click_text(&output, "Light"),
    );
    let output = frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
    assert!(has_text(&output, "#554466"));
    frame(
        &ctx,
        &mut view,
        &mut controls,
        &mut catalog,
        click_text(&output, "Save settings"),
    );
    let saved = Settings::load(&dir.path().join(".config/ts4-mod-manager/settings.json")).unwrap();
    assert_eq!(saved.custom_themes[0].light.accent, "#554466");
    assert_eq!(saved.custom_themes[0].dark.accent, "#CCAADD");
    let restarted = egui::Context::default();
    ui::setup(&restarted);
    ui::apply_settings_theme(&restarted, &saved);
    assert_eq!(
        restarted
            .style_of(egui::Theme::Light)
            .visuals
            .hyperlink_color,
        egui::Color32::from_rgb(0x55, 0x44, 0x66)
    );
    assert_eq!(
        restarted
            .style_of(egui::Theme::Dark)
            .visuals
            .hyperlink_color,
        egui::Color32::from_rgb(0xCC, 0xAA, 0xDD)
    );
}

#[test]
fn reset_changes_only_the_visible_palette_and_copy_exports_both_versions() {
    let dir = tempfile::tempdir().unwrap();
    let settings = Settings {
        theme: ts4_mod_manager_native::settings::Theme::Dark,
        active_custom_theme: Some("Plum".into()),
        custom_themes: vec![
            ts4_mod_manager_native::settings::CustomTheme::parse(&paired_theme_json().to_string())
                .unwrap(),
        ],
        ..Settings::default()
    };
    let mut controls = Controls::new(
        dir.path().join("managed"),
        dir.path().into(),
        settings.clone(),
        || {},
    )
    .unwrap();
    controls.settings_page = true;
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    ui::apply_settings_theme(&ctx, &settings);
    let mut catalog = Catalog::default();
    let mut view = View::default();
    let mut render = |events| {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1200.0, 1100.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                view.show_with_controls(ui, &mut catalog, &mut controls);
            },
        );
        output.textures_delta.clear();
        output
    };
    render(vec![]);
    let output = render(vec![]);
    render(click_text(&output, "Reset colors"));
    let output = render(vec![]);
    let copied = render(click_text(&output, "Copy theme JSON"));
    let json = copied
        .platform_output
        .commands
        .iter()
        .find_map(|command| match command {
            egui::OutputCommand::CopyText(text) => {
                Some(serde_json::from_str::<serde_json::Value>(text).unwrap())
            }
            _ => None,
        })
        .expect("copy a paired theme");
    assert_eq!(json["light"], paired_theme_json()["light"]);
    assert_eq!(
        json["dark"],
        serde_json::to_value(ts4_mod_manager_native::settings::ThemeColors::base(true)).unwrap()
    );
    let output = render(vec![]);
    render(click_text(&output, "Save settings"));
    let saved = Settings::load(&dir.path().join(".config/ts4-mod-manager/settings.json")).unwrap();
    assert_eq!(serde_json::to_value(&saved.custom_themes[0]).unwrap(), json);
}

fn click_text(output: &egui::FullOutput, label: &str) -> Vec<egui::Event> {
    let pos = output
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::epaint::Shape::Text(t) if t.galley.text() == label => {
                Some(t.pos + t.galley.rect.center().to_vec2())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("Missing {label}"));
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
#[test]
fn theme_editor_saves_and_applies_a_theme_that_survives_restart() {
    let dir = tempfile::tempdir().unwrap();
    let mut controls = Controls::new(
        dir.path().join("managed"),
        dir.path().into(),
        Settings::default(),
        || {},
    )
    .unwrap();
    controls.settings_page = true;
    let mut catalog = Catalog::default();
    let mut view = View::default();
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
    let output = frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
    frame(
        &ctx,
        &mut view,
        &mut controls,
        &mut catalog,
        click_text(&output, "Create custom theme"),
    );
    let output = frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
    frame(
        &ctx,
        &mut view,
        &mut controls,
        &mut catalog,
        click_text(&output, "Save settings"),
    );
    let settings =
        Settings::load(&dir.path().join(".config/ts4-mod-manager/settings.json")).unwrap();
    assert_eq!(
        settings.active_custom_theme.as_deref(),
        Some("Custom theme 1")
    );
    assert_eq!(settings.custom_themes.len(), 1);
    let restarted = egui::Context::default();
    ui::setup(&restarted);
    ui::apply_settings_theme(&restarted, &settings);
    assert_eq!(
        restarted.style_of(egui::Theme::Dark).visuals.panel_fill,
        ctx.style_of(egui::Theme::Dark).visuals.panel_fill
    );
    let builtin = Settings::default();
    ui::apply_settings_theme(&restarted, &builtin);
    assert!(Settings::parse(&settings.export(false).unwrap()).is_ok());
}
#[test]
fn settings_save_button_stays_visible_above_scrolling_content() {
    let dir = tempfile::tempdir().unwrap();
    let mut controls = Controls::new(
        dir.path().join("managed"),
        dir.path().into(),
        Settings::default(),
        || {},
    )
    .unwrap();
    controls.settings_page = true;
    let mut catalog = Catalog::default();
    let mut view = View::default();
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
    let output = frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
    assert!(has_text(&output, "Save settings"));
    frame(
        &ctx,
        &mut view,
        &mut controls,
        &mut catalog,
        click_text(&output, "Save settings"),
    );
    frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
    let output = frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
    assert!(
        has_text(&output, "Settings saved"),
        "saved file={}, text={:?}",
        dir.path()
            .join(".config/ts4-mod-manager/settings.json")
            .exists(),
        output
            .shapes
            .iter()
            .filter_map(|s| match &s.shape {
                egui::epaint::Shape::Text(t) => Some(t.galley.text()),
                _ => None,
            })
            .collect::<Vec<_>>()
    );
}

#[test]
fn settings_transfer_remains_compatible_with_the_previous_app() {
    let mut settings = Settings::default();
    let theme = ts4_mod_manager_native::settings::CustomTheme::base();
    settings.active_custom_theme = Some(theme.name.clone());
    settings.custom_themes.push(theme);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("transfer.json");
    settings.save_new(&path).unwrap();
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    assert!(json.get("customThemes").is_none());
    assert!(json.get("activeCustomTheme").is_none());
}

#[test]
fn color_mode_switch_preserves_the_custom_editor_draft() {
    let dir = tempfile::tempdir().unwrap();
    let mut controls = Controls::new(
        dir.path().join("managed"),
        dir.path().into(),
        Settings::default(),
        || {},
    )
    .unwrap();
    controls.settings_page = true;
    let mut catalog = Catalog::default();
    let mut view = View::default();
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
    let output = frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
    frame(
        &ctx,
        &mut view,
        &mut controls,
        &mut catalog,
        click_text(&output, "Create custom theme"),
    );
    controls.set_color_mode(&ctx, ts4_mod_manager_native::settings::Theme::Light);
    let output = frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
    frame(
        &ctx,
        &mut view,
        &mut controls,
        &mut catalog,
        click_text(&output, "Save settings"),
    );
    assert!(
        Settings::load(&dir.path().join(".config/ts4-mod-manager/settings.json"))
            .unwrap()
            .active_custom_theme
            .is_some()
    );
    let output = frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
    frame(&ctx, &mut view, &mut controls, &mut catalog, click_text(&output, "Use default colors"));
    let output = frame(&ctx, &mut view, &mut controls, &mut catalog, vec![]);
    frame(&ctx, &mut view, &mut controls, &mut catalog, click_text(&output, "Save settings"));
    let saved = Settings::load(&dir.path().join(".config/ts4-mod-manager/settings.json")).unwrap();
    assert!(saved.active_custom_theme.is_none());
    assert_eq!(saved.custom_themes.len(), 1);

}
