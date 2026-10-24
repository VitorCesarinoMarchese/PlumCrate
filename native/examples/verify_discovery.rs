//! Capture the real CurseForge browser using synthetic provider responses, without network access.
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use ts4_mod_manager_core::{
    curseforge_browse::{BrowseQuery, parse_browse_response, parse_categories_response},
    curseforge_client::SourceLookupError,
};
use ts4_mod_manager_native::{
    catalog::Catalog,
    ui::{self, View},
};

const CASES: [(&str, bool, [f32; 2], u8); 16] = [
    ("browse-dark", true, [1200.0, 800.0], 0),
    ("browse-search-focus", true, [1200.0, 800.0], 7),
    ("browse-search-query", true, [1200.0, 800.0], 8),
    ("browse-search-small", true, [640.0, 480.0], 8),
    ("browse-small-top", true, [640.0, 480.0], 9),
    ("browse-light", false, [1200.0, 800.0], 0),
    ("browse-narrow", true, [680.0, 760.0], 0),
    ("browse-small", true, [640.0, 480.0], 0),
    ("project-dark", true, [1200.0, 800.0], 1),
    ("project-small", false, [640.0, 480.0], 1),
    ("browse-setup", true, [1200.0, 800.0], 2),
    ("browse-error", false, [640.0, 480.0], 3),
    ("browse-user", true, [1870.0, 1006.0], 0),
    ("browse-categories", true, [1200.0, 800.0], 4),
    ("browse-types", false, [680.0, 760.0], 5),
    ("browse-filters-small", true, [640.0, 480.0], 6),
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
    opened: bool,
    typed: bool,
}
impl Capture {
    fn stage(&mut self, ctx: &egui::Context) {
        let (_, dark, size, state) = CASES[self.case];
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
        self.view = View::default();
        self.view.browse_page = true;
        let generation = self.catalog.begin_scan("/synthetic/discovery-game".into());
        let owned = (1..=2).map(|id| serde_json::from_value(serde_json::json!({
            "key":format!("owned-{id}"), "id":format!("owned-{id}"), "name":format!("Downloaded fixture {id}"),
            "files":[format!("owned-{id}.package")], "mod_files":[format!("owned-{id}.package")],
            "preview":null, "source":"managed", "enabled":id==1, "group_path":[format!("owned-{id}")],
            "sourceUrl":format!("https://www.curseforge.com/sims4/mods/synthetic-owned-{id}"),
            "sourceAttachment":{"providerId":"curseforge", "projectId":id,
                "sourceUrl":format!("https://www.curseforge.com/sims4/mods/synthetic-owned-{id}"),
                "title":format!("Downloaded fixture {id}"), "confidence":100,
                "attachedBy":"user", "attachedAt":"2026-10-23T00:00:00Z"}
        })).unwrap()).collect();
        self.catalog.accept_scan(generation, Ok(owned));
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

        self.view.discovery.categories = parse_categories_response(
            r#"{"data":[
            {"id":10,"gameId":78062,"name":"Create a Sim","isClass":true},
            {"id":11,"gameId":78062,"name":"Clothing","classId":10,"parentCategoryId":10},
            {"id":13,"gameId":78062,"name":"Adult","classId":10,"parentCategoryId":11},
            {"id":14,"gameId":78062,"name":"Feminine","classId":10,"parentCategoryId":11},
            {"id":12,"gameId":78062,"name":"Body","classId":10,"parentCategoryId":10},
            {"id":42,"gameId":78062,"name":"Hairstyles","classId":10,"parentCategoryId":12},
            {"id":43,"gameId":78062,"name":"Feminine","classId":10,"parentCategoryId":12},
            {"id":20,"gameId":78062,"name":"Build / Buy","isClass":true},
            {"id":21,"gameId":78062,"name":"Build Mode","classId":20,"parentCategoryId":20},
            {"id":22,"gameId":78062,"name":"Lighting","classId":20,"parentCategoryId":21},
            {"id":30,"gameId":78062,"name":"Mods","isClass":true},
            {"id":40,"gameId":78062,"name":"Rooms / Lots","isClass":true},
            {"id":50,"gameId":78062,"name":"Sims / Households","isClass":true},
            {"id":60,"gameId":78062,"name":"Pets","isClass":true},
            {"id":70,"gameId":78062,"name":"Save Files","isClass":true},
            {"id":80,"gameId":78062,"name":"Worlds","isClass":true},
            {"id":90,"gameId":78062,"name":"Translations","isClass":true}
        ]}"#,
        )
        .unwrap();
        self.view.discovery.class_id = Some(10);
        self.view.discovery.category = Some(42);
        self.view.discovery.game_versions =
            vec!["1.128.90".into(), "1.127.41".into(), "1.126.78".into()];
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
        self.opened = false;
        self.typed = false;
        self.staged = Instant::now();
    }
}
impl eframe::App for Capture {
    fn raw_input_hook(&mut self, _: &egui::Context, input: &mut egui::RawInput) {
        if self.case >= CASES.len() { return; }
        if !self.opened && matches!(CASES[self.case].3, 7 | 8) {
            input.events.push(egui::Event::Key { key: egui::Key::F, physical_key: None,
                pressed: true, repeat: false, modifiers: egui::Modifiers::CTRL });
            self.opened = true;
        } else if CASES[self.case].3 == 8 && !self.typed {
            input.events.push(egui::Event::Text("Hair collection".into()));
            self.typed = true;
        }
        if !self.opened && self.staged.elapsed() > Duration::from_millis(200)
            && CASES[self.case].2[1] < 600.0 && CASES[self.case].3 == 0
            && let Some(card) = self.view.discovery.stats.first_card {
            input.events.push(egui::Event::PointerMoved(card.center()));
            input.events.push(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -140.0),
                phase: egui::TouchPhase::Move,
                modifiers: egui::Modifiers::NONE,
            });
        }
    }
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        if self.case == CASES.len() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        self.view.show(ui, &mut self.catalog);
        let state = CASES[self.case].3;
        if !self.opened && self.staged.elapsed() > Duration::from_millis(200) {
            match state {
                4 => egui::Popup::open_id(
                    &ctx,
                    self.view
                        .discovery
                        .stats
                        .category_control_id
                        .unwrap()
                        .with("popup"),
                ),
                5 => egui::Popup::open_id(
                    &ctx,
                    self.view
                        .discovery
                        .stats
                        .browse_by_id
                        .unwrap()
                        .with("popup"),
                ),
                6 => {
                    let id = self.view.discovery.stats.filters_toggle_id.unwrap();
                    let mut collapsing =
                        egui::collapsing_header::CollapsingState::load_with_default_open(
                            &ctx, id, true,
                        );
                    collapsing.set_open(true);
                    collapsing.store(&ctx);
                }
                _ => {}
            }
            self.opened = true;
        }
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
                opened: false,
                typed: false,
            };
            app.stage(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}
