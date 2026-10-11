use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use ts4_mod_manager_native::{
    catalog::Catalog,
    cli::Options,
    controls::Controls,
    settings::Settings,
    ui::{self, View},
    worker::{self, Completion, Jobs, Request},
};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let options = Options::parse(std::env::args().skip(1))?;
    if options.verify_workflows {
        println!("{}", ts4_mod_manager_native::workflows::verify()?);
        return Ok(());
    }
    if options.help {
        println!(
            "TS4 Mod Manager native manager\n\nUsage: ts4-mod-manager-native [--root PATH] [--managed-root PATH]\n\n  --root PATH          Sims 4 folder containing Mods\n  --home PATH          Home to use for detection and default managed path\n  --verify-workflows   Verify management using newly created temporary fixtures\n  --inspect            Print catalog JSON and exit; requires --root\n  --query TEXT         Initial name/filename search\n  --settings           Open the full Settings page\n  --dark               Start with dark theme\n  --size WIDTHxHEIGHT   Initial window size, default 1200x800\n  --screenshot PATH    Save a rendered PNG and exit; requires --root\n\nManage local mods, check releases, and install verified browser downloads."
        );
        return Ok(());
    }
    if options.inspect {
        let output = worker::scan(
            options.root.as_deref().expect("validated root"),
            &options.managed_root,
        )?;
        let mut catalog = Catalog::default();
        let generation = catalog.begin_scan(output.instance.path.clone());
        catalog.accept_scan(generation, Ok(output.mods));
        let search_start = Instant::now();
        catalog.search(options.query);
        let search_ms = search_start.elapsed().as_secs_f64() * 1000.0;
        let visible = catalog
            .visible
            .iter()
            .map(|index| &catalog.mods[*index])
            .collect::<Vec<_>>();
        println!(
            "{}",
            serde_json::json!({"instance": output.instance, "total": catalog.mods.len(), "matches": visible.len(), "scanMs": output.elapsed_ms, "searchMs": search_ms, "mods": visible})
        );
        return Ok(());
    }
    fastframe_log::Logging::new("ts4-mod-manager-native", env!("CARGO_PKG_VERSION"))
        .filter("warn")
        .init()
        .map_err(|error| error.to_string())?;
    let settings = Settings::load(&options.home.join(".config/ts4-mod-manager/settings.json"))?;
    let captured = Arc::new(Mutex::new(None));
    let capture_required = options.screenshot.is_some();
    let capture_result = Arc::clone(&captured);
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size(options.size)
        .with_min_inner_size([640.0, 480.0]);
    if capture_required {
        viewport = viewport
            .with_min_inner_size(options.size)
            .with_max_inner_size(options.size)
            .with_resizable(false);
    }
    let native_options = eframe::NativeOptions {
        viewport,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        "Sims 4 Mod Manager · Native",
        native_options,
        Box::new(move |cc| {
            ui::setup(&cc.egui_ctx);
            ui::apply_settings_theme(&cc.egui_ctx, &settings);
            if options.dark {
                cc.egui_ctx.set_theme(egui::Theme::Dark);
            } else if capture_required && settings.active_custom_theme.is_none() {
                cc.egui_ctx.set_theme(egui::Theme::Light);
            }
            let ctx = cc.egui_ctx.clone();
            let jobs = Jobs::start(move || ctx.request_repaint())?;
            let wake_ctx = cc.egui_ctx.clone();
            let mut controls = Controls::new(
                options.managed_root.clone(),
                options.home.clone(),
                settings.clone(),
                move || wake_ctx.request_repaint(),
            )?;
            controls.settings_page = options.settings_page;
            let mut view = View::default();
            view.discovery = ts4_mod_manager_native::discovery::DiscoverView::with_cache(options.home.join(".cache/ts4-mod-manager/photos"));
            jobs.submit(Request::Detect {
                generation: 0,
                home: options.home,
            });
            let mut app = NativeApp {
                controls,
                catalog: Catalog::default(),
                view,
                jobs,
                managed_root: options.managed_root,
                scrolling: Default::default(),
                screenshot: options.screenshot,
                capture_requested: false,
                stable_frames: 0,
                started: Instant::now(),
                captured,
            };
            app.catalog.search(options.query);
            if let Some(root) = options
                .root
                .or_else(|| settings.selected_root.as_ref().map(PathBuf::from))
            {
                app.scan(root);
            }
            Ok(Box::new(app))
        }),
    )
    .map_err(|error| error.to_string())?;
    if capture_required {
        capture_result
            .lock()
            .expect("capture result")
            .take()
            .ok_or("Window closed before screenshot completed")??;
    }
    Ok(())
}

struct NativeApp {
    controls: Controls,
    catalog: Catalog,
    view: View,
    jobs: Jobs,
    managed_root: PathBuf,
    scrolling: fastframe_scroll::Scrolling,
    screenshot: Option<PathBuf>,
    capture_requested: bool,
    stable_frames: u8,
    started: Instant,
    captured: Arc<Mutex<Option<Result<(), String>>>>,
}

impl NativeApp {
    fn scan(&mut self, root: PathBuf) {
        self.view.root_input = root.to_string_lossy().into_owned();
        self.view.scan_ms = None;
        let generation = self.catalog.begin_scan(root.clone());
        self.jobs.submit(Request::Scan {
            generation,
            root,
            managed_root: self.managed_root.clone(),
        });
    }

    fn poll(&mut self) {
        let completions = self.jobs.drain().collect::<Vec<_>>();
        let mut detected_root = None;
        for completion in completions {
            match completion {
                Completion::Detected {
                    generation,
                    instances,
                } if generation == self.catalog.generation() => {
                    if self.catalog.root.is_none() {
                        detected_root = instances.first().map(|instance| instance.path.clone());
                    }
                    self.view.instances = instances;
                }
                Completion::Scanned { generation, result } => {
                    if generation != self.catalog.generation() {
                        continue;
                    }
                    let mods = result.map(|output| {
                        self.view.scan_ms = Some(output.elapsed_ms);
                        if !self
                            .view
                            .instances
                            .iter()
                            .any(|instance| instance.path == output.instance.path)
                        {
                            self.view.instances.push(output.instance);
                        }
                        output.mods
                    });
                    self.catalog.accept_scan(generation, mods);
                    if self.catalog.selected.is_none() && !self.catalog.mods.is_empty() {
                        self.catalog.select(0);
                    }
                }
                _ => {}
            }
        }
        if let Some(root) = detected_root {
            self.scan(root);
        }
    }
}

impl eframe::App for NativeApp {
    fn logic(&mut self, _ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();
        self.controls.poll(&self.catalog);
        if let Some(root) = self
            .controls
            .refresh
            .take()
            .or_else(|| self.controls.roots_changed.take())
        {
            self.scan(root);
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.scrolling.apply(ui.ctx());
        if self.controls.busy() && ui.input(|i| i.viewport().close_requested()) {
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }
        if let Some(root) = self
            .view
            .show_with_controls(ui, &mut self.catalog, &mut self.controls)
        {
            self.scan(root);
        }
        self.controls.show_dialogs(ui.ctx(), &self.catalog);
        let Some(path) = self.screenshot.as_ref() else {
            return;
        };
        if let Some(image) = ui.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(Arc::clone(image)),
                _ => None,
            })
        }) {
            let bytes = image
                .pixels
                .iter()
                .flat_map(|pixel| pixel.to_array())
                .collect::<Vec<_>>();
            let result = image::save_buffer_with_format(
                path,
                &bytes,
                image.size[0] as u32,
                image.size[1] as u32,
                image::ColorType::Rgba8,
                image::ImageFormat::Png,
            )
            .map_err(|error| error.to_string());
            *self.captured.lock().expect("capture result") = Some(result);
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        } else if self.started.elapsed() > Duration::from_secs(30) {
            *self.captured.lock().expect("capture result") =
                Some(Err("Screenshot timed out".into()));
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
        } else if !self.catalog.loading && !self.capture_requested {
            if ui.ctx().has_pending_images() {
                self.stable_frames = 0;
            } else {
                self.stable_frames += 1;
            }
            if self.stable_frames >= 3 {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
                self.capture_requested = true;
            }
            ui.ctx().request_repaint();
        }
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
}
