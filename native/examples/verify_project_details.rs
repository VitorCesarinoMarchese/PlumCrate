//! Capture production project widgets with synthetic data and no provider requests.
#[allow(dead_code)]
#[path = "../tests/support/project_details.rs"]
mod support;
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use ts4_mod_manager_native::{
    catalog::Catalog,
    project_details::Load,
    ui::{self, View},
};
const CASES: [(&str, bool, [f32; 2], u8, u8); 12] = [
    ("details-dark", true, [1200.0, 800.0], 0, 0),
    ("details-light", false, [1200.0, 800.0], 0, 0),
    ("details-user", true, [1871.0, 1006.0], 0, 0),
    ("details-narrow", true, [680.0, 760.0], 0, 1),
    ("details-small-description", false, [640.0, 480.0], 0, 1),
    ("details-small-dependencies", true, [640.0, 480.0], 0, 2),
    ("details-loading", true, [1200.0, 800.0], 1, 0),
    ("details-error", false, [640.0, 480.0], 2, 1),
    ("details-empty", false, [1200.0, 800.0], 3, 0),
    ("details-dependency-error", true, [680.0, 760.0], 4, 2),
    ("details-browse-loading", true, [1200.0, 800.0], 5, 0),
    ("details-library-loading", false, [1200.0, 800.0], 6, 0),
];
struct Capture {
    view: View,
    catalog: Catalog,
    directory: PathBuf,
    case: usize,
    staged: Instant,
    started: Instant,
    requested: bool,
    jumped: bool,
}
impl Capture {
    fn stage(&mut self, ctx: &egui::Context) {
        let (_, dark, size, state, _) = CASES[self.case];
        ctx.set_theme(if dark {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        });
        ctx.all_styles_mut(|s| s.animation_time = 0.0);
        for command in [
            egui::ViewportCommand::MinInnerSize(size.into()),
            egui::ViewportCommand::MaxInnerSize(size.into()),
            egui::ViewportCommand::InnerSize(size.into()),
        ] {
            ctx.send_viewport_cmd(command);
        }
        self.view = View::default();
        self.view.browse_page = state != 6;
        self.catalog = Catalog::default();
        support::seed(ctx, &mut self.view.discovery);
        match state {
            1 => {
                self.view.discovery.details.description = Load::Loading;
                self.view.discovery.details.project = Load::Loading;
            }
            2 => {
                self.view.discovery.details.description = Load::Failed(
                    "Could not read this information from CurseForge. Try again.".into(),
                );
                self.view.discovery.details.project =
                    Load::Failed("Could not load verified releases. Try again.".into());
            }
            3 => {
                self.view.discovery.details.description = Load::Ready(Default::default());
                self.view.discovery.details.chosen = Some(101);
            }
            4 => {
                self.view.discovery.details.references.insert(
                    9,
                    Load::Failed(
                        "Could not read this information from CurseForge. Try again.".into(),
                    ),
                );
            }
            5 => {
                self.view.discovery.selected = None;
                self.view.discovery.browser.begin(Default::default());
            }
            6 => {
                self.catalog.begin_scan(PathBuf::from("/synthetic/game"));
            }
            _ => {}
        }
        self.requested = false;
        self.jumped = false;
        self.staged = Instant::now();
    }
}
impl eframe::App for Capture {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.case == CASES.len() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        self.view.show(ui, &mut self.catalog);
        let jump = CASES[self.case].4;
        if jump != 0 && !self.jumped && self.staged.elapsed() > Duration::from_millis(250) {
            let (id, rect) = self
                .view
                .discovery
                .stats
                .detail_scroll
                .expect("project scroll area");
            let heading = if jump == 1 {
                self.view.discovery.stats.description_heading
            } else {
                self.view.discovery.stats.dependencies_heading
            }
            .unwrap();
            let mut state = egui::scroll_area::State::load(&ctx, id).unwrap();
            state.offset.y += (heading.top() - rect.top()).max(0.0);
            state.store(&ctx, id);
            self.jumped = true;
        }
        let screenshot = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = screenshot {
            let size = CASES[self.case].2;
            assert_eq!(image.size, [size[0] as usize, size[1] as usize]);
            let bytes = image
                .pixels
                .iter()
                .flat_map(|p| p.to_array())
                .collect::<Vec<_>>();
            image::save_buffer(
                self.directory.join(format!("{}.png", CASES[self.case].0)),
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
            && self.staged.elapsed() > Duration::from_millis(600)
            && ctx.content_rect().size() == CASES[self.case].2.into()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            self.requested = true;
        }
        assert!(
            self.started.elapsed() < Duration::from_secs(40),
            "Project captures timed out"
        );
        ctx.request_repaint_after(Duration::from_millis(30));
    }
}
fn main() -> eframe::Result {
    let directory = PathBuf::from(std::env::args_os().nth(1).expect("capture directory"));
    std::fs::create_dir_all(&directory).unwrap();
    eframe::run_native(
        "Synthetic project description verification",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            ui::setup(&cc.egui_ctx);
            let mut app = Capture {
                view: View::default(),
                catalog: Catalog::default(),
                directory,
                case: 0,
                staged: Instant::now(),
                started: Instant::now(),
                requested: false,
                jumped: false,
            };
            app.stage(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}
