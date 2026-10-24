//! Capture direct browser-download and Activity states with synthetic provider metadata.
#[allow(dead_code)]
#[path = "../tests/support/browser_import.rs"]
mod support;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use ts4_mod_manager_native::{
    browser_import::Phase,
    settings::{Settings, Theme},
    ui,
};
const CASES: [(&str, bool, [f32; 2], bool); 6] = [
    ("browser-download-dark", true, [1200.0, 800.0], false),
    ("browser-download-light", false, [1200.0, 800.0], false),
    ("browser-download-small", true, [640.0, 480.0], false),
    ("browser-download-user", false, [1871.0, 1006.0], false),
    ("browser-import-waiting-dark", true, [1200.0, 800.0], true),
    ("browser-import-waiting-small", false, [640.0, 480.0], true),
];
struct Capture {
    fixture: support::Fixture,
    directory: PathBuf,
    case: usize,
    frames: u8,
    requested: bool,
    staged: Instant,
    started: Instant,
}
impl Capture {
    fn stage(&mut self, ctx: &egui::Context) {
        let (_, dark, size, waiting) = CASES[self.case];
        ui::apply_settings_theme(
            ctx,
            &Settings {
                theme: if dark { Theme::Dark } else { Theme::Light },
                ..Default::default()
            },
        );
        ctx.set_theme(if dark {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        });
        ctx.all_styles_mut(|style| style.animation_time = 0.0);
        for command in [
            egui::ViewportCommand::MinInnerSize(size.into()),
            egui::ViewportCommand::MaxInnerSize(size.into()),
            egui::ViewportCommand::InnerSize(size.into()),
        ] {
            ctx.send_viewport_cmd(command);
        }
        self.fixture.controls.browser_import.prepare_loaded(
            self.fixture.catalog.root.clone().unwrap(),
            support::KEY,
            self.fixture.project.clone(),
        );
        self.fixture.controls.browser_import.download_dir =
            self.fixture.directory.path().join("Downloads");
        if waiting {
            let _ = self.fixture.controls.browser_import.start(ctx).unwrap();
        }
        self.frames = 0;
        self.requested = false;
        self.staged = Instant::now();
    }
}
impl eframe::App for Capture {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.fixture.controls.poll(&mut self.fixture.catalog);
        self.fixture.view.show_with_controls(
            ui,
            &mut self.fixture.catalog,
            &mut self.fixture.controls,
        );
        self.fixture
            .controls
            .show_dialogs(&ctx, &self.fixture.catalog);
        if let Some(image) = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        }) {
            assert_eq!(image.size, CASES[self.case].2.map(|n| n as usize));
            let bytes: Vec<_> = image
                .pixels
                .iter()
                .flat_map(|pixel| pixel.to_array())
                .collect();
            image::save_buffer(
                self.directory.join(format!("{}.png", CASES[self.case].0)),
                &bytes,
                image.size[0] as u32,
                image.size[1] as u32,
                image::ColorType::Rgba8,
            )
            .unwrap();
            assert_eq!(
                std::fs::read_dir(self.fixture.directory.path().join("game/Mods"))
                    .unwrap()
                    .count(),
                0
            );
            assert!(!self.fixture.directory.path().join("managed/mods").exists());
            self.case += 1;
            if self.case == CASES.len() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            self.stage(&ctx);
        } else if !self.requested
            && ctx.content_rect().size() == egui::Vec2::from(CASES[self.case].2)
        {
            self.frames += 1;
            if self.frames >= 5 && self.staged.elapsed() > Duration::from_millis(350) {
                if CASES[self.case].3 {
                    assert_eq!(self.fixture.controls.browser_import.phase, Phase::Waiting);
                } else {
                    assert!(!self.fixture.controls.review_open());
                    assert!(ctx.content_rect().contains_rect(self.fixture.controls.activity_button_rect.unwrap()));
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
                self.requested = true;
            }
        }
        assert!(
            self.started.elapsed() < Duration::from_secs(60),
            "Browser import capture timed out"
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
    let fixture = support::fixture();
    eframe::run_native(
        "Browser import verification",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size(CASES[0].2)
                .with_resizable(false),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| {
            ui::setup(&cc.egui_ctx);
            let mut capture = Capture {
                fixture,
                directory,
                case: 0,
                frames: 0,
                requested: false,
                staged: Instant::now(),
                started: Instant::now(),
            };
            capture.stage(&cc.egui_ctx);
            Ok(Box::new(capture))
        }),
    )?;
    println!("6 browser-import captures; synthetic metadata, no mod files downloaded or installed");
    Ok(())
}
