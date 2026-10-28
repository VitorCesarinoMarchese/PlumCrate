//! Capture production update notices with explicitly synthetic provider data.
#[allow(dead_code)]
#[path = "../tests/support/updates.rs"]
mod support;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use plumcrate_core::mod_updates::UpdateStatus;
use plumcrate::{
    settings::{Settings, Theme},
    ui,
};

const CASES: [(&str, bool, [f32; 2]); 9] = [
    ("updates-library-dark", true, [1200.0, 800.0]),
    ("updates-detail-dark", true, [1200.0, 800.0]),
    ("updates-detail-light", false, [1200.0, 800.0]),
    ("updates-unknown-small", false, [640.0, 480.0]),
    ("updates-error-small", true, [640.0, 480.0]),
    ("updates-detail-user", true, [1871.0, 1006.0]),
    ("updates-confirm-small", false, [640.0, 480.0]),
    ("updates-page-light", false, [1200.0, 800.0]),
    ("updates-page-small", false, [640.0, 480.0]),
];
struct Capture {
    fixture: support::Fixture,
    directory: PathBuf,
    case: usize,
    frames: usize,
    opened: bool,
    requested: bool,
    staged: Instant,
    started: Instant,
    metadata: Vec<u8>,
}
impl Capture {
    fn stage(&mut self, ctx: &egui::Context) {
        let (_, dark, size) = CASES[self.case];
        let theme = Settings {
            theme: if dark { Theme::Dark } else { Theme::Light },
            ..Default::default()
        };
        ui::apply_settings_theme(ctx, &theme);
        ctx.set_theme(if dark {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        });
        self.fixture.view = ui::View::default();
        let id = self.fixture.catalog.mods[0].id.clone().unwrap();
        let mut report = self.fixture.report.clone();
        if !matches!(self.case, 3 | 6) {
            report.installed = report.releases.iter().find(|r| r.file_id == 100).cloned();
            report.status = UpdateStatus::Available(
                report
                    .releases
                    .iter()
                    .find(|r| r.file_id == 200)
                    .unwrap()
                    .clone(),
            );
        }
        self.fixture.controls.updates.reports.insert(
            id,
            if self.case == 4 {
                Err("Could not reach CurseForge. Check your connection and try again.".into())
            } else {
                Ok(report.clone())
            },
        );
        if self.case == 6 {
            let release = report.releases.iter().find(|r| r.file_id == 100).unwrap();
            self.fixture
                .controls
                .review_installed_release(&self.fixture.catalog, &report, release);
        }
        for command in [
            egui::ViewportCommand::MinInnerSize(size.into()),
            egui::ViewportCommand::MaxInnerSize(size.into()),
            egui::ViewportCommand::InnerSize(size.into()),
        ] {
            ctx.send_viewport_cmd(command);
        }
        self.frames = 0;
        self.opened = self.case == 0 || self.case == 6;
        self.requested = false;
        self.staged = Instant::now();
    }
}
impl eframe::App for Capture {
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        if !self.opened
            && self.frames >= 2
            && let Some(row) = if self.case >= 7 { self.fixture.view.stats.updates_tab } else { self.fixture.view.stats.first_row }
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
            self.frames = 0;
            self.staged = Instant::now();
        }
    }
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
        let screenshot = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = screenshot {
            assert_eq!(image.size, CASES[self.case].2.map(|n| n as usize));
            let bytes: Vec<_> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
            image::save_buffer(
                self.directory.join(format!("{}.png", CASES[self.case].0)),
                &bytes,
                image.size[0] as u32,
                image.size[1] as u32,
                image::ColorType::Rgba8,
            )
            .unwrap();
            let id = self.fixture.catalog.mods[0].id.as_ref().unwrap();
            assert_eq!(
                std::fs::read(
                    self.fixture
                        .directory
                        .path()
                        .join("managed/mods")
                        .join(id)
                        .join("meta.json")
                )
                .unwrap(),
                self.metadata
            );
            self.case += 1;
            if self.case == CASES.len() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            self.stage(&ctx);
        } else if !self.requested
            && self.opened
            && ctx.content_rect().size() == egui::Vec2::from(CASES[self.case].2)
        {
            self.frames += 1;
            if self.frames >= 5 && self.staged.elapsed() >= Duration::from_millis(350) {
                if (1..=5).contains(&self.case) {
                    assert!(
                        ctx.content_rect()
                            .contains_rect(self.fixture.view.stats.detail_close.unwrap())
                    );
                }
                if self.case == 6 {
                    assert!(
                        ctx.content_rect()
                            .contains_rect(self.fixture.controls.confirm_rect.unwrap())
                    );
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
                self.requested = true;
            }
        } else {
            self.frames += 1;
        }
        assert!(
            self.started.elapsed() < Duration::from_secs(60),
            "Update capture timed out"
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
    let metadata = std::fs::read(
        fixture
            .directory
            .path()
            .join("managed/mods")
            .join(fixture.catalog.mods[0].id.as_ref().unwrap())
            .join("meta.json"),
    )?;
    eframe::run_native(
        "Update notice verification",
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
                opened: false,
                requested: false,
                staged: Instant::now(),
                started: Instant::now(),
                metadata,
            };
            capture.stage(&cc.egui_ctx);
            Ok(Box::new(capture))
        }),
    )?;
    println!("9 production update captures; synthetic provider data, no operations approved");
    Ok(())
}
