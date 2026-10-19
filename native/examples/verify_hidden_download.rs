//! Exercise the real download watcher and management queue while the window is hidden.
#[allow(dead_code)]
#[path = "../tests/support/browser_import.rs"]
mod support;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use ts4_mod_manager_native::{browser_import::Phase, ui};
struct Probe {
    fixture: support::Fixture,
    signals: PathBuf,
    started: Instant,
    frames: u64,
}
impl eframe::App for Probe {
    fn logic(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.fixture.controls.poll(&mut self.fixture.catalog);
        if self.fixture.controls.browser_import.phase == Phase::Complete {
            let root = self.fixture.directory.path();
            assert_eq!(
                std::fs::read(root.join("game/Mods/fixture.package")).unwrap(),
                support::BYTES
            );
            assert_eq!(
                std::fs::read(root.join("Downloads/fixture.package")).unwrap(),
                support::BYTES
            );
            std::fs::write(
                self.signals.join("complete"),
                b"imported and enabled; original preserved",
            )
            .unwrap();
        }
        if self.signals.join("stop").exists() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        self.fixture.view.show_with_controls(
            ui,
            &mut self.fixture.catalog,
            &mut self.fixture.controls,
        );
        self.fixture
            .controls
            .show_dialogs(ui.ctx(), &self.fixture.catalog);
        self.frames += 1;
        if self.frames == 3 {
            let _ = self
                .fixture
                .controls
                .browser_import
                .start(ui.ctx())
                .unwrap();
            std::fs::write(
                self.signals.join("ready.json"),
                serde_json::to_vec(&serde_json::json!({
                    "downloads": self.fixture.directory.path().join("Downloads"),
                    "game": self.fixture.directory.path().join("game/Mods"),
                }))
                .unwrap(),
            )
            .unwrap();
        }
        std::fs::write(self.signals.join("heartbeat"), self.frames.to_string()).unwrap();
        assert!(
            self.started.elapsed() < Duration::from_secs(40),
            "hidden-window verification timed out"
        );
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let signals = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("Provide a synthetic signal directory")?,
    );
    eframe::run_native(
        "TS4 hidden download verification",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([900.0, 700.0])
                .with_app_id("ts4-hidden-download-verification"),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| {
            ui::setup(&cc.egui_ctx);
            let wake = cc.egui_ctx.clone();
            let fixture = support::fixture_with_wake(move || wake.request_repaint());
            Ok(Box::new(Probe {
                fixture,
                signals,
                started: Instant::now(),
                frames: 0,
            }))
        }),
    )?;
    Ok(())
}
