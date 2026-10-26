//! Capture production multi-selection controls, using temporary mod files only.
#[allow(dead_code)]
#[path = "../tests/support/updates.rs"]
mod support;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use ts4_mod_manager_native::{
    settings::{Settings, Theme},
    ui,
};
const CASES: [(&str, bool, bool, bool, [f32; 2]); 7] = [
    ("cards-light", false, false, false, [1200., 800.]),
    ("list-dark", true, true, false, [1200., 800.]),
    ("list-narrow", false, true, false, [680., 760.]),
    ("list-minimum", false, true, false, [640., 480.]),
    ("cards-wide", false, false, false, [1876., 996.]),
    ("trash-review", true, false, true, [1200., 800.]),
    ("trash-review-minimum", false, true, true, [640., 480.]),
];
struct Capture {
    fixture: support::Fixture,
    dir: PathBuf,
    case: usize,
    frames: usize,
    list_selected: bool,
    clicks: usize,
    requested: bool,
    scans: usize,
    started: Instant,
}
impl Capture {
    fn stage(&mut self, ctx: &egui::Context) {
        self.fixture = support::fixture();
        let (_, dark, list, review, size) = CASES[self.case];
        ui::apply_settings_theme(
            ctx,
            &Settings {
                theme: if dark { Theme::Dark } else { Theme::Light },
                ..Default::default()
            },
        );
        self.list_selected = !list;
        self.clicks = if review { 3 } else { 2 };
        self.frames = 0;
        self.requested = false;
        self.scans = 0;
        for cmd in [
            egui::ViewportCommand::MinInnerSize(size.into()),
            egui::ViewportCommand::MaxInnerSize(size.into()),
            egui::ViewportCommand::InnerSize(size.into()),
        ] {
            ctx.send_viewport_cmd(cmd);
        }
    }
}
impl eframe::App for Capture {
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        if self.frames < 3 || self.fixture.controls.busy() {
            return;
        }
        let rect = if !self.list_selected {
            self.fixture
                .view
                .stats
                .list_switch
                .inspect(|_| self.list_selected = true)
        } else if self.clicks > 0 {
            let total = if CASES[self.case].3 { 3 } else { 2 };
            let rect = if self.clicks == total { self.fixture.view.stats.select_mode }
                else if self.clicks == total - 1 { self.fixture.view.stats.first_checkbox }
                else { self.fixture.view.stats.trash_selected };
            if rect.is_some() { self.clicks -= 1; }
            rect
        } else {
            None
        };
        if let Some(rect) = rect {
            let pos = rect.center();
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
            self.frames = 0;
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.fixture.controls.poll(&mut self.fixture.catalog);
        if let Some(root) = self.fixture.controls.refresh.take() {
            self.scans += 1;
            let scan = ts4_mod_manager_native::worker::scan(
                &root,
                &self.fixture.directory.path().join("managed"),
            )
            .unwrap();
            self.fixture.view.scan_ms = Some(scan.elapsed_ms);
            let mods = scan.mods;
            let generation = self.fixture.catalog.begin_scan(root);
            self.fixture.catalog.accept_scan(generation, Ok(mods));
        }
        self.fixture.view.show_with_controls(
            ui,
            &mut self.fixture.catalog,
            &mut self.fixture.controls,
        );
        self.fixture
            .controls
            .show_dialogs(&ctx, &self.fixture.catalog);
        let shot = ctx.input(|i| {
            i.events.iter().find_map(|e| {
                if let egui::Event::Screenshot { image, .. } = e {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let Some(image) = shot {
            assert!(
                self.fixture
                    .view
                    .stats
                    .first_row
                    .unwrap()
                    .contains_rect(self.fixture.view.stats.first_checkbox.unwrap()),
                "the complete checkbox must fit in the visible row"
            );
            assert_eq!(self.scans, 1, "only initial loading may scan the library");
            assert_eq!(self.fixture.controls.review_open(), CASES[self.case].3);
            assert!(self.fixture.view.stats.detail_close.is_none());
            assert_eq!(self.fixture.catalog.marked.len(), 1);
            assert_eq!(
                std::fs::read(self.fixture.directory.path().join("source/fixture.package"))
                    .unwrap(),
                b"synthetic fixture bytes"
            );
            assert_eq!(image.size, CASES[self.case].4.map(|v| v as usize));
            let bytes: Vec<_> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
            image::save_buffer(
                self.dir.join(format!("{}.png", CASES[self.case].0)),
                &bytes,
                image.size[0] as u32,
                image.size[1] as u32,
                image::ColorType::Rgba8,
            )
            .unwrap();
            self.case += 1;
            if self.case == CASES.len() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            self.stage(&ctx);
        } else if !self.requested
            && self.list_selected
            && self.clicks == 0
            && self.frames >= 5
            && !self.fixture.controls.busy()
            && ctx.content_rect().size() == egui::Vec2::from(CASES[self.case].4)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            self.requested = true;
        }
        self.frames += 1;
        assert!(
            self.started.elapsed() < Duration::from_secs(60),
            "selection capture timed out"
        );
        ctx.request_repaint_after(Duration::from_millis(50));
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("Provide capture directory")?,
    );
    std::fs::create_dir_all(&dir)?;
    eframe::run_native(
        "Library selection verification",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size(CASES[0].4)
                .with_resizable(false),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| {
            ui::setup(&cc.egui_ctx);
            let mut app = Capture {
                fixture: support::fixture(),
                dir,
                case: 0,
                frames: 0,
                list_selected: false,
                clicks: 0,
                requested: false,
                scans: 0,
                started: Instant::now(),
            };
            app.stage(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )?;
    println!("7 selection and trash review captures; no mod files changed, original bytes preserved");
    Ok(())
}
