//! Exercise eframe's desktop URL dispatch with a controlled external browser.
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

const URL: &str = "https://www.curseforge.com/sims4/mods/synthetic-fixture/download/100";
struct Probe {
    receipt: PathBuf,
    started: Instant,
    frames: u64,
    requested: bool,
}
impl eframe::App for Probe {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        ui.label("Verifying desktop browser dispatch with a synthetic release URL.");
        if self.receipt.exists() {
            assert_eq!(std::fs::read_to_string(&self.receipt).unwrap(), URL);
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        self.frames += 1;
        if self.frames >= 3 && !self.requested {
            ui.ctx().open_url(egui::OpenUrl::new_tab(URL));
            self.requested = true;
        }
        plumcrate::desktop_links::dispatch_pending(ui.ctx());
        assert!(
            self.started.elapsed() < Duration::from_secs(15),
            "desktop backend did not launch the external browser"
        );
        ui.ctx().request_repaint_after(Duration::from_millis(50));
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let receipt = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("Provide the browser receipt path")?,
    );
    if std::env::var_os("TS4_BROWSER_RECEIPT").as_deref() != Some(receipt.as_os_str())
        || std::env::var_os("TS4_VERIFY_BROWSER_LINKS").as_deref()
            != Some(std::ffi::OsStr::new("1"))
    {
        return Err("Run through native/verify_browser_links.py with a controlled browser".into());
    }
    eframe::run_native(
        "Browser dispatch verification",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([640.0, 240.0]),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |_| {
            Ok(Box::new(Probe {
                receipt,
                started: Instant::now(),
                frames: 0,
                requested: false,
            }))
        }),
    )?;
    println!("Desktop backend launched the controlled browser with the exact release URL");
    Ok(())
}
