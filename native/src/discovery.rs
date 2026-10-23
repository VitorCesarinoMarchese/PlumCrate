use crate::project_details::{Details, Load, ReviewRequest};
use egui::{RichText, vec2};
use std::{
    collections::VecDeque,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
};
use ts4_mod_manager_core::{browser_import::DependencyCompleteness, curseforge_details::BlockKind};
use ts4_mod_manager_core::{
    curseforge_browse::{
        BrowsePage, BrowseProject, BrowseQuery, BrowseSort, Category, category_options,
    },
    curseforge_client::{
        CurseForgeClient, CurseForgeTransport, SourceLookupError, UreqCurseForgeTransport,
    },
};
#[derive(Clone, Copy, PartialEq, Eq)]
enum DetailSection {
    Description,
    Dependencies,
}

#[derive(Default)]
pub struct Browser {
    pub active: BrowseQuery,
    pub page: Option<BrowsePage>,
    pub loading: bool,
    pub error: Option<SourceLookupError>,
    generation: u64,
    history: Vec<u32>,
    search_epoch: u64,
    cache: VecDeque<(BrowseQuery, BrowsePage)>,
    prefetch: Option<Prefetch>,
}
enum Prefetch {
    Pending(BrowseQuery),
    Failed(BrowseQuery),
}
impl Browser {
    pub fn begin(&mut self, query: BrowseQuery) -> u64 {
        self.search_epoch += 1;
        self.history.clear();
        self.cache.clear();
        self.prefetch = None;
        self.open_page(query)
    }
    pub fn navigate(&mut self, query: BrowseQuery) -> u64 {
        let same_search = BrowseQuery {
            index: query.index,
            ..self.active.clone()
        } == query;
        if self.search_epoch == 0 || !same_search {
            return self.begin(query);
        }
        if query.index > self.active.index {
            self.history.push(self.active.index);
        } else if self.history.last() == Some(&query.index) {
            self.history.pop();
        }
        self.open_page(query)
    }
    fn open_page(&mut self, query: BrowseQuery) -> u64 {
        self.generation += 1;
        self.active = query;
        self.page = self
            .cache
            .iter()
            .find(|(q, _)| q == &self.active)
            .map(|(_, page)| page.clone());
        if let Some(page) = self.page.clone() {
            self.cache_page(self.active.clone(), page);
        }
        self.loading = self.page.is_none();
        self.error = None;
        self.generation
    }
    fn cache_page(&mut self, query: BrowseQuery, page: BrowsePage) {
        self.cache.retain(|(q, _)| q != &query);
        self.cache.push_back((query, page));
        while self.cache.len() > 3 {
            self.cache.pop_front();
        }
    }
    pub fn take_prefetch(&mut self) -> Option<(u64, BrowseQuery)> {
        let query = self.next_query()?;
        if self.cache.iter().any(|(q, _)| q == &query)
            || matches!(&self.prefetch,Some(Prefetch::Pending(q)|Prefetch::Failed(q)) if q==&query)
        {
            return None;
        }
        self.prefetch = Some(Prefetch::Pending(query.clone()));
        Some((self.search_epoch, query))
    }
    pub fn awaiting_prefetch(&self) -> bool {
        self.loading && matches!(&self.prefetch,Some(Prefetch::Pending(q)) if q==&self.active)
    }
    pub fn accept_prefetch(
        &mut self,
        epoch: u64,
        query: BrowseQuery,
        result: Result<BrowsePage, SourceLookupError>,
    ) -> bool {
        if epoch != self.search_epoch
            || !matches!(&self.prefetch,Some(Prefetch::Pending(q)) if q==&query)
        {
            return false;
        }
        match result {
            Ok(page) => {
                self.prefetch = None;
                self.cache_page(query.clone(), page.clone());
                if self.active == query && self.loading {
                    self.page = Some(page);
                    self.loading = false;
                    self.error = None;
                }
            }
            Err(error) => {
                self.prefetch = Some(Prefetch::Failed(query.clone()));
                if self.active == query && self.loading {
                    self.loading = false;
                    self.error = Some(error);
                }
            }
        }
        true
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
                self.cache_page(self.active.clone(), page.clone());
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
    epoch: u64,
    query: BrowseQuery,
    key: String,
    load_filters: bool,
}
enum Completion {
    Page(u64, Result<BrowsePage, SourceLookupError>),
    Prefetched(u64, BrowseQuery, Result<BrowsePage, SourceLookupError>),
    Categories(u64, Result<Vec<Category>, SourceLookupError>),
    Versions(u64, Result<Vec<String>, SourceLookupError>),
}
struct Jobs {
    sender: mpsc::Sender<Request>,
    prefetch: mpsc::Sender<Request>,
    epoch: Arc<AtomicU64>,
    receiver: mpsc::Receiver<Completion>,
}
impl Jobs {
    fn start(ctx: egui::Context) -> std::io::Result<Self> {
        Self::with_client(ctx, |key| {
            CurseForgeClient::new(Some(key), UreqCurseForgeTransport::default())
        })
    }
    fn with_client<T, F>(ctx: egui::Context, factory: F) -> std::io::Result<Self>
    where
        T: CurseForgeTransport + 'static,
        F: Fn(&str) -> Result<CurseForgeClient<T>, SourceLookupError> + Send + Sync + 'static,
    {
        let (sender, requests) = mpsc::channel::<Request>();
        let (prefetch, speculative) = mpsc::channel::<Request>();
        let (results, receiver) = mpsc::channel();
        let epoch = Arc::new(AtomicU64::new(0));
        let factory = Arc::new(factory);
        let background_factory = factory.clone();
        let background_epoch = epoch.clone();
        let background_results = results.clone();
        let background_ctx = ctx.clone();
        std::thread::Builder::new()
            .name("curseforge-preload".into())
            .spawn(move || {
                while let Ok(mut request) = speculative.recv() {
                    while let Ok(next) = speculative.try_recv() {
                        request = next;
                    }
                    if request.epoch != background_epoch.load(Ordering::Acquire) {
                        continue;
                    }
                    let page = background_factory(&request.key)
                        .and_then(|client| client.browse_mods(&request.query));
                    if background_results
                        .send(Completion::Prefetched(request.epoch, request.query, page))
                        .is_err()
                    {
                        break;
                    }
                    background_ctx.request_repaint();
                }
            })?;
        let current_epoch = epoch.clone();
        std::thread::Builder::new()
            .name("curseforge-browser".into())
            .spawn(move || {
                while let Ok(mut request) = requests.recv() {
                    while let Ok(next) = requests.try_recv() {
                        request = next;
                    }
                    if request.epoch != current_epoch.load(Ordering::Acquire) {
                        continue;
                    }
                    let client = factory(&request.key);
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
                    if !request.load_filters
                        || request.epoch != current_epoch.load(Ordering::Acquire)
                    {
                        continue;
                    }
                    let categories = client
                        .as_ref()
                        .map_err(Clone::clone)
                        .and_then(|client| client.browse_categories());
                    if results
                        .send(Completion::Categories(request.epoch, categories))
                        .is_err()
                    {
                        break;
                    }
                    ctx.request_repaint();
                    if request.epoch != current_epoch.load(Ordering::Acquire) {
                        continue;
                    }
                    let versions = client.and_then(|client| client.browse_game_versions());
                    if results
                        .send(Completion::Versions(request.epoch, versions))
                        .is_err()
                    {
                        break;
                    }
                    ctx.request_repaint();
                }
            })?;
        Ok(Self {
            sender,
            prefetch,
            epoch,
            receiver,
        })
    }
}

enum Submission {
    Refresh,
    Navigate,
}

#[derive(Default)]
pub struct DiscoverView {
    pub browser: Browser,
    pub query: String,
    pub sort: BrowseSort,
    pub category: Option<u64>,
    pub class_id: Option<u64>,
    pub game_version: Option<String>,
    pub categories: Vec<Category>,
    pub game_versions: Vec<String>,
    pub selected: Option<BrowseProject>,
    pub details: Details,
    detail_jump: Option<DetailSection>,
    pub stats: Stats,
    import_request: Option<ReviewRequest>,
    library_request: Option<crate::catalog::EntryId>,
    jobs: Option<Jobs>,
    key: Option<String>,
    category_error: Option<SourceLookupError>,
    version_error: Option<SourceLookupError>,
    category_search: String,
    version_search: String,
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
    pub browser_import: Option<egui::Rect>,
    pub dependency_reviews: Vec<(u64, egui::Rect)>,
    pub release: Option<egui::Rect>,
    pub description_jump: Option<egui::Rect>,
    pub dependencies_jump: Option<egui::Rect>,
    pub detail_scroll: Option<(egui::Id,egui::Rect)>,
    pub description_heading: Option<egui::Rect>,
    pub dependencies_heading: Option<egui::Rect>,
    pub next: Option<egui::Rect>,
    pub previous: Option<egui::Rect>,
    pub settings: Option<egui::Rect>,
    pub cards: usize,
    pub skeleton_cards: usize,
    pub browse_by: Option<egui::Rect>,
    pub category: Option<egui::Rect>,
    pub game_version: Option<egui::Rect>,
    pub sort: Option<egui::Rect>,
    pub clear_filters: Option<egui::Rect>,
    pub content_types: Vec<(u64, egui::Rect)>,
    pub filters_toggle: Option<egui::Rect>,
    pub filters_toggle_id: Option<egui::Id>,
    pub browse_by_id: Option<egui::Id>,
    pub category_control_id: Option<egui::Id>,
}
impl DiscoverView {
    pub fn take_import_request(&mut self) -> Option<ReviewRequest> { self.import_request.take() }
    pub fn take_library_request(&mut self) -> Option<crate::catalog::EntryId> { self.library_request.take() }
    pub fn with_cache(directory: std::path::PathBuf) -> Self {
        Self {
            previews: crate::preview::Previews::with_cache(directory),
            ..Self::default()
        }
    }
    fn submit(&mut self, ctx: &egui::Context, query: BrowseQuery, submission: Submission) {
        self.selected = None;
        self.details.clear();
        let generation = match submission {
            Submission::Refresh => self.browser.begin(query.clone()),
            Submission::Navigate => self.browser.navigate(query.clone()),
        };
        if let Some(jobs) = &self.jobs {
            jobs.epoch
                .store(self.browser.search_epoch, Ordering::Release);
        }
        if !self.browser.loading || self.browser.awaiting_prefetch() {
            return;
        }
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
        let jobs = self.jobs.as_ref().unwrap();
        jobs.epoch
            .store(self.browser.search_epoch, Ordering::Release);
        if jobs
            .sender
            .send(Request {
                generation,
                epoch: self.browser.search_epoch,
                query,
                key,
                load_filters: self.categories.is_empty()
                    || self.game_versions.is_empty()
                    || self.category_error.is_some()
                    || self.version_error.is_some(),
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
                class_id: self.class_id,
                game_version: self.game_version.clone(),
                index: 0,
            },
            Submission::Refresh,
        );
    }
    fn preload(&mut self) {
        let (Some(jobs), Some(key)) = (&self.jobs, &self.key) else {
            return;
        };
        let Some((epoch, query)) = self.browser.take_prefetch() else {
            return;
        };
        if jobs
            .prefetch
            .send(Request {
                generation: self.browser.generation,
                epoch,
                query: query.clone(),
                key: key.clone(),
                load_filters: false,
            })
            .is_err()
        {
            self.browser
                .accept_prefetch(epoch, query, Err(SourceLookupError::Network));
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui, key: Option<&str>, focus_search: bool, catalog: &crate::catalog::Catalog) -> bool {
        self.stats = Stats::default();
        let key = key
            .map(str::trim)
            .filter(|key| !key.is_empty())
            .map(str::to_owned);
        if self.browser.generation == 0 || self.key != key {
            self.key = key;
            self.categories.clear();
            self.game_versions.clear();
            self.category_error = None;
            self.version_error = None;
            self.search(ui.ctx());
        }
        if let Some(jobs) = &self.jobs {
            for event in jobs.receiver.try_iter() {
                match event {
                    Completion::Page(generation, result) => {
                        self.browser.accept(generation, result);
                    }
                    Completion::Prefetched(epoch, query, result) => {
                        self.browser.accept_prefetch(epoch, query, result);
                    }
                    Completion::Versions(generation, result)
                        if generation == self.browser.search_epoch =>
                    {
                        match result {
                            Ok(versions) => {
                                self.game_versions = versions;
                                self.version_error = None;
                            }
                            Err(error) => self.version_error = Some(error),
                        }
                    }
                    Completion::Categories(generation, result)
                        if generation == self.browser.search_epoch =>
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
        self.preload();
        self.previews.begin_frame(ui.ctx(), self.browser.generation);
        if focus_search {
            self.selected = None;
        }
        self.details.sync(ui.ctx(), self.selected.as_ref(), self.key.as_deref());
        if self.selected.is_some() {
            if ui.memory(|memory| memory.is_above_modal_layer(ui.layer_id()))
                && !egui::Popup::is_any_open(ui.ctx())
                && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
            {
                self.selected = None;
                self.details.clear();
            } else {
                self.details(ui, catalog);
                return false;
            }
        }
        ui.horizontal(|ui| {
            ui.label(RichText::new("Browse mods").size(27.0).strong().color(ui.visuals().text_color()));
            ui.label(
                RichText::new("The Sims 4 · CurseForge").color(ui.visuals().weak_text_color()),
            );
        });
        let short = ui.ctx().content_rect().height() < 600.0;
        if short {
            egui::ScrollArea::vertical()
                .id_salt("curseforge-short-window")
                .auto_shrink([false, false])
                .show(ui, |ui| self.content(ui, focus_search, false, catalog))
                .inner
        } else {
            self.content(ui, focus_search, true, catalog)
        }
    }
    fn content(&mut self, ui: &mut egui::Ui, focus_search: bool, scroll_cards: bool, catalog: &crate::catalog::Catalog) -> bool {
        ui.add_space(16.0);
        let mut search = false;
        egui::Frame::new()
            .fill(ui.visuals().extreme_bg_color)
            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
            .corner_radius(8)
            .inner_margin(egui::Margin::symmetric(12, 0))
            .show(ui, |ui| {
                crate::ui::text_on_fill(ui, ui.visuals().extreme_bg_color);
                ui.horizontal(|ui| {
                    ui.add(crate::ui::Icon::Search.image(ui.visuals().weak_text_color(), 18.0));
                    let response = ui.add_sized(
                        [(ui.available_width() - 106.0).max(80.0), 42.0],
                        egui::TextEdit::singleline(&mut self.query).text_color(crate::ui::readable_text(ui.visuals().text_color(), &[ui.visuals().text_edit_bg_color()]))
                            .hint_text(egui::RichText::new("Search CurseForge mods and custom content").color(crate::ui::readable_text(ui.visuals().weak_text_color(), &[ui.visuals().text_edit_bg_color()])))
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
        let filters_changed = if ui.ctx().content_rect().height() < 600.0 {
            let count = [
                self.class_id.is_some(),
                self.category.is_some(),
                self.game_version.is_some(),
            ]
            .into_iter()
            .filter(|active| *active)
            .count();
            let response = egui::CollapsingHeader::new(format!("Filters ({count})"))
                .id_salt("browse-filters")
                .show(ui, |ui| self.filters(ui));
            self.stats.filters_toggle = Some(response.header_response.rect);
            self.stats.filters_toggle_id = Some(response.header_response.id);
            response.body_returned.unwrap_or(false)
        } else {
            self.filters(ui)
        };
        if search || filters_changed {
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
                    self.submit(ui.ctx(), self.browser.active.clone(), Submission::Navigate);
                }
            });
            return settings_requested;
        }
        if self.browser.loading {
            ui.label("Searching CurseForge...");
            ui.add_space(12.0);
            self.stats.skeleton_cards = loading_cards(ui);
            return false;
        }
        if let Some(error) = &self.category_error {
            ui.label(
                RichText::new(format!("Categories unavailable. {}", error_message(error)))
                    .color(ui.visuals().warn_fg_color),
            );
        }
        if let Some(error) = &self.version_error {
            ui.label(
                RichText::new(format!(
                    "Game versions unavailable. {}",
                    error_message(error)
                ))
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
                self.submit(ui.ctx(), self.browser.previous_query().unwrap(), Submission::Navigate);
            }
            let next = ui.add_enabled(
                self.browser.next_query().is_some(),
                egui::Button::new("Next"),
            );
            self.stats.next = Some(next.rect);
            if next.clicked() {
                self.submit(ui.ctx(), self.browser.next_query().unwrap(), Submission::Navigate);
            }
        });
        if self.browser.loading {
            return false;
        }
        let Some(page) = self.browser.page.clone() else { return false; };
        self.previews.begin_frame(ui.ctx(), self.browser.generation);
        self.preload();
        ui.add_space(12.0);
        if page.projects.is_empty() {
            ui.add_space(24.0);
            ui.heading("No matching mods");
            ui.label("Try another name or clear the filters.");
            return false;
        }
        self.cards(ui, &page.projects, scroll_cards, catalog);
        false
    }

    fn filters(&mut self, ui: &mut egui::Ui) -> bool {
        let old = (
            self.sort,
            self.class_id,
            self.category,
            self.game_version.clone(),
        );
        ui.horizontal_wrapped(|ui| {
            ui.allocate_ui_with_layout(
                vec2(180.0, 60.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(180.0);
                    let label = ui.label("Browse by");
                    let name = self
                        .categories
                        .iter()
                        .find(|c| Some(c.id) == self.class_id && c.is_class == Some(true))
                        .map(|c| c.name.as_str())
                        .unwrap_or("All");
                    let response = egui::ComboBox::from_id_salt("browse-content-type")
                        .width(180.0)
                        .selected_text(name)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.class_id, None, "All");
                            let mut classes: Vec<_> = self
                                .categories
                                .iter()
                                .filter(|c| c.is_class == Some(true))
                                .collect();
                            classes.sort_by_key(|c| c.display_index.unwrap_or(u32::MAX));
                            for class in classes {
                                let response = ui.selectable_value(
                                    &mut self.class_id,
                                    Some(class.id),
                                    &class.name,
                                );
                                self.stats.content_types.push((class.id, response.rect));
                            }
                        })
                        .response
                        .labelled_by(label.id);
                    self.stats.browse_by = Some(response.rect);
                    self.stats.browse_by_id = Some(response.id);
                    if old.1 != self.class_id {
                        self.category = None;
                        self.category_search.clear();
                    }
                },
            );
            ui.allocate_ui_with_layout(
                vec2(260.0, 60.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(260.0);
                    let label = ui.label("Category");
                    let options = self
                        .class_id
                        .map(|id| category_options(&self.categories, id))
                        .unwrap_or_default();
                    let name = options
                        .iter()
                        .find(|(id, _)| Some(*id) == self.category)
                        .map(|(_, name)| name.as_str())
                        .unwrap_or("All categories");
                    let response = ui
                        .add_enabled_ui(self.class_id.is_some() && !options.is_empty(), |ui| {
                            egui::ComboBox::from_id_salt("browse-category")
                                .width(260.0)
                                .height(250.0)
                                .selected_text(name)
                                .show_ui(ui, |ui| {
                                    ui.add(
                                        egui::TextEdit::singleline(&mut self.category_search).text_color(crate::ui::readable_text(ui.visuals().text_color(), &[ui.visuals().text_edit_bg_color()]))
                                            .hint_text(egui::RichText::new("Find a category").color(crate::ui::readable_text(ui.visuals().weak_text_color(), &[ui.visuals().text_edit_bg_color()])))
                                            .desired_width(240.0),
                                    );
                                    ui.selectable_value(&mut self.category, None, "All categories");
                                    let filter = self.category_search.trim().to_lowercase();
                                    for (id, name) in &options {
                                        if name.to_lowercase().contains(&filter) {
                                            ui.selectable_value(
                                                &mut self.category,
                                                Some(*id),
                                                name,
                                            );
                                        }
                                    }
                                })
                                .response
                        })
                        .inner
                        .labelled_by(label.id)
                        .on_disabled_hover_text(
                            "Choose a content type in Browse by to see its categories.",
                        );
                    self.stats.category = Some(response.rect);
                    self.stats.category_control_id = Some(response.id);
                },
            );
            ui.allocate_ui_with_layout(
                vec2(142.0, 60.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(142.0);
                    let label = ui.label("Game Version");
                    let response = egui::ComboBox::from_id_salt("browse-version")
                        .width(142.0)
                        .height(250.0)
                        .selected_text(self.game_version.as_deref().unwrap_or("All versions"))
                        .show_ui(ui, |ui| {
                            ui.add(
                                egui::TextEdit::singleline(&mut self.version_search).text_color(crate::ui::readable_text(ui.visuals().text_color(), &[ui.visuals().text_edit_bg_color()]))
                                    .hint_text(egui::RichText::new("Find a version").color(crate::ui::readable_text(ui.visuals().weak_text_color(), &[ui.visuals().text_edit_bg_color()])))
                                    .desired_width(130.0),
                            );
                            ui.selectable_value(&mut self.game_version, None, "All versions");
                            for version in &self.game_versions {
                                if version.contains(self.version_search.trim()) {
                                    ui.selectable_value(
                                        &mut self.game_version,
                                        Some(version.clone()),
                                        version,
                                    );
                                }
                            }
                        })
                        .response
                        .labelled_by(label.id);
                    self.stats.game_version = Some(response.rect);
                },
            );
            ui.allocate_ui_with_layout(
                vec2(155.0, 60.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(155.0);
                    let label = ui.label("Sort by");
                    let response = egui::ComboBox::from_id_salt("browse-sort")
                        .width(155.0)
                        .selected_text(self.sort.label())
                        .show_ui(ui, |ui| {
                            for sort in [
                                BrowseSort::Relevance,
                                BrowseSort::Popular,
                                BrowseSort::Updated,
                                BrowseSort::Created,
                                BrowseSort::Downloads,
                                BrowseSort::Name,
                            ] {
                                ui.selectable_value(&mut self.sort, sort, sort.label());
                            }
                        })
                        .response
                        .labelled_by(label.id);
                    self.stats.sort = Some(response.rect);
                },
            );
            ui.allocate_ui_with_layout(
                vec2(104.0, 60.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.add_space(23.0);
                    let response = ui.add_enabled(
                        self.class_id.is_some()
                            || self.category.is_some()
                            || self.game_version.is_some(),
                        egui::Button::new("Clear filters"),
                    );
                    self.stats.clear_filters = Some(response.rect);
                    if response.clicked() {
                        self.class_id = None;
                        self.category = None;
                        self.game_version = None;
                        self.category_search.clear();
                        self.version_search.clear();
                    }
                },
            );
        });
        old != (
            self.sort,
            self.class_id,
            self.category,
            self.game_version.clone(),
        )
    }
    fn cards(&mut self, ui: &mut egui::Ui, projects: &[BrowseProject], scroll: bool, catalog: &crate::catalog::Catalog) {
        let columns = ((ui.available_width() + 16.0) / 280.0).floor().max(1.0) as usize;
        let width = (ui.available_width() - (columns - 1) as f32 * 16.0) / columns as f32;
        let rows = projects.len().div_ceil(columns);
        let generation = self.browser.generation;
        let mut draw = |ui: &mut egui::Ui, visible: std::ops::Range<usize>| {
            for row in visible {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    for project in projects.iter().skip(row * columns).take(columns) {
                        let local = catalog.curseforge_entry(project.id);
                        let frame = egui::Frame::new()
                            .fill(ui.visuals().extreme_bg_color)
                            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                            .corner_radius(12);
                        let content_width = width - frame.total_margin().sum().x;
                        let mut download_rect = egui::Rect::NOTHING;
                        let response = frame
                            .show(ui, |ui| {
                                crate::ui::text_on_fill(ui, ui.visuals().extreme_bg_color);
                                ui.vertical(|ui| {
                                    ui.set_width(content_width);
                                    ui.spacing_mut().item_spacing.y = 0.0;
                                    self.card_photo(
                                        ui,
                                        project.preview_url.as_deref(),
                                        vec2(content_width, 140.0),
                                    );
                                    egui::Frame::new().inner_margin(14).show(ui, |ui| {
                                        ui.set_width(content_width - 28.0);
                                        ui.spacing_mut().item_spacing.y = 8.0;
                                        ui.set_min_height(238.0);
                                        ui.allocate_ui(vec2(ui.available_width(), 64.0), |ui| {
                                            ui.set_min_height(64.0);
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
                                        let colors = crate::ui::chrome_colors(ui);
                                        let (label, icon, fill, ink, hint) = if local.is_some() {
                                            let fill = ui.visuals().selection.bg_fill;
                                            ("View in library", crate::ui::Icon::Check, fill,
                                                crate::ui::readable_text(ui.visuals().text_color(), &[fill]),
                                                "Already in your library. Open the local mod.")
                                        } else {
                                            ("Downloads", crate::ui::Icon::Download, colors.primary, colors.primary_text,
                                                "Download in your browser, then install automatically.")
                                        };
                                        let action = ui.add_sized(
                                            [ui.available_width(), 34.0],
                                            egui::Button::image_and_text(
                                                icon.image(ink, 16.0),
                                                RichText::new(label).color(ink),
                                            ).fill(fill).stroke(egui::Stroke::NONE),
                                        ).on_hover_text(hint);
                                        action.widget_info(|| egui::WidgetInfo::labeled(
                                            egui::WidgetType::Button, ui.is_enabled(), format!("{label}: {}", project.name)));
                                        if action.hovered() || action.has_focus() {
                                            ui.painter().rect_stroke(action.rect.shrink(2.0), 7,
                                                egui::Stroke::new(1.5, ink), egui::StrokeKind::Inside);
                                        }
                                        download_rect = action.rect;
                                        self.stats.card_actions.push(action.rect);
                                        if action.clicked() {
                                            if let Some(identity) = local {
                                                self.library_request = Some(identity.clone());
                                            } else {
                                                self.import_request = Some(ReviewRequest::ProjectId(project.id));
                                            }
                                        }
                                    });
                                });
                            })
                            .response;
                        self.stats.cards += 1;
                        if self.stats.first_card.is_none() {
                            self.stats.first_card = Some(response.rect.intersect(ui.clip_rect()));
                        }
                        let click = ui.interact(
                            egui::Rect::from_min_max(response.rect.min,
                                egui::pos2(response.rect.right(), download_rect.top())),
                            ui.id().with(project.id),
                            egui::Sense::click(),
                        );
                        if self.stats.first_card_id.is_none() {
                            self.stats.first_card_id = Some(click.id);
                        }
                        click
                            .clone()
                            .on_hover_text(format!("View details for {}", project.name))
                            .on_hover_cursor(egui::CursorIcon::PointingHand);
                        click.widget_info(|| egui::WidgetInfo::labeled(
                            egui::WidgetType::Button, ui.is_enabled(), format!("View details for {}", project.name)));
                        if click.clicked() {
                            self.selected = Some(project.clone());
                        }
                        if click.hovered() || click.has_focus() {
                            ui.painter().rect_stroke(
                                response.rect,
                                12,
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
                .show_rows(ui, 408.0, rows, draw);
        } else {
            draw(ui, 0..rows);
        }
    }
    fn card_photo(&mut self, ui: &mut egui::Ui, url: Option<&str>, size: egui::Vec2) {
        let url = url.filter(|url| safe_url(url));
        let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        ui.painter()
            .rect_filled(rect, 6, ui.visuals().widgets.inactive.weak_bg_fill);
        match self.previews.load(ui.ctx(), url, None) {
            crate::preview::State::Ready(texture) => {
                egui::Image::from_texture(texture)
                    .uv(crate::ui::cover_uv(texture.size, rect.size()))
                    .corner_radius(6)
                    .paint_at(ui, rect);
            }
            crate::preview::State::Pending => {}
            crate::preview::State::Missing => {
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "No Preview",
                    egui::FontId::proportional(13.0),
                    ui.visuals().weak_text_color(),
                );
            }
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
                crate::skeleton::block(ui, size);
            }
            crate::preview::State::Missing => {}
        }
    }
    fn details(&mut self, ui: &mut egui::Ui, catalog: &crate::catalog::Catalog) {
        let project = self.selected.as_ref().unwrap().clone();
        let back = ui.horizontal_wrapped(|ui| {
            let back=ui.add(egui::Button::image_and_text(crate::ui::Icon::Back.image(ui.visuals().text_color(),16.0),"Back to results"));
            let description=ui.button("Description");
            self.stats.description_jump=Some(description.rect);
            if description.clicked() {self.detail_jump=Some(DetailSection::Description);}
            let dependencies=ui.button("Dependencies");
            self.stats.dependencies_jump=Some(dependencies.rect);
            if dependencies.clicked() {self.detail_jump=Some(DetailSection::Dependencies);}
            back
        }).inner;
        self.stats.back = Some(back.rect);
        if back.clicked() {
            self.selected = None;
            self.details.clear();
            return;
        }
        ui.add_space(16.0);
        let scroll=egui::ScrollArea::vertical()
            .id_salt(("curseforge-project", project.id))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width().min(1160.0));
                ui.horizontal_top(|ui| {
                    self.photo(ui, project.preview_url.as_deref(), vec2(80.0, 80.0));
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&project.name).size(26.0).strong().color(ui.visuals().text_color()));
                        ui.label(
                            RichText::new(author(&project)).color(ui.visuals().weak_text_color()),
                        );
                        if catalog.curseforge_entry(project.id).is_some() {
                            ui.label(RichText::new("In your library").color(ui.visuals().weak_text_color()));
                        }
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
                if !project.categories.is_empty() {
                    ui.add_space(12.0);
                    ui.horizontal_wrapped(|ui| {
                        for category in &project.categories {
                            ui.label(RichText::new(&category.name).color(ui.visuals().weak_text_color()));
                        }
                    });
                }
                ui.add_space(16.0);
                if let Some(url) = project.source_url.as_deref().filter(|url| safe_url(url)) {
                    ui.horizontal_wrapped(|ui| {
                        if ui.button("Open on CurseForge").clicked() {
                            ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                        }
                        if ui.button("Copy link").clicked() {
                            ui.ctx().copy_text(url.into());
                        }
                        let download = ui.add_enabled(!matches!(self.details.project,Load::Loading),egui::Button::new("Download via website…"));
                        self.stats.browser_import = Some(download.rect);
                        if download.clicked() {
                            self.import_request = Some(match &self.details.project {
                                Load::Ready(loaded) if self.details.chosen.is_some() => ReviewRequest::Loaded {project:loaded.clone(),file_id:self.details.chosen.unwrap()},
                                _=>ReviewRequest::ProjectId(project.id),
                            });
                        }

                    });
                }
                ui.add_space(24.0);
                if ui.available_width() >= 920.0 {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x=24.0;
                        let width=(ui.available_width()-340.0-24.0).min(720.0);
                        ui.allocate_ui_with_layout(vec2(width,0.0),egui::Layout::top_down(egui::Align::Min),|ui| {ui.set_min_width(width);self.description_section(ui,&project);});
                        ui.allocate_ui_with_layout(vec2(340.0,0.0),egui::Layout::top_down(egui::Align::Min),|ui| {ui.set_min_width(340.0);self.dependency_section(ui,&project);});
                    });
                } else {
                    self.description_section(ui,&project);
                    ui.add_space(24.0);
                    self.dependency_section(ui,&project);
                }
                ui.add_space(24.0);
                if !project.screenshots.is_empty() {
                    ui.label(RichText::new("Screenshots").size(18.0).strong().color(ui.visuals().text_color()));
                    ui.add_space(12.0);
                }
                for url in &project.screenshots {
                    self.photo(ui, Some(url), vec2(ui.available_width().min(780.0), 420.0));
                    ui.add_space(12.0);
                }
            });
        self.stats.detail_scroll=Some((scroll.id,scroll.inner_rect));
    }
    fn description_section(&mut self, ui:&mut egui::Ui, project:&BrowseProject) {
        let heading=ui.label(RichText::new("Description").size(18.0).strong().color(ui.visuals().text_color()));
        self.stats.description_heading=Some(heading.rect);
        if self.detail_jump==Some(DetailSection::Description) {heading.scroll_to_me_animation(Some(egui::Align::Min),egui::style::ScrollAnimation::none());self.detail_jump=None;}
        ui.add_space(12.0);
        match &self.details.description {
            Load::Loading | Load::Idle => {
                ui.label("Loading project description...");
                ui.add_space(12.0);
                crate::skeleton::paragraphs(ui);
                ui.add_space(16.0);
                crate::skeleton::paragraphs(ui);
            },
            Load::Failed(error) => {
                ui.label(error);
                if ui.button("Retry description").clicked() { self.details.retry_description(ui.ctx(),project,self.key.as_deref()); }
            },
            Load::Ready(document) => {
                if document.blocks.is_empty() { ui.label("The author has not provided a description."); }
                for (index,block) in document.blocks.iter().enumerate() {
                    match block.kind {
                        BlockKind::Heading(level) => {
                            if index>0 {ui.add_space(12.0);}
                            ui.label(RichText::new(&block.text).size(if level <= 2 {20.0} else {17.0}).strong().color(ui.visuals().text_color()));
                        },
                        BlockKind::Rule => { ui.separator(); },
                        BlockKind::Image => {
                            if let Some(url)=&block.image {
                                match self.previews.load(ui.ctx(),Some(url.as_str()),None) {
                                    crate::preview::State::Ready(texture) => { ui.add(egui::Image::from_texture(texture).max_size(vec2(ui.available_width(),420.0)).corner_radius(6)); },
                                    crate::preview::State::Pending => crate::skeleton::block(ui,vec2(ui.available_width(),180.0)),
                                    crate::preview::State::Missing => {},
                                }
                            }
                            if !block.text.is_empty() { ui.label(RichText::new(&block.text).size(12.0).color(ui.visuals().weak_text_color())); }
                        },
                        _ => {
                            let mut text=RichText::new(&block.text).size(14.0);
                            if block.kind==BlockKind::Preformatted { text=text.monospace(); }
                            if block.kind==BlockKind::Quote { text=text.italics(); }
                            ui.add(egui::Label::new(text).wrap().selectable(true));
                        },
                    }
                    for link in &block.links { ui.hyperlink_to(&link.label,link.url.as_str()); }
                    ui.add_space(6.0);
                }
            },
        }
    }
    fn dependency_section(&mut self, ui:&mut egui::Ui, summary:&BrowseProject) {
        let mut retry_references=false;
        let heading=ui.label(RichText::new("Dependencies for this release").size(18.0).strong().color(ui.visuals().text_color()));
        self.stats.dependencies_heading=Some(heading.rect);
        if self.detail_jump==Some(DetailSection::Dependencies) {heading.scroll_to_me_animation(Some(egui::Align::Min),egui::style::ScrollAnimation::none());self.detail_jump=None;}
        ui.add_space(12.0);
        match &self.details.project {
            Load::Idle | Load::Loading => {
                ui.label("Loading releases and dependencies...");
                ui.add_space(12.0);
                crate::skeleton::block(ui,vec2(ui.available_width(),32.0));
                ui.add_space(16.0);
                crate::skeleton::paragraphs(ui);
            },
            Load::Failed(error) => {
                ui.label(error);
                if ui.button("Retry releases").clicked() { self.details.retry_project(ui.ctx(),summary,self.key.as_deref()); }
            },
            Load::Ready(project) => {
                let response=egui::ComboBox::from_id_salt("project-detail-release")
                    .width(ui.available_width()).wrap_mode(egui::TextWrapMode::Truncate)
                    .selected_text(project.releases.iter().find(|r|Some(r.release.file_id)==self.details.chosen).map(|r|r.release.name.as_str()).unwrap_or("No verified releases"))
                    .show_ui(ui,|ui| {
                        for release in &project.releases {
                            ui.selectable_value(&mut self.details.chosen,Some(release.release.file_id),format!("{} · {}",release.release.name,release.release.published_at.format("%Y-%m-%d")));
                        }
                    });
                self.stats.release=Some(response.response.rect);
                if let Some(file)=project.releases.iter().find(|r|Some(r.release.file_id)==self.details.chosen) {
                    ui.add_space(8.0);
                    ui.add(egui::Label::new(RichText::new(&file.release.file_name).size(12.0).color(ui.visuals().weak_text_color())).wrap());
                    match file.dependencies.completeness {
                        DependencyCompleteness::Unknown => { ui.label("Dependency information is unavailable for this release. Check the author's description."); },
                        DependencyCompleteness::Partial => { ui.label(RichText::new("Some dependency information could not be read. This list may be incomplete.").strong().color(ui.visuals().text_color())); },
                        DependencyCompleteness::Complete if file.dependencies.relations.is_empty() => { ui.label("No dependencies declared for this release. Check the author's description for additional requirements."); },
                        _ => {},
                    }
                    ui.add_space(8.0);
                    for relation in &file.dependencies.relations {
                        ui.separator();
                        ui.label(RichText::new(relation.kind.label()).size(12.0).color(if relation.kind==ts4_mod_manager_core::browser_import::RelationKind::Incompatible {ui.visuals().text_color()} else {ui.visuals().weak_text_color()}));
                        match self.details.references.get(&relation.project_id) {
                            Some(Load::Ready(reference)) => {
                                ui.add(egui::Label::new(RichText::new(&reference.title).strong().color(ui.visuals().text_color())).wrap());
                                ui.horizontal_wrapped(|ui| {
                                    ui.hyperlink_to("Open project",reference.source_url.as_str());
                                    if relation.kind.can_install() {
                                        let review=ui.button("Review download…");
                                        self.stats.dependency_reviews.push((relation.project_id,review.rect));
                                        if review.clicked() { self.import_request=Some(ReviewRequest::ProjectId(reference.project_id)); }
                                    }
                                });
                            },
                            Some(Load::Failed(error)) => { ui.label(format!("CurseForge project #{}",relation.project_id)); ui.label(error); },
                            _ => { ui.label(format!("CurseForge project #{}",relation.project_id)); crate::skeleton::block(ui,vec2(ui.available_width()*0.8,14.0)); },
                        }
                        if !relation.kind.can_install() {
                            ui.label(match relation.kind {
                                ts4_mod_manager_core::browser_import::RelationKind::Incompatible=>"Do not install this together with the selected release.",
                                ts4_mod_manager_core::browser_import::RelationKind::Embedded=>"The provider lists this as an embedded library. No separate install is needed.",
                                _=>"Check the author's instructions for this relationship before installing separately.",
                            });
                        }
                        ui.add_space(8.0);
                    }
                    if !file.dependencies.relations.is_empty() {
                        ui.label(RichText::new("Review each dependency separately. CurseForge identifies projects, but does not guarantee a compatible version.").size(12.0).color(ui.visuals().weak_text_color()));
                    }
                    retry_references=self.details.references.values().any(|r|matches!(r,Load::Failed(_))) && ui.button("Retry dependency lookups").clicked();
                } else { ui.label("No public release supports verified automatic installation. Check the description and project page."); }
                if !project.complete { ui.label("Only part of the release history is available."); }
            },
        }
        if retry_references {self.details.retry_references();}
        let links=match &self.details.description {Load::Ready(document)=>document.author_projects(),_=>vec![]};
        if !links.is_empty() {
            ui.add_space(24.0);
            ui.label(RichText::new("Author-linked mods").size(18.0).strong().color(ui.visuals().text_color()));
            ui.label("These links come from the description. They are not declared dependencies.");
            for link in links {
                ui.add_space(12.0);
                ui.hyperlink_to(&link.label,link.url.as_str());
                match self.details.author_projects.get(&link.url) {
                    Some(Load::Ready(reference)) => {
                        ui.label(&reference.title);
                        if ui.button("Review linked mod…").clicked() {self.import_request=Some(ReviewRequest::ProjectId(reference.project_id));}
                    },
                    Some(Load::Loading) => {ui.label("Checking the exact CurseForge project...");crate::skeleton::block(ui,vec2(ui.available_width()*0.8,14.0));},
                    state => {
                        if let Some(Load::Failed(error))=state {ui.label(error);}
                        if ui.button("Inspect linked mod").clicked() {self.details.inspect_author(ui.ctx(),link.url.clone(),self.key.as_deref());}
                    },
                }
            }
        }
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

fn loading_cards(ui: &mut egui::Ui) -> usize {
    let columns = ((ui.available_width() + 16.0) / 280.0).floor().max(1.0) as usize;
    let width = (ui.available_width() - (columns - 1) as f32 * 16.0) / columns as f32;
    egui::ScrollArea::vertical()
        .id_salt("pending-discovery-cards")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for _ in 0..2 {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    for _ in 0..columns {
                        let frame = egui::Frame::new()
                            .fill(ui.visuals().extreme_bg_color)
                            .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                            .corner_radius(12);
                        let inner_width = width - frame.total_margin().sum().x;
                        frame.show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.set_width(inner_width);
                                ui.spacing_mut().item_spacing.y = 0.0;
                                crate::skeleton::block(ui, vec2(inner_width, 140.0));
                                egui::Frame::new().inner_margin(14).show(ui, |ui| {
                                    ui.set_width(inner_width - 28.0);
                                    ui.set_min_height(238.0);
                                    ui.spacing_mut().item_spacing.y = 8.0;
                                    for (fraction, height) in [
                                        (0.9, 16.0),
                                        (0.7, 16.0),
                                        (0.55, 11.0),
                                        (1.0, 13.0),
                                        (0.94, 13.0),
                                        (0.7, 13.0),
                                        (0.45, 11.0),
                                        (0.8, 11.0),
                                        (1.0, 34.0),
                                    ] {
                                        crate::skeleton::block(ui, vec2(ui.available_width() * fraction, height));
                                    }
                                });
                            });
                        });
                    }
                });
            }
        });
    columns * 2
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

#[cfg(test)]
mod preload_tests {
    use super::*;
    use std::{
        sync::{Condvar, Mutex},
        time::{Duration, Instant},
    };
    use ts4_mod_manager_core::curseforge_client::{CurseForgeRequest, CurseForgeTransport};
    struct Provider {
        indices: Arc<Mutex<Vec<u32>>>,
        gate: Option<Arc<(Mutex<bool>, Condvar)>>,
        started: Option<mpsc::Sender<()>>,
    }
    impl CurseForgeTransport for Provider {
        fn get(&self, request: &CurseForgeRequest) -> Result<(u16, String), SourceLookupError> {
            let url = url::Url::parse(&request.url).unwrap();
            if !url.path().ends_with("/mods/search") {
                return Ok((200, r#"{"data":[]}"#.into()));
            }
            let index = url
                .query_pairs()
                .find(|(k, _)| k == "index")
                .unwrap()
                .1
                .parse::<u32>()
                .unwrap();
            self.indices.lock().unwrap().push(index);
            if index == 24
                && let Some(gate) = &self.gate
            {
                self.started.as_ref().unwrap().send(()).unwrap();
                let (lock, signal) = &**gate;
                let (_guard, timeout) = signal
                    .wait_timeout_while(lock.lock().unwrap(), Duration::from_secs(2), |open| !*open)
                    .unwrap();
                if timeout.timed_out() {
                    return Err(SourceLookupError::Network);
                }
            }
            Ok((200,serde_json::json!({"data":[{"id":index+1,"gameId":78062,"name":format!("Page {index} project")}],"pagination":{"index":index,"pageSize":24,"resultCount":24,"totalCount":72}}).to_string()))
        }
    }
    #[test]
    fn blocked_prefetch_does_not_delay_a_new_foreground_search() {
        let (started, receiver) = mpsc::channel();
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        let worker_gate = gate.clone();
        let indices = Arc::new(Mutex::new(vec![]));
        let jobs = Jobs::with_client(egui::Context::default(), move |key| {
            CurseForgeClient::new(
                Some(key),
                Provider {
                    indices: indices.clone(),
                    gate: Some(worker_gate.clone()),
                    started: Some(started.clone()),
                },
            )
        })
        .unwrap();
        jobs.epoch.store(1, Ordering::Release);
        jobs.prefetch
            .send(Request {
                generation: 1,
                epoch: 1,
                query: BrowseQuery {
                    index: 24,
                    ..Default::default()
                },
                key: "synthetic".into(),
                load_filters: false,
            })
            .unwrap();
        receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        jobs.epoch.store(2, Ordering::Release);
        jobs.sender
            .send(Request {
                generation: 2,
                epoch: 2,
                query: BrowseQuery {
                    search: "new search".into(),
                    ..Default::default()
                },
                key: "synthetic".into(),
                load_filters: false,
            })
            .unwrap();
        let completion = jobs.receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        *gate.0.lock().unwrap() = true;
        gate.1.notify_all();
        assert!(matches!(completion, Completion::Page(2, Ok(_))));
    }
    fn frame(
        ctx: &egui::Context,
        view: &mut DiscoverView,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    vec2(1200.0, 800.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                view.show(ui, Some("synthetic"), false, &crate::catalog::Catalog::default());
            },
        );
        output.textures_delta.clear();
        output
    }
    #[test]
    fn next_click_renders_cached_cards_and_fetches_only_one_page_ahead() {
        let ctx = egui::Context::default();
        crate::ui::setup(&ctx);
        let indices = Arc::new(Mutex::new(vec![]));
        let requests = indices.clone();
        let jobs = Jobs::with_client(ctx.clone(), move |key| {
            CurseForgeClient::new(
                Some(key),
                Provider {
                    indices: requests.clone(),
                    gate: None,
                    started: None,
                },
            )
        })
        .unwrap();
        let mut view = DiscoverView {
            jobs: Some(jobs),
            ..Default::default()
        };
        let deadline = Instant::now() + Duration::from_secs(3);
        while !view.browser.cache.iter().any(|(q, _)| q.index == 24) {
            frame(&ctx, &mut view, vec![]);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(*indices.lock().unwrap(), vec![0, 24]);
        let pos = view.stats.next.unwrap().center();
        let output = frame(
            &ctx,
            &mut view,
            vec![
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
            ],
        );
        assert_eq!(view.browser.active.index, 24);
        assert!(!view.browser.loading);
        assert_eq!(view.stats.skeleton_cards, 0);
        assert!(output.shapes.iter().any(|s|matches!(&s.shape,egui::epaint::Shape::Text(t) if t.galley.text().contains("Page 24 project"))));
        while !view.browser.cache.iter().any(|(q, _)| q.index == 48) {
            frame(&ctx, &mut view, vec![]);
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(*indices.lock().unwrap(), vec![0, 24, 48]);
    }
}
