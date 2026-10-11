//! Capture the real CurseForge browser using synthetic provider responses, without network access.
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use ts4_mod_manager_core::{
    curseforge_browse::{BrowseQuery, parse_browse_response},
    curseforge_client::SourceLookupError,
};
use ts4_mod_manager_native::{
    catalog::Catalog,
    ui::{self, View},
};

const CASES: [(&str, bool, [f32; 2], u8); 8] = [
    ("browse-dark", true, [1200.0, 800.0], 0),
    ("browse-light", false, [1200.0, 800.0], 0),
    ("browse-narrow", true, [680.0, 760.0], 0),
    ("browse-small", true, [640.0, 480.0], 0),
    ("project-dark", true, [1200.0, 800.0], 1),
    ("project-small", false, [640.0, 480.0], 1),
    ("browse-setup", true, [1200.0, 800.0], 2),
    ("browse-error", false, [640.0, 480.0], 3),
];
struct FixtureBytes(egui::load::Bytes);
impl egui::load::BytesLoader for FixtureBytes {
    fn id(&self) -> &str {
        "synthetic-curseforge-images"
    }
    fn load(&self, _: &egui::Context, uri: &str) -> egui::load::BytesLoadResult {
        if uri.starts_with("https://fixture.invalid/") {
            Ok(egui::load::BytesPoll::Ready {
                size: None,
                bytes: self.0.clone(),
                mime: Some("image/png".into()),
            })
        } else {
            Err(egui::load::LoadError::NotSupported)
        }
    }
    fn forget(&self, _: &str) {}
    fn forget_all(&self) {}
    fn byte_size(&self) -> usize {
        self.0.len()
    }
}
struct Capture {
    view: View,
    catalog: Catalog,
    case: usize,
    directory: PathBuf,
    requested: bool,
    staged: Instant,
    started: Instant,
}
impl Capture {
    fn stage(&mut self, ctx: &egui::Context) {
        let (_, dark, size, state) = CASES[self.case];
        ctx.set_theme(if dark {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        });
        for command in [
            egui::ViewportCommand::MinInnerSize(size.into()),
            egui::ViewportCommand::MaxInnerSize(size.into()),
            egui::ViewportCommand::InnerSize(size.into()),
        ] {
            ctx.send_viewport_cmd(command);
        }
        self.view = View::default();
        self.view.browse_page = true;
        let projects=(0..24).map(|index|serde_json::json!({
            "id":index+1,"gameId":78062,
            "name":if index==0 {"Synthetic hair collection with a long descriptive project title".into()} else {format!("Synthetic collection {}",index+1)},
            "summary":"Synthetic test content. A collection of items for your Sims, with a longer summary to check the card layout and text wrapping.",
            "authors":[{"name":"Fixture creator"}],
            "downloadCount":1200000-index*1000,"dateModified":"2026-10-01T10:00:00Z",
            "categories":[{"id":42,"gameId":78062,"name":"Hair"}],
            "links":{"websiteUrl":"https://www.curseforge.com/"},
            "logo":if index%3==1 {serde_json::Value::Null} else {serde_json::json!({"url":format!("https://fixture.invalid/{index}.png")})},
            "screenshots":[{"url":"https://fixture.invalid/preview.png"}]
        })).collect::<Vec<_>>();
        let page=parse_browse_response(&serde_json::json!({"data":projects,"pagination":{"index":0,"pageSize":24,"resultCount":24,"totalCount":120}}).to_string()).unwrap();
        // Supply local synthetic image bytes under synthetic HTTPS URIs without networking.

        self.view.discovery.categories = page.projects[0].categories.clone();
        let generation = self.view.discovery.browser.begin(BrowseQuery::default());
        self.view
            .discovery
            .browser
            .accept(generation, Ok(page.clone()));
        match state {
            1 => self.view.discovery.selected = Some(page.projects[0].clone()),
            2 => {
                self.view
                    .discovery
                    .browser
                    .accept(generation, Err(SourceLookupError::MissingApiKey));
            }
            3 => {
                self.view
                    .discovery
                    .browser
                    .accept(generation, Err(SourceLookupError::RateLimited));
            }
            _ => {}
        }
        self.requested = false;
        self.staged = Instant::now();
    }
}
impl eframe::App for Capture {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.view.show(ui, &mut self.catalog);
        if let Some(image) = ctx.input(|input| {
            input.events.iter().find_map(|event| {
                if let egui::Event::Screenshot { image, .. } = event {
                    Some(image.clone())
                } else {
                    None
                }
            })
        }) {
            assert_eq!(
                image.size,
                [
                    CASES[self.case].2[0] as usize,
                    CASES[self.case].2[1] as usize
                ]
            );
            if CASES[self.case].3 <= 1 {
                assert!(
                    image
                        .pixels
                        .iter()
                        .filter(|pixel| pixel.to_array() == [30, 120, 95, 255])
                        .count()
                        > 100,
                    "synthetic preview must be visible"
                );
            }
            let bytes = image
                .pixels
                .iter()
                .flat_map(|pixel| pixel.to_array())
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
            && self.staged.elapsed() > Duration::from_millis(500)
            && ctx.content_rect().size() == CASES[self.case].2.into()
            && !ctx.has_pending_images()
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            self.requested = true;
        }
        assert!(
            self.started.elapsed() < Duration::from_secs(60),
            "Discovery capture timed out"
        );
        ctx.request_repaint_after(Duration::from_millis(30));
    }
}
fn main() -> eframe::Result {
    let directory = PathBuf::from(std::env::args().nth(1).expect("capture directory"));
    std::fs::create_dir_all(&directory).unwrap();
    let fixture = tempfile::tempdir().unwrap();
    let photo = fixture.path().join("preview.png");
    let mut img = image::RgbImage::new(320, 200);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        *pixel = image::Rgb(if x < 100 {
            [30, 120, 95]
        } else if y < 100 {
            [235, 185, 65]
        } else {
            [75, 95, 160]
        });
    }
    img.save(&photo).unwrap();
    eframe::run_native(
        "Synthetic CurseForge browser verification",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size([1200.0, 800.0]),
            ..Default::default()
        },
        Box::new(move |cc| {
            ui::setup(&cc.egui_ctx);
            cc.egui_ctx
                .add_bytes_loader(std::sync::Arc::new(FixtureBytes(
                    std::fs::read(&photo).unwrap().into(),
                )));
            let mut app = Capture {
                view: View::default(),
                catalog: Catalog::default(),
                case: 0,
                directory,
                requested: false,
                staged: Instant::now(),
                started: Instant::now(),
            };
            app.stage(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}
