//! Capture Activity with isolated fixtures; never starts a download or touches user mods.
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use ts4_mod_manager_native::{
    catalog::Catalog,
    controls::Controls,
    settings::{Settings, Theme},
    ui::{self, View},
};

const CASES: [(&str, bool, [f32; 2], u8); 9] = [
    ("idle-dark", true, [1876.0, 996.0], 0),
    ("waiting-header", true, [1876.0, 996.0], 1),
    ("waiting-open", true, [1200.0, 800.0], 2),
    ("waiting-small", true, [640.0, 480.0], 2),
    ("waiting-light", false, [1200.0, 800.0], 2),
    ("error-toast", true, [1200.0, 800.0], 3),
    ("error-open", true, [640.0, 480.0], 4),
    ("success-toast", true, [1200.0, 800.0], 5),
    ("success-expired", true, [1200.0, 800.0], 6),
];
struct Capture {
    fixture: tempfile::TempDir,
    controls: Controls,
    view: View,
    catalog: Catalog,
    directory: PathBuf,
    case: usize,
    staged: Instant,
    clicked: bool,
    requested: bool,
}
impl Capture {
    fn stage(&mut self, ctx: &egui::Context) {
        let (_, dark, size, kind) = CASES[self.case];
        let settings = Settings {
            theme: if dark { Theme::Dark } else { Theme::Light },
            ..Settings::default()
        };
        self.controls = Controls::new(
            self.fixture.path().join("managed"),
            self.fixture.path().into(),
            settings.clone(),
            || {},
        )
        .unwrap();
        ui::apply_settings_theme(ctx, &settings);
        ctx.send_viewport_cmd(egui::ViewportCommand::MinInnerSize(size.into()));
        ctx.send_viewport_cmd(egui::ViewportCommand::MaxInnerSize(size.into()));
        ctx.send_viewport_cmd(egui::ViewportCommand::Resizable(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size.into()));
        egui::Popup::close_all(ctx);
        let game = self.fixture.path().join("The Sims 4");
        std::fs::create_dir_all(game.join("Mods")).unwrap();
        self.catalog = Catalog::default();
        let generation = self.catalog.begin_scan(game);
        self.catalog.accept_scan(generation, Ok(vec![]));
        self.view = View::default();
        if matches!(kind, 1 | 2) {
            self.controls.browser_import.phase =
                ts4_mod_manager_native::browser_import::Phase::Waiting;
            self.controls.browser_import.project =
                Some(ts4_mod_manager_core::browser_import::BrowserProject {
                    project_id: 1,
                    title: "Synthetic collection".into(),
                    author: None,
                    source_url: "https://fixture.invalid/project".into(),
                    preview_url: None,
                    releases: vec![],
                    complete: true,
                });
        }
        if matches!(kind, 3 | 4) {
            self.controls.error = Some("Path collision: the original file was kept. Choose a different destination before retrying.".into());
        }
        if matches!(kind, 5 | 6) {
            self.controls.settings_page = true;
            self.controls.settings.check_app_updates = false;
        }
        self.clicked = false;
        self.requested = false;
        self.staged = Instant::now();
    }
}
impl eframe::App for Capture {
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        if self.case >= CASES.len() {
            return;
        }
        let kind = CASES[self.case].3;
        if kind == 6
            && self.staged.elapsed() > Duration::from_millis(700)
            && let Some(time) = &mut input.time
        {
            *time += 10.0;
        }
        if !self.clicked && self.staged.elapsed() > Duration::from_millis(300) {
            let pos = if matches!(kind, 2 | 4) {
                self.controls.activity_button_rect.map(|rect| rect.center())
            } else if matches!(kind, 5 | 6) {
                Some(egui::pos2(1119.0, 103.0))
            } else {
                None
            };
            if let Some(pos) = pos {
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
                self.clicked = true;
            }
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.case >= CASES.len() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        self.controls.poll(&mut self.catalog);
        self.view
            .show_with_controls(ui, &mut self.catalog, &mut self.controls);
        if let Some(image) = ui.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        }) {
            let bytes: Vec<_> = image
                .pixels
                .iter()
                .flat_map(|pixel| pixel.to_array())
                .collect();
            image::save_buffer_with_format(
                self.directory.join(format!("{}.png", CASES[self.case].0)),
                &bytes,
                image.size[0] as u32,
                image.size[1] as u32,
                image::ColorType::Rgba8,
                image::ImageFormat::Png,
            )
            .unwrap();
            self.case += 1;
            if self.case == CASES.len() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            self.stage(&ctx);
        } else if !self.requested
            && self.staged.elapsed() > Duration::from_millis(1200)
            && ctx.content_rect().size() == CASES[self.case].2.into()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            self.requested = true;
        }
        assert!(
            self.staged.elapsed() < Duration::from_secs(20),
            "Activity capture timed out: expected {:?}, got {:?}",
            CASES[self.case].2,
            ctx.content_rect().size()
        );
        ctx.request_repaint_after(Duration::from_millis(30));
    }
}
fn main() -> eframe::Result {
    let directory = PathBuf::from(std::env::args().nth(1).expect("capture directory"));
    std::fs::create_dir_all(&directory).unwrap();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size(CASES[0].2)
            .with_min_inner_size(CASES[0].2)
            .with_max_inner_size(CASES[0].2)
            .with_resizable(false),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Synthetic Activity verification",
        options,
        Box::new(move |cc| {
            ui::setup(&cc.egui_ctx);
            let fixture = tempfile::tempdir().unwrap();
            let controls = Controls::new(
                fixture.path().join("managed"),
                fixture.path().into(),
                Settings::default(),
                || {},
            )
            .unwrap();
            let mut capture = Capture {
                fixture,
                controls,
                view: View::default(),
                catalog: Catalog::default(),
                directory,
                case: 0,
                staged: Instant::now(),
                clicked: false,
                requested: false,
            };
            capture.stage(&cc.egui_ctx);
            Ok(Box::new(capture))
        }),
    )
}
