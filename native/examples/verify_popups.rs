//! Capture production review popups using synthetic data, without approving any operation.
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use ts4_mod_manager_native::{catalog, management, preview, settings};

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
        let identity = controls.identity(catalog).unwrap();
        match case {
            0..=3 => controls.picked(PickPurpose::Import(identity, case != 3), Some(if case == 3 { source.with_extension("") } else { source }), catalog),
            4 => controls.dialog = Some(Dialog::Rename(identity, "Synthetic mod collection".into())),
            5 => controls.dialog = Some(Dialog::Manual(identity, String::new())),
            6 => controls.confirm(catalog, Action::RemoveSource, "Remove source", (0..40).map(|index| format!("Synthetic review line {index}: metadata is preserved until you confirm.")).collect()),
            7 => controls.dialog = Some(Dialog::SettingsImport(Settings::default(), false)),
            8 => controls.candidates = Some((identity, vec![])),
            9 => controls.trash_entries = Some((identity, vec![])),
            _ => unreachable!(),
        }
    }

    pub fn assert_unapproved(controls: &Controls) {
        assert!(controls.operations.is_empty());
        assert!(!controls.settings_path.exists());
    }
}

const CASES: [(&str, bool, [f32; 2]); 10] = [
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
        self.stable_frames = 0;
        self.requested = false;
        self.staged = Instant::now();
    }
}

impl eframe::App for Capture {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.controls.poll(&self.catalog);
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
            {
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
    std::fs::create_dir_all(source.with_extension(""))?;
    let source_check = source.clone();
    let mut catalog = catalog::Catalog::default();
    let generation = catalog.begin_scan(root);
    catalog.accept_scan(generation, Ok(vec![]));
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
    println!("10 production popup captures; no operations approved or settings saved");
    Ok(())
}
