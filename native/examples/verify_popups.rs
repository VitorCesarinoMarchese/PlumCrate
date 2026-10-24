//! Capture production review popups using synthetic data, without approving any operation.
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use ts4_mod_manager_native::{app_update, catalog, discovery, management, preview, settings, updates, browser_import, project_details};
#[allow(dead_code)]
#[path="../src/skeleton.rs"]
mod skeleton;

#[allow(dead_code)]
#[path = "../src/ui.rs"]
mod ui;

#[path = "../src/file_picker.rs"]
mod file_picker;
#[path = "../src/popup.rs"]
mod popup;

// Compile the actual controller so captures can stage its private review states.
#[allow(dead_code)]
mod controls {
    include!("../src/controls.rs");

    pub fn stage(controls: &mut Controls, catalog: &Catalog, case: usize, source: PathBuf) {
        controls.dialog = None;
        controls.candidates = None;
        controls.trash_entries = None;
        controls.settings_page = (17..=19).contains(&case);
        controls.settings = Settings::default();
        controls.theme_edit = None;
        controls.theme_edit_original = None;
        let identity = controls.identity(catalog).unwrap();
        match case {
            0..=3 => controls.picked(PickPurpose::Import(identity, case != 3), Some(if case == 3 { source.with_extension("") } else { source }), catalog),
            4 => controls.dialog = Some(Dialog::Rename(identity, "Synthetic mod collection".into())),
            5 => controls.dialog = Some(Dialog::Manual(identity, String::new())),
            6 => controls.confirm(catalog, Action::RemoveSource, "Remove source", (0..40).map(|index| format!("Synthetic review line {index}: metadata is preserved until you confirm.")).collect()),
            7 => controls.dialog = Some(Dialog::SettingsImport(Settings::default(), false)),
            8 => controls.candidates = Some((identity, vec![])),
            9 => controls.trash_entries = Some((identity, vec![])),
            10..=16 | 20..=23 => {},
            17..=18 => {
                let mut theme = crate::settings::CustomTheme::base();
                theme.name = "Contrast check".into();
                theme.light.text = theme.light.surface.clone();
                theme.dark.text = theme.dark.surface.clone();
                controls.settings.active_custom_theme = Some(theme.name.clone());
                controls.settings.custom_themes = vec![theme.clone()];
                controls.theme_edit = Some(theme);
            },
            24 => {
                let mut theme = crate::settings::CustomTheme::base();
                theme.name = "Synthetic amber palette".into();
                theme.dark = crate::settings::ThemeColors {
                    accent: "#F3CC3E".into(), background: "#241C0D".into(), surface: "#483B18".into(),
                    text: "#F4EED9".into(), muted_text: "#C4B890".into(), border: "#75633A".into(),
                };
                assert!(theme.dark.contrast_warnings().is_empty());
                controls.settings.active_custom_theme = Some(theme.name.clone());
                controls.settings.custom_themes = vec![theme];
            },
            19 => {
                controls.settings.game_roots.push(catalog.root.as_ref().unwrap().display().to_string());
            },
            _ => unreachable!(),
        }
    }

    pub fn assert_unapproved(controls: &Controls) {
        assert!(controls.operations.is_empty());
        assert!(!controls.settings_path.exists());
    }
}

const CASES: [(&str, bool, [f32; 2]); 25] = [
    ("import-zip-light", false, [1200.0, 800.0]),
    ("import-zip-dark", true, [1200.0, 800.0]),
    ("import-zip-small", false, [640.0, 480.0]),
    ("import-folder-narrow", true, [680.0, 760.0]),
    ("rename-dark", true, [1200.0, 800.0]),
    ("manual-source-light", false, [1200.0, 800.0]),
    ("confirmation-long", false, [680.0, 760.0]),
    ("settings-import-dark", true, [1200.0, 800.0]),
    ("source-empty-light", false, [1200.0, 800.0]),
    ("trash-empty-dark", true, [1200.0, 800.0]),
    ("mod-details-light", false, [1200.0, 800.0]),
    ("mod-details-dark", true, [1200.0, 800.0]),
    ("mod-details-narrow", true, [680.0, 760.0]),
    ("mod-details-small", false, [640.0, 480.0]),
    ("mod-details-long", true, [680.0, 760.0]),
    ("help-light", false, [1200.0, 800.0]),
    ("help-small-dark", true, [640.0, 480.0]),
    ("theme-contrast-light", false, [680.0, 760.0]),
    ("theme-contrast-small-dark", true, [640.0, 480.0]),
    ("settings-draft-light", false, [1200.0, 800.0]),
    ("compact-no-previews", false, [680.0, 760.0]),
    ("mod-details-square-light", false, [1200.0, 800.0]),
    ("mod-details-square-dark", true, [680.0, 760.0]),
    ("mod-details-one-file", true, [1200.0, 800.0]),
    ("mod-details-one-file-amber", true, [680.0, 800.0]),
];

struct Capture {
    controls: controls::Controls,
    catalog: catalog::Catalog,
    view: ui::View,
    source: PathBuf,
    directory: PathBuf,
    case: usize,
    stable_frames: u8,
    requested: bool,
    opened: bool,
    started: Instant,
    staged: Instant,
}

impl Capture {
    fn stage(&mut self, ctx: &egui::Context) {
        let (_, dark, size) = CASES[self.case];
        ctx.set_theme(if dark {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        });
        for command in [
            egui::ViewportCommand::MinInnerSize(size.into()),
            egui::ViewportCommand::MaxInnerSize(size.into()),
            egui::ViewportCommand::InnerSize(size.into()),
        ] {
            ctx.send_viewport_cmd(command);
        }
        controls::stage(
            &mut self.controls,
            &self.catalog,
            self.case,
            self.source.clone(),
        );
        if self.case == 24 {
            ui::apply_settings_theme(ctx, &self.controls.settings);
            ctx.set_theme(egui::Theme::Dark);
        }
        self.view = ui::View::default();
        self.view.scan_ms = Some(0.0);
        self.opened = (17..=20).contains(&self.case);
        if self.case >= 21 { self.catalog.mods[0].preview = Some(self.source.with_file_name("square-preview.png").display().to_string()); }
        if self.case >= 23 {
            self.catalog.mods[0].files = vec!["Synthetic collection.package".into()];
            self.catalog.mods[0].source_attachment = Some(serde_json::from_value(serde_json::json!({
                "providerId":"fixture", "sourceUrl":"https://example.com/synthetic-collection", "title":"Synthetic collection",
                "author":"Synthetic creator", "attachedBy":"user", "attachedAt":"2026-10-11T00:00:00Z"
            })).unwrap());
        }
        if self.case == 20 { for entry in &mut self.catalog.mods { entry.preview = None; } }
        self.catalog.mods[0].name = if self.case == 14 {
            "Synthetic collection with a very long name, release date, and additional pack details"
                .into()
        } else {
            "A synthetic collection".into()
        };
        self.stable_frames = 0;
        self.requested = false;
        self.staged = Instant::now();
    }
}

impl eframe::App for Capture {
    fn raw_input_hook(&mut self, _ctx: &egui::Context, input: &mut egui::RawInput) {
        if (15..=16).contains(&self.case) && !self.opened && self.stable_frames >= 2 {
            input.events.push(egui::Event::Key { key: egui::Key::F1, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE });
            self.opened = true;
            self.stable_frames = 0;
            self.staged = Instant::now();
        }
        if ((10..=14).contains(&self.case) || (21..=24).contains(&self.case))
            && !self.opened
            && self.stable_frames >= 2
            && let Some(row) = self.view.stats.first_row
        {
            let pos = row.center();
            input.events.extend([
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
            self.opened = true;
            self.stable_frames = 0;
            self.staged = Instant::now();
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.controls.poll(&mut self.catalog);
        self.view
            .show_with_controls(ui, &mut self.catalog, &mut self.controls);
        self.controls.show_dialogs(&ctx, &self.catalog);
        if let Some(image) = ctx.input(|input| {
            input.events.iter().find_map(|event| {
                if let egui::Event::Screenshot { image, .. } = event {
                    Some(image.clone())
                } else {
                    None
                }
            })
        }) {
            let bytes = image
                .pixels
                .iter()
                .flat_map(|pixel| pixel.to_array())
                .collect::<Vec<_>>();
            if (10..=14).contains(&self.case) || (21..=24).contains(&self.case) {
                let preview_pixels = image.pixels.iter().filter(|pixel| pixel.to_array() == [30, 120, 95, 255]).count();
                assert!(preview_pixels >= 15_000, "details preview must remain readable, including in a short window");
            }
            let path = self.directory.join(format!("{}.png", CASES[self.case].0));
            let size = CASES[self.case].2;
            assert_eq!(image.size, [size[0] as usize, size[1] as usize]);
            image::save_buffer(
                path,
                &bytes,
                image.size[0] as u32,
                image.size[1] as u32,
                image::ColorType::Rgba8,
            )
            .unwrap();
            controls::assert_unapproved(&self.controls);
            self.case += 1;
            if self.case == CASES.len() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            self.stage(&ctx);
        } else if !self.requested
            && ctx.content_rect().size() == egui::Vec2::from(CASES[self.case].2)
        {
            self.stable_frames += 1;
            if self.stable_frames >= 5
                && self.staged.elapsed() >= Duration::from_millis(350)
                && !ctx.has_pending_images()
                && (self.case < 10 || self.opened)
            {
                if (10..=14).contains(&self.case) || (21..=24).contains(&self.case) {
                    let close = self.view.stats.detail_close.expect("details must actually open before capture");
                    assert!(ctx.content_rect().contains_rect(close));
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
                self.requested = true;
            }
        }
        assert!(
            self.started.elapsed() < Duration::from_secs(60),
            "Popup capture timed out"
        );
        ctx.request_repaint_after(Duration::from_millis(40));
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let directory = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("Provide a capture directory")?,
    );
    std::fs::create_dir_all(&directory)?;
    let fixture = tempfile::tempdir()?;
    let root = fixture.path().join("The Sims 4");
    std::fs::create_dir_all(root.join("Mods"))?;
    let source = fixture.path().join("Downloads").join("Synthetic very long archive name with a release date and extra collection details (461347).zip");
    std::fs::create_dir_all(source.parent().unwrap())?;
    std::fs::write(&source, b"synthetic capture fixture, never imported")?;
    image::RgbaImage::from_pixel(320, 320, image::Rgba([30, 120, 95, 255])).save(source.with_file_name("square-preview.png"))?;
    std::fs::create_dir_all(source.with_extension(""))?;
    let source_check = source.clone();
    let mut catalog = catalog::Catalog::default();
    let generation = catalog.begin_scan(root);
    let preview = fixture.path().join("preview.png");
    image::RgbaImage::from_pixel(320, 180, image::Rgba([30, 120, 95, 255])).save(&preview)?;
    let entries = ["A synthetic collection", "B another collection", "C collection without preview"]
        .into_iter().enumerate().map(|(index, name)| serde_json::from_value(serde_json::json!({
            "key":name, "id":name, "name":name,
            "files": (0..16).map(|n| format!("Synthetic pack/file{n:02}.package")).collect::<Vec<_>>(),
            "mod_files":["Synthetic pack/file00.package"], "source":"managed", "enabled":false,
            "group_path":[], "preview": if index < 2 { preview.to_str() } else { None },
            "sourceUrl":"https://example.com/synthetic-collection"
        })).unwrap()).collect();
    catalog.accept_scan(generation, Ok(entries));
    let controls = controls::Controls::new(
        fixture.path().join("managed"),
        fixture.path().join("home"),
        settings::Settings::default(),
        || {},
    )?;
    eframe::run_native(
        "Review popup verification",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size(CASES[0].2)
                .with_resizable(false),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| {
            ui::setup(&cc.egui_ctx);
            let mut app = Capture {
                controls,
                catalog,
                view: ui::View::default(),
                source,
                directory,
                case: 0,
                stable_frames: 0,
                requested: false,
                opened: false,
                started: Instant::now(),
                staged: Instant::now(),
            };
            app.stage(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )?;
    assert_eq!(
        std::fs::read(source_check)?,
        b"synthetic capture fixture, never imported"
    );
    println!("{} production popup captures; no operations approved or settings saved", CASES.len());
    Ok(())
}
