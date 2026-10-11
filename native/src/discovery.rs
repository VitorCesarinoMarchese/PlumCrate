use egui::{RichText, vec2};
use std::sync::mpsc;
use ts4_mod_manager_core::{
    curseforge_browse::{BrowsePage, BrowseProject, BrowseQuery, BrowseSort, Category},
    curseforge_client::{CurseForgeClient, SourceLookupError, UreqCurseForgeTransport},
};

#[derive(Default)]
pub struct Browser {
    pub active: BrowseQuery,
    pub page: Option<BrowsePage>,
    pub loading: bool,
    pub error: Option<SourceLookupError>,
    generation: u64,
    history: Vec<u32>,
}
impl Browser {
    pub fn begin(&mut self, query: BrowseQuery) -> u64 {
        if query.search != self.active.search
            || query.sort != self.active.sort
            || query.category != self.active.category
            || query.index == 0
        {
            self.history.clear();
        } else if query.index > self.active.index {
            self.history.push(self.active.index);
        } else if self.history.last() == Some(&query.index) {
            self.history.pop();
        }
        self.generation += 1;
        self.active = query;
        self.page = None;
        self.loading = true;
        self.error = None;
        self.generation
    }
    pub fn accept(
        &mut self,
        generation: u64,
        result: Result<BrowsePage, SourceLookupError>,
    ) -> bool {
        if generation != self.generation {
            return false;
        }
        self.loading = false;
        match result {
            Ok(page) => {
                self.page = Some(page);
                self.error = None;
            }
            Err(error) => {
                self.error = Some(error);
            }
        }
        true
    }
    pub fn next_query(&self) -> Option<BrowseQuery> {
        Some(BrowseQuery {
            index: self.page.as_ref()?.next_index()?,
            ..self.active.clone()
        })
    }
    pub fn previous_query(&self) -> Option<BrowseQuery> {
        Some(BrowseQuery {
            index: *self.history.last()?,
            ..self.active.clone()
        })
    }
    pub fn error_message(&self) -> Option<&'static str> {
        self.error.as_ref().map(error_message)
    }
}
fn error_message(error: &SourceLookupError) -> &'static str {
    match error {
        SourceLookupError::MissingApiKey => {
            "Add your CurseForge API key in Settings, then save settings to browse mods."
        }
        SourceLookupError::Unauthorized => {
            "CurseForge rejected the API key. Check the key in Settings and save it again."
        }
        SourceLookupError::RateLimited => {
            "CurseForge is receiving too many requests. Please wait a moment, then try again."
        }
        SourceLookupError::Network => {
            "Could not reach CurseForge. Check your connection, then try again."
        }
        SourceLookupError::ProviderUnavailable => "CurseForge is unavailable. Try again later.",
        SourceLookupError::InvalidResponse | SourceLookupError::NoUsableEvidence => {
            "CurseForge returned an unreadable response. Try again later."
        }
    }
}
struct Request {
    generation: u64,
    query: BrowseQuery,
    key: String,
}
enum Completion {
    Page(u64, Result<BrowsePage, SourceLookupError>),
    Categories(u64, Result<Vec<Category>, SourceLookupError>),
}
struct Jobs {
    sender: mpsc::Sender<Request>,
    receiver: mpsc::Receiver<Completion>,
}
impl Jobs {
    fn start(ctx: egui::Context) -> std::io::Result<Self> {
        let (sender, requests) = mpsc::channel::<Request>();
        let (results, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("curseforge-browser".into())
            .spawn(move || {
                while let Ok(mut request) = requests.recv() {
                    while let Ok(next) = requests.try_recv() {
                        request = next;
                    }
                    let client = CurseForgeClient::new(
                        Some(&request.key),
                        UreqCurseForgeTransport::default(),
                    );
                    let page = client
                        .as_ref()
                        .map_err(Clone::clone)
                        .and_then(|client| client.browse_mods(&request.query));
                    if results
                        .send(Completion::Page(request.generation, page))
                        .is_err()
                    {
                        break;
                    }
                    ctx.request_repaint();
                    let categories = client.and_then(|client| client.browse_categories());
                    if results
                        .send(Completion::Categories(request.generation, categories))
                        .is_err()
                    {
                        break;
                    }
                    ctx.request_repaint();
                }
            })?;
        Ok(Self { sender, receiver })
    }
}

#[derive(Default)]
pub struct DiscoverView {
    pub browser: Browser,
    pub query: String,
    pub sort: BrowseSort,
    pub category: Option<u64>,
    pub categories: Vec<Category>,
    pub selected: Option<BrowseProject>,
    pub stats: Stats,
    jobs: Option<Jobs>,
    key: Option<String>,
    category_error: Option<SourceLookupError>,
    previews: crate::preview::Previews,
}
#[derive(Default)]
pub struct Stats {
    pub search: Option<egui::Rect>,
    pub search_button: Option<egui::Rect>,
    pub first_card: Option<egui::Rect>,
    pub first_card_id: Option<egui::Id>,
    pub card_actions: Vec<egui::Rect>,
    pub back: Option<egui::Rect>,
    pub next: Option<egui::Rect>,
    pub previous: Option<egui::Rect>,
    pub settings: Option<egui::Rect>,
    pub cards: usize,
}
impl DiscoverView {
    pub fn with_cache(directory: std::path::PathBuf) -> Self {
        Self {
            previews: crate::preview::Previews::with_cache(directory),
            ..Self::default()
        }
    }
    fn submit(&mut self, ctx: &egui::Context, query: BrowseQuery) {
        self.selected = None;
        let generation = self.browser.begin(query.clone());
        let Some(key) = self.key.clone().filter(|key| !key.trim().is_empty()) else {
            self.browser
                .accept(generation, Err(SourceLookupError::MissingApiKey));
            return;
        };
        if self.jobs.is_none() {
            match Jobs::start(ctx.clone()) {
                Ok(jobs) => self.jobs = Some(jobs),
                Err(_) => {
                    self.browser
                        .accept(generation, Err(SourceLookupError::Network));
                    return;
                }
            }
        }
        if self
            .jobs
            .as_ref()
            .unwrap()
            .sender
            .send(Request {
                generation,
                query,
                key,
            })
            .is_err()
        {
            self.browser
                .accept(generation, Err(SourceLookupError::Network));
        }
    }
    fn search(&mut self, ctx: &egui::Context) {
        self.submit(
            ctx,
            BrowseQuery {
                search: self.query.trim().into(),
                sort: self.sort,
                category: self.category,
                index: 0,
                ..Default::default()
            },
        );
    }
    pub fn show(&mut self, ui: &mut egui::Ui, key: Option<&str>, focus_search: bool) -> bool {
        self.stats = Stats::default();
        let key = key
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .map(str::to_owned);
        if self.browser.generation == 0 || self.key != key {
            self.key = key;
            self.categories.clear();
            self.category_error = None;
            self.search(ui.ctx());
        }
        if let Some(jobs) = &self.jobs {
            for event in jobs.receiver.try_iter() {
                match event {
                    Completion::Page(generation, result) => {
                        self.browser.accept(generation, result);
                    }
                    Completion::Categories(generation, result)
                        if generation == self.browser.generation =>
                    {
                        match result {
                            Ok(categories) => {
                                self.categories = categories;
                                self.category_error = None;
                            }
                            Err(error) => self.category_error = Some(error),
                        }
                    }
                    _ => {}
                }
            }
        }
        self.previews.begin_frame(ui.ctx(), self.browser.generation);
        if focus_search {
            self.selected = None;
        }
        if self.selected.is_some() {
            if ui.memory(|memory| memory.is_above_modal_layer(ui.layer_id()))
                && !egui::Popup::is_any_open(ui.ctx())
                && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
            {
                self.selected = None;
            } else {
                self.details(ui);
                return false;
            }
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new("Browse mods").size(27.0).strong());
            ui.label(
                RichText::new("The Sims 4 · CurseForge").color(ui.visuals().weak_text_color()),
            );
        });
        let short = ui.ctx().content_rect().height() < 600.0;
        if short {
            egui::ScrollArea::vertical()
                .id_salt("curseforge-short-window")
                .auto_shrink([false, false])
                .show(ui, |ui| self.content(ui, focus_search, false))
                .inner
        } else {
            self.content(ui, focus_search, true)
        }
    }
    fn content(&mut self, ui: &mut egui::Ui, focus_search: bool, scroll_cards: bool) -> bool {
        ui.add_space(16.0);
        let mut search = false;
        egui::Frame::new()
            .fill(ui.visuals().extreme_bg_color)
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .corner_radius(8)
            .inner_margin(egui::Margin::symmetric(12, 0))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(crate::ui::Icon::Search.image(ui.visuals().weak_text_color(), 18.0));
                    let response = ui.add_sized(
                        [(ui.available_width() - 106.0).max(80.0), 42.0],
                        egui::TextEdit::singleline(&mut self.query)
                            .hint_text("Search CurseForge mods and custom content")
                            .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(0, 12))),
                    );
                    self.stats.search = Some(response.rect);
                    response.clone().on_hover_text("Search CurseForge (Ctrl+F)");
                    if focus_search {
                        response.request_focus();
                    }
                    search = response.lost_focus()
                        && ui.input(|input| input.key_pressed(egui::Key::Enter));
                    let button = ui.add_sized(
                        [88.0, 36.0],
                        egui::Button::new("Search")
                            .fill(ui.visuals().selection.bg_fill)
                            .stroke(ui.visuals().selection.stroke),
                    );
                    self.stats.search_button = Some(button.rect);
                    search |= button.clicked();
                });
            });
        ui.add_space(12.0);
        let old = (self.sort, self.category);
        ui.horizontal_wrapped(|ui| {
            ui.label("Category");
            let name = self
                .categories
                .iter()
                .find(|category| Some(category.id) == self.category)
                .map(|category| category.name.as_str())
                .unwrap_or("All categories");
            egui::ComboBox::from_id_salt("browse-category")
                .width(180.0)
                .selected_text(name)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.category, None, "All categories");
                    for category in &self.categories {
                        ui.selectable_value(&mut self.category, Some(category.id), &category.name);
                    }
                });
            ui.label("Sort by");
            egui::ComboBox::from_id_salt("browse-sort")
                .width(150.0)
                .selected_text(self.sort.label())
                .show_ui(ui, |ui| {
                    for sort in [
                        BrowseSort::Popular,
                        BrowseSort::Downloads,
                        BrowseSort::Updated,
                        BrowseSort::Name,
                    ] {
                        ui.selectable_value(&mut self.sort, sort, sort.label());
                    }
                });
        });
        if search || old != (self.sort, self.category) {
            self.search(ui.ctx());
        }
        ui.add_space(16.0);
        if let Some(message) = self.browser.error_message() {
            ui.heading(
                if self.browser.error == Some(SourceLookupError::MissingApiKey) {
                    "Connect to CurseForge"
                } else {
                    "Search unavailable"
                },
            );
            ui.label(message);
            ui.add_space(12.0);
            let mut settings_requested = false;
            ui.horizontal(|ui| {
                let response = ui.button("Open Settings");
                self.stats.settings = Some(response.rect);
                settings_requested = response.clicked();
                if self.browser.error != Some(SourceLookupError::MissingApiKey)
                    && ui.button("Try again").clicked()
                {
                    self.submit(ui.ctx(), self.browser.active.clone());
                }
            });
            return settings_requested;
        }
        if self.browser.loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Searching CurseForge...");
            });
            return false;
        }
        if let Some(error) = &self.category_error {
            ui.label(
                RichText::new(format!("Categories unavailable. {}", error_message(error)))
                    .color(ui.visuals().warn_fg_color),
            );
        }
        let Some(page) = self.browser.page.clone() else {
            return false;
        };
        ui.horizontal_wrapped(|ui| {
            let total = page.pagination.total_count;
            ui.label(
                RichText::new(format!("{total} results")).color(ui.visuals().weak_text_color()),
            );
            if total > 0 {
                let start = page.pagination.index + 1;
                let end = page.pagination.index + page.pagination.result_count;
                ui.label(
                    RichText::new(format!("Showing {start}–{end}"))
                        .color(ui.visuals().weak_text_color()),
                );
            }
            let previous = ui.add_enabled(
                self.browser.previous_query().is_some(),
                egui::Button::new("Previous"),
            );
            self.stats.previous = Some(previous.rect);
            if previous.clicked() {
                self.submit(ui.ctx(), self.browser.previous_query().unwrap());
            }
            let next = ui.add_enabled(
                self.browser.next_query().is_some(),
                egui::Button::new("Next"),
            );
            self.stats.next = Some(next.rect);
            if next.clicked() {
                self.submit(ui.ctx(), self.browser.next_query().unwrap());
            }
        });
        if self.browser.loading {
            return false;
        }
        ui.add_space(12.0);
        if page.projects.is_empty() {
            ui.add_space(24.0);
            ui.heading("No matching mods");
            ui.label("Try another name or select All categories.");
            return false;
        }
        self.cards(ui, &page.projects, scroll_cards);
        false
    }
    fn cards(&mut self, ui: &mut egui::Ui, projects: &[BrowseProject], scroll: bool) {
        let columns = ((ui.available_width() + 16.0) / 280.0).floor().max(1.0) as usize;
        let width = (ui.available_width() - (columns - 1) as f32 * 16.0) / columns as f32;
        let rows = projects.len().div_ceil(columns);
        let generation = self.browser.generation;
        let mut draw = |ui: &mut egui::Ui, visible: std::ops::Range<usize>| {
            for row in visible {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    for project in projects.iter().skip(row * columns).take(columns) {
                        let frame = egui::Frame::new()
                            .fill(ui.visuals().extreme_bg_color)
                            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                            .corner_radius(8)
                            .inner_margin(14);
                        let response = frame
                            .show(ui, |ui| {
                                ui.vertical(|ui| {
                                    ui.set_width(width - 28.0);
                                    ui.set_min_height(238.0);
                                    ui.allocate_ui(vec2(ui.available_width(), 64.0), |ui| {
                                        ui.set_min_height(64.0);
                                        ui.horizontal_top(|ui| {
                                            self.photo(
                                                ui,
                                                project.preview_url.as_deref(),
                                                vec2(60.0, 60.0),
                                            );
                                            ui.vertical(|ui| {
                                                bounded_label(ui, &project.name, 16.0, 2);
                                                ui.add(
                                                    egui::Label::new(
                                                        RichText::new(author(project))
                                                            .size(12.0)
                                                            .color(ui.visuals().weak_text_color()),
                                                    )
                                                    .truncate(),
                                                );
                                            });
                                        });
                                    });
                                    ui.add_space(10.0);
                                    ui.allocate_ui(vec2(ui.available_width(), 54.0), |ui| {
                                        ui.set_min_height(54.0);
                                        bounded_label(ui, &project.summary, 14.0, 3);
                                    });
                                    ui.add_space(8.0);
                                    let category = project
                                        .categories
                                        .first()
                                        .map(|category| category.name.as_str())
                                        .unwrap_or("Uncategorized");
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(category)
                                                .size(12.0)
                                                .color(ui.visuals().weak_text_color()),
                                        )
                                        .truncate(),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "{} downloads · {}",
                                            downloads(project.downloads),
                                            updated(project)
                                        ))
                                        .size(12.0)
                                        .color(ui.visuals().weak_text_color()),
                                    );
                                    let action = ui.add_sized(
                                        [ui.available_width(), 34.0],
                                        egui::Button::new("View details"),
                                    );
                                    self.stats.card_actions.push(action.rect);
                                    if action.clicked() {
                                        self.selected = Some(project.clone());
                                    }
                                });
                            })
                            .response;
                        self.stats.cards += 1;
                        if self.stats.first_card.is_none() {
                            self.stats.first_card = Some(response.rect.intersect(ui.clip_rect()));
                        }
                        let click = ui.interact(
                            response.rect,
                            ui.id().with(project.id),
                            egui::Sense::click(),
                        );
                        if self.stats.first_card_id.is_none() {
                            self.stats.first_card_id = Some(click.id);
                        }
                        click
                            .clone()
                            .on_hover_text(format!("View {}", project.name));
                        if click.clicked() {
                            self.selected = Some(project.clone());
                        }
                        if click.hovered() || click.has_focus() {
                            ui.painter().rect_stroke(
                                response.rect,
                                8,
                                ui.visuals().selection.stroke,
                                egui::StrokeKind::Inside,
                            );
                        }
                    }
                });
            }
        };
        if scroll {
            egui::ScrollArea::vertical()
                .id_salt(("curseforge-results", generation))
                .auto_shrink([false, false])
                .show_rows(ui, 268.0, rows, draw);
        } else {
            draw(ui, 0..rows);
        }
    }
    fn photo(&mut self, ui: &mut egui::Ui, url: Option<&str>, size: egui::Vec2) {
        let url = url.filter(|url| safe_url(url));
        match self.previews.load(ui.ctx(), url, None) {
            crate::preview::State::Ready(texture) => {
                ui.add(
                    egui::Image::from_texture(texture)
                        .max_size(size)
                        .corner_radius(6),
                );
            }
            crate::preview::State::Pending => {
                ui.allocate_ui(size, |ui| {
                    ui.centered_and_justified(|ui| {
                        ui.spinner();
                    });
                });
            }
            crate::preview::State::Missing => {}
        }
    }
    fn details(&mut self, ui: &mut egui::Ui) {
        let project = self.selected.as_ref().unwrap().clone();
        let back = ui.add(egui::Button::image_and_text(
            crate::ui::Icon::Back.image(ui.visuals().text_color(), 16.0),
            "Back to results",
        ));
        self.stats.back = Some(back.rect);
        if back.clicked() {
            self.selected = None;
            return;
        }
        ui.add_space(16.0);
        egui::ScrollArea::vertical()
            .id_salt(("curseforge-project", project.id))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal_top(|ui| {
                    self.photo(ui, project.preview_url.as_deref(), vec2(80.0, 80.0));
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&project.name).size(26.0).strong());
                        ui.label(
                            RichText::new(author(&project)).color(ui.visuals().weak_text_color()),
                        );
                        ui.label(
                            RichText::new(format!(
                                "{} downloads · Updated {}",
                                downloads(project.downloads),
                                updated(&project)
                            ))
                            .color(ui.visuals().weak_text_color()),
                        );
                    });
                });
                ui.add_space(16.0);
                ui.label(&project.summary);
                ui.add_space(12.0);
                ui.horizontal_wrapped(|ui| {
                    for category in &project.categories {
                        ui.label(
                            RichText::new(&category.name).color(ui.visuals().weak_text_color()),
                        );
                    }
                });
                ui.add_space(16.0);
                if let Some(url) = project.source_url.as_deref().filter(|url| safe_url(url)) {
                    ui.horizontal(|ui| {
                        if ui.button("Open on CurseForge").clicked() {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                        }
                        if ui.button("Copy link").clicked() {
                            ui.ctx().copy_text(url.into());
                        }
                    });
                }
                ui.add_space(24.0);
                if !project.screenshots.is_empty() {
                    ui.label(RichText::new("Screenshots").size(18.0).strong());
                    ui.add_space(12.0);
                }
                for url in &project.screenshots {
                    self.photo(ui, Some(url), vec2(ui.available_width().min(780.0), 420.0));
                    ui.add_space(12.0);
                }
            });
    }
}
fn safe_url(value: &str) -> bool {
    url::Url::parse(value).is_ok_and(|url| {
        url.scheme() == "https"
            && url.host_str().is_some()
            && url.username().is_empty()
            && url.password().is_none()
    })
}
fn author(project: &BrowseProject) -> String {
    if project.authors.is_empty() {
        "Author not provided".into()
    } else {
        format!("by {}", project.authors.join(", "))
    }
}
fn updated(project: &BrowseProject) -> &str {
    project
        .updated
        .as_deref()
        .and_then(|date| date.split('T').next())
        .unwrap_or("Date not provided")
}
fn downloads(count: f64) -> String {
    if count >= 1_000_000.0 {
        format!("{:.1}M", count / 1_000_000.0)
    } else if count >= 1_000.0 {
        format!("{:.1}K", count / 1_000.0)
    } else {
        format!("{count:.0}")
    }
}

fn bounded_label(ui: &mut egui::Ui, text: &str, size: f32, rows: usize) {
    let mut job = egui::text::LayoutJob::simple(
        text.into(),
        egui::FontId::proportional(size),
        ui.visuals().text_color(),
        ui.available_width(),
    );
    job.wrap.max_rows = rows;
    let galley = ui.fonts_mut(|fonts| fonts.layout_job(job));
    ui.add(egui::Label::new(galley)).on_hover_text(text);
}
