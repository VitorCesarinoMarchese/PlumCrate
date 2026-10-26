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
const CASES: [(&str, bool, [f32; 2], u8); 6] = [
    ("trash-light", false, [1200., 800.], 0),
    ("trash-dark", true, [1200., 800.], 0),
    ("trash-minimum", false, [640., 480.], 0),
    ("delete-review-dark", true, [1200., 800.], 1),
    ("delete-review-minimum", false, [640., 480.], 1),
    ("delete-acknowledged-minimum", false, [640., 480.], 2),
];
struct Capture {
    fixture: support::Fixture,
    dir: PathBuf,
    case: usize,
    frames: usize,
    clicks: usize,
    requested: bool,
    scans: usize,
    started: Instant,
}
impl Capture {
    fn stage(&mut self, ctx: &egui::Context) {
        self.fixture = support::fixture();
        let (_, dark, size, kind) = CASES[self.case];
        ui::apply_settings_theme(
            ctx,
            &Settings {
                theme: if dark { Theme::Dark } else { Theme::Light },
                ..Default::default()
            },
        );
        let root = self.fixture.catalog.root.clone().unwrap();
        let managed = self.fixture.directory.path().join("managed");
        let trash = self
            .fixture
            .directory
            .path()
            .join(".local/share/Trash/files");
        let id = self.fixture.catalog.mods[0].id.clone().unwrap();
        ts4_mod_manager_core::lifecycle::uninstall_managed_mod(
            &managed,
            &root.join("Mods"),
            &trash,
            &id,
        )
        .unwrap();
        let generation = self.fixture.catalog.begin_scan(root.clone());
        self.fixture.catalog.accept_scan(
            generation,
            Ok(ts4_mod_manager_native::worker::scan(&root, &managed)
                .unwrap()
                .mods),
        );
        self.fixture.controls.refresh = None;
        self.fixture.controls.open_trash(&self.fixture.catalog);
        self.clicks = kind as usize;
        self.frames = 0;
        self.started = Instant::now();
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
        let rect = if self.clicks > 0 {
            let rect = if self.clicks == CASES[self.case].3 as usize {
                self.fixture.controls.empty_trash_rect
            } else {
                self.fixture.controls.delete_ack_rect
            };
            if rect.is_some() {
                self.clicks -= 1;
            }
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
            assert!(self.fixture.controls.review_open());
            assert!(self.fixture.view.stats.detail_close.is_none());
            if CASES[self.case].3 == 2 {
                assert!(self.fixture.controls.confirm_rect.is_some());
            } else if CASES[self.case].3 == 1 {
                assert!(self.fixture.controls.confirm_rect.is_none());
            }
            assert_eq!(
                std::fs::read(self.fixture.directory.path().join("source/fixture.package"))
                    .unwrap(),
                b"synthetic fixture bytes"
            );
            assert_eq!(image.size, CASES[self.case].2.map(|v| v as usize));
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
            && self.clicks == 0
            && self.fixture.controls.review_open()
            && (CASES[self.case].3 == 0 || self.fixture.controls.delete_ack_rect.is_some())
            && self.frames >= 5
            && !self.fixture.controls.busy()
            && ctx.content_rect().size() == egui::Vec2::from(CASES[self.case].2)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            self.requested = true;
        }
        self.frames += 1;
        assert!(
            self.started.elapsed() < Duration::from_secs(60),
            "trash capture timed out"
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
        "Trash deletion verification",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size(CASES[0].2)
                .with_resizable(false),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| {
            ui::setup(&cc.egui_ctx);
            cc.egui_ctx.all_styles_mut(|style| style.animation_time = 0.0);
            let mut app = Capture {
                fixture: support::fixture(),
                dir,
                case: 0,
                frames: 0,
                clicks: 0,
                requested: false,
                scans: 0,
                started: Instant::now(),
            };
            app.stage(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )?;
    println!(
        "6 trash deletion controls captured; permanent deletion never approved, originals preserved"
    );
    Ok(())
}
