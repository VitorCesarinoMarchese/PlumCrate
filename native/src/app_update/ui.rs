use super::*;
use std::sync::mpsc::{self, Receiver};

#[derive(Default)]
enum Phase {
    #[default]
    Idle,
    Checking,
    Downloading {
        received: u64,
        total: u64,
    },
    Ready(Staged),
    Installing,
}
enum Event {
    Checked(Result<Option<Release>, String>),
    Progress(u64, u64),
    Downloaded(Result<Staged, String>),
    Restarted(Result<(), String>),
}

/// All network, hashing, version probing and installation work runs off the UI thread.
#[derive(Default)]
pub struct AppUpdates {
    target: Option<Target>,
    unsupported: Option<String>,
    release: Option<Release>,
    phase: Phase,
    receiver: Option<Receiver<Event>>,
    last_check: Option<Instant>,
    error: Option<String>,
    checked: bool,
    open: bool,
    pub restart_requested: bool,
}

impl AppUpdates {
    /// Called only by interactive launches, so tests and inspection stay offline.
    pub fn activate(&mut self) {
        match Target::detect() {
            Ok(target) => self.target = Some(target),
            Err(reason) => self.unsupported = Some(reason),
        }
    }
    pub fn installing(&self) -> bool {
        matches!(self.phase, Phase::Installing)
    }
    fn pending(&self) -> bool {
        matches!(
            self.phase,
            Phase::Checking | Phase::Downloading { .. } | Phase::Installing
        )
    }
    fn start(
        &mut self,
        ctx: &egui::Context,
        name: &str,
        work: impl FnOnce(mpsc::Sender<Event>) + Send + 'static,
    ) {
        let (sender, receiver) = mpsc::channel();
        let wake = ctx.clone();
        match std::thread::Builder::new()
            .name(name.into())
            .spawn(move || {
                work(sender);
                wake.request_repaint();
            }) {
            Ok(_) => self.receiver = Some(receiver),
            Err(_) => {
                self.phase = Phase::Idle;
                self.error = Some("Could not start the app update worker. Try again.".into());
            }
        }
    }
    fn start_check(&mut self, ctx: &egui::Context) {
        if self.target.is_none() || self.pending() || matches!(self.phase, Phase::Ready(_)) {
            return;
        }
        self.last_check = Some(Instant::now());
        self.phase = Phase::Checking;
        self.error = None;
        self.start(ctx, "app-update-check", |sender| {
            let _ = sender.send(Event::Checked(check()));
        });
    }
    fn start_download(&mut self, ctx: &egui::Context) {
        if self.pending() || matches!(self.phase, Phase::Ready(_)) {
            return;
        }
        let (Some(target), Some(release)) = (self.target.clone(), self.release.clone()) else {
            return;
        };
        self.phase = Phase::Downloading {
            received: 0,
            total: release.asset.size,
        };
        self.error = None;
        let wake = ctx.clone();
        self.start(ctx, "app-update-download", move |sender| {
            let mut last_progress = Instant::now();
            let result = download(target, &release, |received, total| {
                if last_progress.elapsed() >= Duration::from_millis(100) || received == total {
                    let _ = sender.send(Event::Progress(received, total));
                    wake.request_repaint();
                    last_progress = Instant::now();
                }
            });
            let _ = sender.send(Event::Downloaded(result));
        });
    }
    fn start_restart(&mut self, ctx: &egui::Context, blocked: bool) {
        if blocked || !matches!(self.phase, Phase::Ready(_)) {
            return;
        }
        let Phase::Ready(staged) = std::mem::replace(&mut self.phase, Phase::Installing) else {
            unreachable!()
        };
        self.start(ctx, "app-update-install", move |sender| {
            let result = staged
                .install()
                .and_then(|installed| installed.restart(std::env::args_os().skip(1).collect()));
            let _ = sender.send(Event::Restarted(result));
        });
    }
    pub fn poll(&mut self, ctx: &egui::Context, automatic: bool) {
        let mut events = Vec::new();
        let mut disconnected = false;
        if let Some(receiver) = &self.receiver {
            loop {
                match receiver.try_recv() {
                    Ok(event) => events.push(event),
                    Err(mpsc::TryRecvError::Empty) => break,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }
        }
        let mut completed = false;
        for event in events {
            match event {
                Event::Progress(received, total) => {
                    self.phase = Phase::Downloading { received, total }
                }
                Event::Checked(result) => {
                    self.phase = Phase::Idle;
                    self.checked = true;
                    completed = true;
                    match result {
                        Ok(release) => self.release = release,
                        Err(error) => self.error = Some(error),
                    }
                }
                Event::Downloaded(result) => {
                    completed = true;
                    match result {
                        Ok(staged) => self.phase = Phase::Ready(staged),
                        Err(error) => {
                            self.phase = Phase::Idle;
                            self.error = Some(error);
                        }
                    }
                }
                Event::Restarted(result) => {
                    completed = true;
                    self.phase = Phase::Idle;
                    match result {
                        Ok(()) => self.restart_requested = true,
                        Err(error) => self.error = Some(error),
                    }
                }
            }
        }
        if completed {
            self.receiver = None;
        }
        if disconnected && !completed {
            self.receiver = None;
            self.phase = Phase::Idle;
            self.error = Some("The app update worker stopped unexpectedly. Try again.".into());
        }
        if automatic
            && !self.restart_requested
            && self
                .last_check
                .is_none_or(|time| time.elapsed() >= Duration::from_secs(24 * 60 * 60))
        {
            self.start_check(ctx);
        }
        if automatic
            && self.target.is_some()
            && !matches!(self.phase, Phase::Ready(_))
            && let Some(last) = self.last_check
        {
            ctx.request_repaint_after(
                Duration::from_secs(24 * 60 * 60)
                    .saturating_sub(last.elapsed())
                    .max(Duration::from_secs(1)),
            );
        }
        if self.pending() {
            ctx.request_repaint_after(Duration::from_millis(100));
        }
    }
    pub fn banner(&mut self, ui: &mut egui::Ui) {
        if self.release.is_some() && ui.button("App update available").clicked() {
            self.open = true;
        }
    }
    pub fn settings(&mut self, ui: &mut egui::Ui, automatic: &mut bool) {
        ui.heading("App updates");
        ui.label(format!("TS4 Mod Manager {}", env!("CARGO_PKG_VERSION")));
        if let Some(reason) = &self.unsupported {
            ui.add(egui::Label::new(reason).wrap());
            ui.hyperlink_to(
                "Download from GitHub",
                format!("https://github.com/{REPOSITORY}/releases"),
            );
            return;
        }
        ui.checkbox(automatic, "Automatically check for app updates");
        ui.label("Checks GitHub on startup and once a day while open. Downloads and restarts need your approval.");
        ui.horizontal_wrapped(|ui| {
            if ui
                .add_enabled(
                    self.target.is_some()
                        && !self.pending()
                        && !matches!(self.phase, Phase::Ready(_)),
                    egui::Button::new("Check for app updates"),
                )
                .clicked()
            {
                self.start_check(ui.ctx());
            }
            if self.release.is_some() && ui.button("Review app update").clicked() {
                self.open = true;
            }
        });
        if matches!(self.phase, Phase::Checking) {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Checking GitHub...");
            });
        } else if self.checked && self.release.is_none() && self.error.is_none() {
            ui.label("No newer compatible AppImage release is available.");
        }
        if let Some(error) = &self.error {
            ui.add(
                egui::Label::new(egui::RichText::new(error).color(ui.visuals().error_fg_color))
                    .wrap(),
            );
        }
    }
    pub fn show_dialog(&mut self, ctx: &egui::Context, blocked: bool) {
        if !self.open {
            return;
        }
        let Some(release) = self.release.clone() else {
            self.open = false;
            return;
        };
        let mut open = self.open;
        egui::Window::new("Update TS4 Mod Manager").id(egui::Id::new("app-update-review"))
            .open(&mut open).collapsible(false).resizable(false).default_width(420.0)
            .show(ctx, |ui| {
                ui.label(format!("{} → {}", env!("CARGO_PKG_VERSION"), release.version));
                ui.hyperlink_to("Release notes", &release.notes);
                ui.label("Your mods and settings stay in place. The previous AppImage is kept beside this copy in a backup folder.");
                if let Some(error) = &self.error { ui.add(egui::Label::new(egui::RichText::new(error).color(ui.visuals().error_fg_color)).wrap()); }
                match &self.phase {
                    Phase::Downloading { received, total } => {
                        ui.add(egui::ProgressBar::new(*received as f32 / (*total).max(1) as f32)
                            .text(format!("{:.1} / {:.1} MB", *received as f64 / 1_000_000.0, *total as f64 / 1_000_000.0)));
                        ui.label("Downloading and verifying the AppImage...");
                    }
                    Phase::Ready(_) => {
                        ui.label("Download verified. Ready to install.");
                        if blocked { ui.label("Finish mod operations, close other reviews, and save or discard settings before restarting."); }
                        if ui.add_enabled(!blocked, egui::Button::new("Restart to update")).clicked() { self.start_restart(ctx, blocked); }
                    }
                    Phase::Installing => { ui.horizontal(|ui| { ui.spinner(); ui.label("Installing and restarting..."); }); }
                    Phase::Idle => {
                        if ui.button(if self.error.is_some() { "Retry app download" } else { "Download app update" }).clicked() { self.start_download(ctx); }
                    }
                    Phase::Checking => { ui.spinner(); }
                }
            });
        self.open = open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn automatic_checks_do_nothing_without_an_appimage() {
        let mut updates = AppUpdates::default();
        updates.poll(&egui::Context::default(), true);
        assert!(updates.receiver.is_none());
        assert!(updates.last_check.is_none());
    }

    #[test]
    fn automatic_rechecks_schedule_a_repaint_while_idle() {
        let directory = tempfile::tempdir().unwrap();
        let ctx = egui::Context::default();
        let mut updates = AppUpdates {
            target: Some(super::super::tests::target(directory.path())),
            last_check: Some(Instant::now()),
            ..Default::default()
        };
        let mut delay = Duration::MAX;
        for _ in 0..5 {
            let mut output = ctx.run_ui(egui::RawInput::default(), |_| updates.poll(&ctx, true));
            delay = output.viewport_output[&egui::ViewportId::ROOT].repaint_delay;
            output.textures_delta.clear();
        }
        assert!(
            delay < Duration::from_secs(24 * 60 * 60),
            "the idle window has no next update check scheduled"
        );
        assert!(updates.receiver.is_none());
    }

    #[test]
    fn checking_does_not_start_a_download() {
        let mut updates = AppUpdates::default();
        let (sender, receiver) = mpsc::channel();
        updates.receiver = Some(receiver);
        updates.phase = Phase::Checking;
        sender
            .send(Event::Checked(Ok(Some(super::super::tests::selected()))))
            .unwrap();
        updates.poll(&egui::Context::default(), false);
        assert!(updates.release.is_some());
        assert!(matches!(updates.phase, Phase::Idle));
        assert!(updates.receiver.is_none());
    }

    #[test]
    fn restart_waits_for_operations_and_unsaved_settings() {
        let directory = tempfile::tempdir().unwrap();
        let target = super::super::tests::target(directory.path());
        let release = super::super::tests::selected();
        let staged = stage_download(
            target.clone(),
            &release,
            release.sha256.as_ref().unwrap(),
            std::io::Cursor::new(super::super::tests::image()),
            |_, _| {},
        )
        .unwrap();
        let mut updates = AppUpdates {
            phase: Phase::Ready(staged),
            ..Default::default()
        };
        updates.start_restart(&egui::Context::default(), true);
        assert!(matches!(updates.phase, Phase::Ready(_)));
        assert!(updates.receiver.is_none());
        assert!(!updates.restart_requested);
        assert_eq!(fs::read(target.path).unwrap(), super::super::tests::image());
    }

    #[test]
    fn installing_blocks_mod_mutations_and_window_closure() {
        let directory = tempfile::tempdir().unwrap();
        let mut controls = crate::controls::Controls::new(
            directory.path().join("managed"),
            directory.path().into(),
            crate::settings::Settings::default(),
            || {},
        )
        .unwrap();
        for _ in 0..100 {
            controls.poll(&mut crate::catalog::Catalog::default());
            if !controls.busy() {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(!controls.busy());
        controls.app_updates.phase = Phase::Installing;
        assert!(controls.busy());
        let result = controls.fixture_action(
            directory.path().join("game"),
            None,
            crate::management::Action::RemoveSource,
        );
        match result {
            Err(error) => assert!(error.contains("app update"), "{error}"),
            Ok(_) => panic!("queued a mutation while restarting"),
        }
    }
}
