use crate::{
    catalog::{Catalog, EntryId},
    management::{Action, Event, Identity, Job, Mutations, Output},
    settings::{Settings, Theme},
};
use std::path::PathBuf;
#[path = "controls/preferences.rs"]
mod preferences;
#[path = "controls/activity.rs"]
mod activity;
pub use preferences::PreferencesSection;

struct Operation {
    job: Job,
    status: String,
    error: bool,
}
enum Dialog {
    Import {
        identity: Identity,
        zip: bool,
        path: PathBuf,
        name: String,
    },
    Rename(Identity, String),
    Manual(Identity, String),
    Confirm {
        identity: Identity,
        action: Action,
        title: String,
        lines: Vec<String>,
    },
    SettingsImport(Settings, bool),
}
enum PickPurpose {
    Import(Identity, bool),
    GameFolder,
    DownloadsFolder,
    SettingsImport,
    SettingsExport(Settings),
}
struct QueuedDownload {
    request: crate::project_details::ReviewRequest,
    project_id: u64,
    title: String,
    root: PathBuf,
}
struct PendingPicker {
    purpose: PickPurpose,
    receiver: std::sync::mpsc::Receiver<Option<PathBuf>>,
}
pub struct Controls {
    pub app_updates: crate::app_update::AppUpdates,
    pub updates: crate::updates::UpdateState,
    pub browser_import: crate::browser_import::BrowserImport,
    pub activity_button_rect: Option<egui::Rect>,
    pub activity_popup_rect: Option<egui::Rect>,
    pub browser_cancel_rect: Option<egui::Rect>,
    browser_pending_start: bool,
    download_queue: std::collections::VecDeque<QueuedDownload>,
    active_download: Option<u64>,
    pub queue_remove_rects: Vec<egui::Rect>,
    pub queue_skip_rect: Option<egui::Rect>,
    pub check_updates_rect: Option<egui::Rect>,
    pub record_release_rect: Option<egui::Rect>,
    pub open_update_page_rect: Option<egui::Rect>,
    release_choice: Option<(String, u64)>,
    pub settings_page: bool,
    preferences: preferences::PreferencesView,
    theme_json: String,
    theme_edit: Option<crate::settings::CustomTheme>,
    theme_edit_original: Option<String>,
    pub confirm_rect: Option<egui::Rect>,
    pub import_rect: Option<egui::Rect>,
    previews: crate::preview::Previews,
    activity: activity::Activity,
    worker: Mutations,
    managed: PathBuf,
    trash: PathBuf,
    pub settings: Settings,
    saved_settings: Settings,
    settings_path: PathBuf,
    next: u64,
    recovering: bool,
    operations: Vec<Operation>,
    dialog: Option<Dialog>,
    candidates: Option<(
        Identity,
        Vec<ts4_mod_manager_core::source_candidates::SourceCandidate>,
    )>,
    trash_entries: Option<(Identity, Vec<ts4_mod_manager_core::lifecycle::TrashEntry>)>,
    picker: Option<PendingPicker>,
    picker_directory: PathBuf,
    #[cfg(test)]
    picker_sender: Option<std::sync::mpsc::Sender<Option<PathBuf>>>,
    include_key: bool,
    notice: Option<String>,
    pub error: Option<String>,
    pub refresh: Option<PathBuf>,
    pub roots_changed: Option<PathBuf>,
}
impl Controls {
    pub fn new(
        managed: PathBuf,
        home: PathBuf,
        settings: Settings,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> std::io::Result<Self> {
        let theme_edit = settings
            .custom_themes
            .iter()
            .find(|t| settings.active_custom_theme.as_ref() == Some(&t.name))
            .cloned();
        let theme_edit_original = settings.active_custom_theme.clone();
        Ok(Self {
            app_updates: crate::app_update::AppUpdates::default(),
            updates: crate::updates::UpdateState::default(),
            browser_import: crate::browser_import::BrowserImport::new(&home),
            activity_button_rect: None,
            activity_popup_rect: None,
            browser_cancel_rect: None,
            browser_pending_start: false,
            download_queue: Default::default(),
            active_download: None,
            queue_remove_rects: vec![],
            queue_skip_rect: None,
            check_updates_rect: None,
            record_release_rect: None,
            open_update_page_rect: None,
            release_choice: None,
            settings_page: false,
            preferences: Default::default(),
            theme_json: String::new(),
            theme_edit,
            theme_edit_original,
            confirm_rect: None,
            import_rect: None,
            previews: crate::preview::Previews::with_cache(home.join(".cache/ts4-mod-manager/photos")),
            activity: Default::default(),
            worker: Mutations::start_with_recovery(managed.clone(), wake)?,
            managed,
            trash: home.join(".local/share/Trash/files"),
            saved_settings: settings.clone(),
            settings,
            settings_path: home.join(".config/ts4-mod-manager/settings.json"),
            next: 1,
            recovering: true,
            operations: vec![],
            dialog: None,
            candidates: None,
            trash_entries: None,
            picker: None,
            picker_directory: if home.join("Downloads").is_dir() { home.join("Downloads") } else { home.clone() },
            #[cfg(test)]
            picker_sender: None,
            include_key: false,
            notice: None,
            error: None,
            refresh: None,
            roots_changed: None,
        })
    }
    pub fn source_api_key(&self) -> Option<&str> { self.saved_settings.curseforge_api_key.as_deref() }
    pub fn automatic_app_updates(&self) -> bool { self.saved_settings.check_app_updates }
    pub fn check_updates(&mut self, catalog: &Catalog, mod_ids: Vec<String>) {
        self.updates.sync(catalog, self.saved_settings.curseforge_api_key.as_deref());
        if self.updates.pending || catalog.loading || mod_ids.is_empty() { return; }
        if self.source_api_key().is_none() {
            self.settings_page = true;
            self.notice = Some("Save your CurseForge API key, then check for updates.".into());
            return;
        }
        if let Some(mut identity) = self.identity(catalog) {
            identity.entry = None;
            let token = self.updates.begin();
            self.release_choice = None;
            let operation = identity.operation;
            self.submit(identity, Action::CheckUpdates {
                mod_ids,
                api_key: self.saved_settings.curseforge_api_key.clone(),
                epoch: self.updates.epoch.clone(),
                token,
            });
            if !self.operations.last().is_some_and(|job| job.job.identity.operation == operation) { self.updates.pending = false; }
        }
    }
    pub fn update_summary(&self, ui: &mut egui::Ui, offer_navigation: bool) -> bool {
        let mut review = false;
        if self.updates.pending {
            ui.horizontal(|ui| { ui.spinner(); ui.label("Checking CurseForge releases..."); });
        } else if !self.updates.reports.is_empty() {
            let available = self.updates.reports.keys().filter(|id| self.updates.available(id)).count();
            let errors = self.updates.reports.values().filter(|r| r.is_err()).count();
            let unknown = self.updates.reports.values().filter(|r| matches!(r, Ok(report) if matches!(report.status,
                ts4_mod_manager_core::mod_updates::UpdateStatus::Untracked
                | ts4_mod_manager_core::mod_updates::UpdateStatus::LocalContentChanged
                | ts4_mod_manager_core::mod_updates::UpdateStatus::InstalledReleaseUnavailable
                | ts4_mod_manager_core::mod_updates::UpdateStatus::Incomplete))).count();
            egui::Frame::new().fill(ui.visuals().selection.bg_fill).corner_radius(10).inner_margin(12).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(egui::RichText::new(format!("{available} updates found")).color(ui.visuals().selection.stroke.color));
                    if unknown > 0 { ui.label(format!("{unknown} need a version")); }
                    if errors > 0 { ui.label(format!("{errors} failed checks")); }
                    review = offer_navigation && ui.button("Review").clicked();
                });
            });
        }
        review
    }
    pub fn update_details(&mut self, ui: &mut egui::Ui, catalog: &Catalog) {
        use ts4_mod_manager_core::mod_updates::UpdateStatus;
        self.record_release_rect = None;
        self.open_update_page_rect = None;
        let Some(entry) = catalog.selected_mod() else { return; };
        let Some(mod_id) = entry.id.as_ref().filter(|_| entry.source_attachment.as_ref()
            .is_some_and(|a| a.provider_id == "curseforge" && a.project_id.is_some())) else {
            ui.label("Use Find source and confirm a CurseForge project to track its releases.");
            return;
        };
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Updates").size(16.0).strong());
            if ui.add_enabled(!catalog.loading && !self.updates.pending, egui::Button::new("Check this mod")).clicked() {
                self.check_updates(catalog, vec![mod_id.clone()]);
            }
        });
        let Some(result) = self.updates.reports.get(mod_id).cloned() else {
            ui.label(if self.updates.pending { "Checking releases..." } else { "Not checked yet." });
            return;
        };
        let report = match result {
            Ok(report) => report,
            Err(error) => { ui.label(egui::RichText::new("Check failed").strong()); ui.label(error); return; }
        };
        match &report.status {
            UpdateStatus::Available(release) => {
                ui.label(egui::RichText::new("Newer public project release available").strong()
                    .color(ui.visuals().selection.stroke.color));
                ui.label(format!("{} · {}", release.file_name, release.published_at.format("%Y-%m-%d")));
                ui.label("Project releases may target other components or game versions.");
            }
            UpdateStatus::Untracked => { ui.label("Installed version unknown. Choose the release you installed below."); }
            UpdateStatus::LocalContentChanged => { ui.label("Local files changed. Confirm the installed release again."); }
            UpdateStatus::InstalledReleaseUnavailable => { ui.label("The recorded release is unavailable. Its version cannot be compared."); }
            UpdateStatus::Incomplete => { ui.label("The release list is incomplete. Update status is unknown."); }
            UpdateStatus::NoNewerRelease => { ui.label("No newer public release found."); }
        }
        if !report.complete { ui.label("Only part of the release history could be checked."); }
        if let Some(installed) = &report.installed {
            ui.label(format!("Recorded installed release: {}", installed.file_name));
        }
        let open = ui.button("Open mod page");
        self.open_update_page_rect = Some(open.rect);
        if open.clicked() { ui.ctx().open_url(egui::OpenUrl::new_tab(&report.source_url)); }

        if !report.releases.is_empty() {
            egui::CollapsingHeader::new("Record installed release").id_salt(("installed-release", mod_id))
                .default_open(matches!(report.status, UpdateStatus::Untracked | UpdateStatus::LocalContentChanged | UpdateStatus::InstalledReleaseUnavailable))
                .show(ui, |ui| {
                    let mut selected = self.release_choice.as_ref().filter(|(id, _)| id == mod_id).map(|(_, file)| *file);
                    let name = selected.and_then(|id| report.releases.iter().find(|r| r.file_id == id))
                        .map(|r| r.file_name.as_str()).unwrap_or("Choose the release you installed");
                    egui::ComboBox::from_id_salt(("release-choice", mod_id)).width(ui.available_width().min(480.0))
                        .selected_text(name).show_ui(ui, |ui| {
                            for release in &report.releases {
                                ui.selectable_value(&mut selected, Some(release.file_id),
                                    format!("{} · {}", release.file_name, release.published_at.format("%Y-%m-%d")));
                            }
                        });
                    self.release_choice = selected.map(|id| (mod_id.clone(), id));
                    let release = selected.and_then(|id| report.releases.iter().find(|r| r.file_id == id));
                    let button = ui.add_enabled(release.is_some() && !catalog.loading && !self.busy(),
                        egui::Button::new("Review installed release"));
                    self.record_release_rect = Some(button.rect);
                    if button.clicked() && let Some(release) = release {
                        self.review_installed_release(catalog, &report, release);
                    }
                });
        }
        if matches!(report.status, UpdateStatus::Available(_)) {
            ui.label("Download from the author's page, then record the release after replacing your local files.");
        }
    }
    pub fn review_installed_release(&mut self, catalog: &Catalog, report: &ts4_mod_manager_core::mod_updates::UpdateReport, release: &ts4_mod_manager_core::mod_updates::Release) {
        let Some(entry) = catalog.selected_mod() else { return; };
        let mut lines = vec![entry.name.clone(), report.source_url.clone(), release.file_name.clone(),
            "Confirm that this release describes the files you have installed.".into(),
            "Notify me of newer stable public releases from this project. These may include other components or game versions.".into(),
            "This records a version; it does not replace your local files.".into()];
        if entry.source_attachment.as_ref().and_then(|a| a.confidence).is_some_and(|c| c < 70) {
            lines.push("Low confidence source. Verify this project before recording a release.".into());
        }
        self.confirm(catalog, Action::RecordRelease { report: Box::new(report.clone()), release: release.clone(), token: self.updates.token() }, "Confirm installed release", lines);
    }
    pub fn prepare_browser_import(&mut self, ctx: &egui::Context, catalog: &Catalog, request: crate::project_details::ReviewRequest) {
        let (project_id, title) = match &request {
            crate::project_details::ReviewRequest::ProjectId(id) => (*id, format!("CurseForge project {id}")),
            crate::project_details::ReviewRequest::Loaded { project, .. } => (project.project_id, project.title.clone()),
        };
        if self.browser_import.running() || self.browser_import.preparing() || !self.download_queue.is_empty() {
            if let Some(label) = self.download_labels().get(&project_id) {
                self.notice = Some(if label == "Downloading…" { format!("{title} is already downloading.") } else { format!("{title} is already in the download queue ({label}).") });
                return;
            }
            let Some(root) = catalog.root.clone() else {
                self.notice = Some("Choose a Sims 4 game folder before downloading a mod.".into()); return;
            };
            self.download_queue.push_back(QueuedDownload { request, project_id, title: title.clone(), root });
            self.notice = Some(match self.download_queue.len() {
                1 => format!("{title} is next in the download queue."),
                position => format!("{title} is #{position} in the download queue."),
            });
            return;
        }
        self.start_browser_import(ctx, catalog, request, project_id);
    }
    /// Project IDs in download order: the active download first, then waiting mods.
    pub fn download_positions(&self) -> Vec<u64> {
        let active = (self.browser_import.running() || self.browser_import.preparing())
            .then(|| self.browser_import.project.as_ref().map(|project| project.project_id).or(self.active_download))
            .flatten();
        active.into_iter().chain(self.download_queue.iter().map(|queued| queued.project_id)).collect()
    }
    /// Short per-project labels for download buttons, such as "Next in queue".
    pub fn download_labels(&self) -> std::collections::HashMap<u64, String> {
        let active = self.browser_import.running() || self.browser_import.preparing();
        self.download_positions().into_iter().enumerate().map(|(index, id)| {
            let queued = if active { index } else { index + 1 };
            (id, match queued { 0 => "Downloading…".into(), 1 => "Next in queue".into(), n => format!("#{n} in queue") })
        }).collect()
    }
    pub fn download_queue(&self) -> Vec<(u64, String)> {
        self.download_queue.iter().map(|queued| (queued.project_id, queued.title.clone())).collect()
    }
    pub fn remove_queued_download(&mut self, project_id: u64) {
        if let Some(index) = self.download_queue.iter().position(|queued| queued.project_id == project_id) {
            let removed = self.download_queue.remove(index).unwrap();
            self.notice = Some(format!("{} removed from the download queue.", removed.title));
        }
    }
    pub fn skip_failed_download(&mut self) {
        self.browser_import.dismiss();
    }
    fn advance_download_queue(&mut self, ctx: &egui::Context, catalog: &Catalog) {
        let before = self.download_queue.len();
        self.download_queue.retain(|queued| catalog.root.as_ref() == Some(&queued.root));
        if self.download_queue.len() < before {
            self.notice = Some("Download queue cleared because the game folder changed.".into());
        }
        if self.download_queue.is_empty()
            || self.browser_import.running()
            || self.browser_import.preparing()
            || self.browser_import.phase == crate::browser_import::Phase::Failed
            || self.browser_pending_start
            || self.busy()
            || catalog.loading
            || self.picker.is_some()
        {
            return;
        }
        let next = self.download_queue.pop_front().unwrap();
        self.start_browser_import(ctx, catalog, next.request, next.project_id);
    }
    fn start_browser_import(&mut self, ctx: &egui::Context, catalog: &Catalog, request: crate::project_details::ReviewRequest, project_id: u64) {
        let Some(root) = catalog.root.clone().filter(|_| !catalog.loading && !self.busy()) else {
            self.notice = Some("Choose a Sims 4 game folder before downloading a mod.".into()); return;
        };
        let Some(key) = self.source_api_key().map(str::to_owned) else {
            self.settings_page = true; self.notice = Some("Save your CurseForge API key before downloading.".into()); return;
        };
        if self.picker.is_some() { self.notice = Some("Finish choosing a folder before downloading.".into()); return; }
        self.browser_pending_start = true;
        self.active_download = Some(project_id);
        match request {
            crate::project_details::ReviewRequest::ProjectId(project_id) => self.browser_import.prepare(ctx, root, &key, project_id),
            crate::project_details::ReviewRequest::Loaded {project,file_id} => {
                if !project.releases.iter().any(|r|r.release.file_id==file_id) {self.browser_pending_start=false;self.notice=Some("Choose a verified release before downloading.".into());return;}
                self.browser_import.prepare_loaded(root,&key,project);
                self.browser_import.chosen=Some(file_id);
            },
        }
    }
    fn browser_import_status(&mut self, ui: &mut egui::Ui) {
        use crate::browser_import::Phase;
        if matches!(self.browser_import.phase, Phase::Waiting | Phase::Queued | Phase::Installing | Phase::Failed | Phase::Cancelled)
            && let Some(project) = &self.browser_import.project {
            ui.label(egui::RichText::new(&project.title).strong().color(ui.visuals().text_color()));
        }
        match self.browser_import.phase {
            Phase::Loading | Phase::Ready => {
                ui.horizontal_wrapped(|ui| {
                    ui.add(egui::Spinner::new().size(16.0));
                    ui.label("Preparing browser download…");
                    let cancel = ui.button("Cancel download");
                    self.browser_cancel_rect = Some(cancel.rect);
                    if cancel.clicked() { self.browser_import.cancel(); }
                });
            },
            Phase::Waiting | Phase::Queued | Phase::Installing => {
                ui.horizontal_wrapped(|ui| {
                    ui.add(egui::Spinner::new().size(16.0));
                    ui.label(match self.browser_import.phase { Phase::Waiting => "Waiting for browser download…", Phase::Queued => "Download verified. Installation queued…", _ => "Installing in the approved game folder…" });
                    let cancel = ui.add_enabled(self.browser_import.phase != Phase::Installing, egui::Button::new("Cancel download import"));
                    self.browser_cancel_rect = Some(cancel.rect);
                    if cancel.clicked() { self.browser_import.cancel(); self.notice = Some("Browser download import cancelled.".into()); }
                });
                ui.label(egui::RichText::new(format!("Download folder: {}", self.browser_import.download_dir.display())).small().color(ui.visuals().weak_text_color()));
            }
            Phase::Failed | Phase::Cancelled => {
                ui.horizontal_wrapped(|ui| {
                    ui.label(if self.browser_import.phase == Phase::Cancelled { "Browser download import cancelled." } else { "Browser download import failed." });
                    if self.browser_import.can_retry() && ui.button("Retry browser import").clicked() {
                        match self.browser_import.retry(ui.ctx()) { Ok(url) => ui.ctx().open_url(egui::OpenUrl::new_tab(url)), Err(error) => self.notice = Some(error) }
                    }
                });
                if let Some(error) = &self.browser_import.error { ui.label(error); }
                if !self.browser_import.can_retry() && self.download_queue.is_empty() { ui.label("Open the project and choose Download in browser to start a new request."); }
            }
            _ => {}
        }
        self.download_queue_status(ui);
    }
    fn download_queue_status(&mut self, ui: &mut egui::Ui) {
        self.queue_remove_rects.clear();
        self.queue_skip_rect = None;
        if self.download_queue.is_empty() { return; }
        if self.browser_import.phase == crate::browser_import::Phase::Failed {
            ui.horizontal_wrapped(|ui| {
                ui.label("Download queue paused.");
                let skip = ui.button("Skip and continue");
                self.queue_skip_rect = Some(skip.rect);
                if skip.clicked() { self.skip_failed_download(); }
            });
        }
        ui.label(egui::RichText::new("Up next").strong().color(ui.visuals().text_color()));
        let mut remove = None;
        for (index, queued) in self.download_queue.iter().enumerate() {
            ui.horizontal_wrapped(|ui| {
                ui.label(egui::RichText::new(&queued.title).color(ui.visuals().text_color()));
                let position = if index == 0 { "Next in queue".to_owned() } else { format!("#{} in queue", index + 1) };
                ui.label(egui::RichText::new(position).color(ui.visuals().weak_text_color()));
                let button = ui.button("Remove").on_hover_text("Remove this mod from the download queue");
                self.queue_remove_rects.push(button.rect);
                if button.clicked() { remove = Some(queued.project_id); }
            });
        }
        if let Some(project_id) = remove { self.remove_queued_download(project_id); }
    }
    fn start_picker(
        &mut self,
        purpose: PickPurpose,
        ctx: &egui::Context,
        initial: Option<PathBuf>,
    ) {
        if self.picker.is_some() {
            return;
        }
        let kind = match &purpose {
            PickPurpose::Import(_, true) => crate::file_picker::Kind::ImportZip,
            PickPurpose::Import(_, false) => crate::file_picker::Kind::ImportFolder,
            PickPurpose::GameFolder => crate::file_picker::Kind::GameFolder,
            PickPurpose::DownloadsFolder => crate::file_picker::Kind::DownloadsFolder,
            PickPurpose::SettingsImport => crate::file_picker::Kind::SettingsImport,
            PickPurpose::SettingsExport(_) => crate::file_picker::Kind::SettingsExport,
        };
        let initial = initial.unwrap_or_else(|| self.picker_directory.clone());
        #[cfg(not(test))]
        let result = crate::file_picker::open(kind, initial, ctx.clone());
        #[cfg(test)]
        let result: std::io::Result<_> = {
            let _ = (kind, initial, ctx);
            let (sender, receiver) = std::sync::mpsc::channel();
            self.picker_sender = Some(sender);
            Ok(receiver)
        };
        match result {
            Ok(receiver) => self.picker = Some(PendingPicker { purpose, receiver }),
            Err(error) => {
                self.error = Some(format!("Cannot open the desktop file picker: {error}"))
            }
        }
    }
    pub fn choose_game_folder(&mut self, ctx: &egui::Context, initial: Option<PathBuf>) {
        self.start_picker(PickPurpose::GameFolder, ctx, initial);
    }
    pub fn picker_open(&self) -> bool {
        self.picker.is_some()
    }
    fn picked(&mut self, purpose: PickPurpose, path: Option<PathBuf>, catalog: &Catalog) {
        let Some(path) = path else {
            return;
        };
        if let Some(parent) = path.parent() {
            self.picker_directory = parent.to_path_buf();
        }
        match purpose {
            PickPurpose::GameFolder => self.roots_changed = Some(path),
            PickPurpose::DownloadsFolder => {
                match self.browser_import.set_download_directory(path) {
                    Ok(()) => self.notice = Some("Download folder changed for this session.".into()),
                    Err(error) => self.error = Some(error),
                }
            },
            PickPurpose::Import(identity, zip) => {
                if !identity.current(
                    catalog.root.as_ref(),
                    catalog.selected.as_ref(),
                    catalog.generation(),
                ) {
                    self.notice =
                        Some("Game folder or selection changed. Choose the import again.".into());
                    return;
                }
                let name = (if zip {
                    path.file_stem()
                } else {
                    path.file_name()
                })
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_default();
                self.dialog = Some(Dialog::Import {
                    identity,
                    zip,
                    path,
                    name,
                });
            }
            PickPurpose::SettingsImport => match Settings::read_transfer(&path) {
                Ok(mut settings) => {
                    let included = settings.curseforge_api_key.is_some();
                    settings.preserve_omitted_key(&self.saved_settings);
                    if settings.custom_themes.is_empty() {
                        settings.custom_themes = self.saved_settings.custom_themes.clone();
                    }
                    self.dialog = Some(Dialog::SettingsImport(settings, included));
                }
                Err(error) => self.error = Some(error),
            },
            PickPurpose::SettingsExport(settings) => match settings.save_new(&path) {
                Ok(()) => self.notice = Some("Settings exported".into()),
                Err(error) => self.error = Some(error),
            },
        }
    }
    fn identity(&mut self, catalog: &Catalog) -> Option<Identity> {
        let root = catalog.root.clone()?;
        let identity = Identity {
            operation: self.next,
            root,
            entry: catalog.selected.clone(),
            generation: catalog.generation(),
        };
        self.next += 1;
        Some(identity)
    }
    fn submit(&mut self, identity: Identity, action: Action) {
        if self.app_updates.installing() {
            self.error = Some("Wait for the app update to finish restarting before changing mods.".into());
            return;
        }
        if matches!(action, Action::Lookup(_)) && self.operations.iter().any(|o|
            matches!(o.job.action, Action::Lookup(_)) && (o.status == "Queued" || o.status == "Running")) {
            self.notice = Some("A source lookup is already running. Local actions remain available.".into());
            return;
        }
        if !action.read_only() && !matches!(action, Action::Toggle { .. }) {
            self.updates.invalidate();
            self.release_choice = None;
        }
        let job = Job {
            identity,
            managed: self.managed.clone(),
            trash: self.trash.clone(),
            action,
        };
        match self.worker.submit(job.clone()) {
            Ok(()) => self.operations.push(Operation {
                job,
                status: "Queued".into(),
                error: false,
            }),
            Err(e) => self.error = Some(e),
        }
    }
    fn approve(
        &mut self,
        catalog: &Catalog,
        identity: &Identity,
        action: Action,
    ) -> Result<u64, String> {
        if matches!(&action, Action::RecordRelease { token, .. } if *token != self.updates.token()) {
            return Err("Update results changed. Check again before recording a release.".into());
        }
        if !identity.current(
            catalog.root.as_ref(),
            catalog.selected.as_ref(),
            catalog.generation(),
        ) {
            return Err("Selection or catalog changed. Review again before approving.".into());
        }
        let mut identity = identity.clone();
        identity.operation = self.next;
        self.next += 1;
        let operation = identity.operation;
        self.submit(identity, action);
        if self
            .operations
            .last()
            .is_some_and(|job| job.job.identity.operation == operation)
        {
            Ok(operation)
        } else {
            Err(self
                .error
                .clone()
                .unwrap_or_else(|| "Cannot queue approved operation".into()))
        }
    }
    pub(crate) fn fixture_action(
        &mut self,
        root: PathBuf,
        entry: Option<crate::catalog::EntryId>,
        action: Action,
    ) -> Result<Output, String> {
        let mut catalog = Catalog::default();
        catalog.begin_scan(root);
        catalog.selected = entry;
        let identity = self.identity(&catalog).ok_or("Missing fixture root")?;
        let operation = if matches!(
            &action,
            Action::Migrate { .. }
                | Action::ManualSource(_)
                | Action::Attach(_)
                | Action::RemoveSource
                | Action::Trash
                | Action::Restore(_)
        ) {
            self.approve(&catalog, &identity, action)?
        } else {
            let operation = identity.operation;
            self.submit(identity, action);
            operation
        };
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
        loop {
            for event in self.worker.drain() {
                match event {
                    Event::Recovered(_) => self.recovering = false,
                    Event::Running(identity) => {
                        if let Some(job) = self
                            .operations
                            .iter_mut()
                            .find(|job| job.job.identity.operation == identity.operation)
                        {
                            job.status = "Running".into();
                        }
                    }
                    Event::Finished(identity, result) if identity.operation == operation => {
                        if let Some(job) = self
                            .operations
                            .iter_mut()
                            .find(|job| job.job.identity.operation == operation)
                        {
                            job.status = "Finished".into();
                            job.error = result.is_err();
                        }
                        return result;
                    }
                    _ => {}
                }
            }
            if std::time::Instant::now() >= deadline {
                return Err("Fixture workflow timed out".into());
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
    pub fn busy(&self) -> bool {
        self.app_updates.installing()
            || self.recovering
            || self
                .operations
                .iter()
                .any(|o| !o.job.action.read_only() && (o.status == "Queued" || o.status == "Running"))
    }
    pub fn poll(&mut self, catalog: &mut Catalog) {
        self.browser_import.sync(catalog.root.as_ref(), self.saved_settings.curseforge_api_key.as_deref());
        if let Some(request) = self.browser_import.poll() {
            let token = request.token;
            let identity = Identity { operation: self.next, root: request.root.clone(), entry: None, generation: catalog.generation() };
            self.next += 1;
            let operation = identity.operation;
            self.submit(identity, Action::ImportBrowser(Box::new(request)));
            if !self.operations.last().is_some_and(|job| job.job.identity.operation == operation) {
                self.browser_import.finish(token, Err("Could not queue the verified installation. Retry the download session.".into()));
            }
        }
        self.updates.sync(catalog, self.saved_settings.curseforge_api_key.as_deref());
        if let Some(picker) = &self.picker {
            match picker.receiver.try_recv() {
                Ok(path) => {
                    let picker = self.picker.take().unwrap();
                    self.picked(picker.purpose, path, catalog);
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.picker = None;
                    self.error = Some("The desktop file picker closed unexpectedly. Try again.".into());
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
            }
        }
        for event in self.worker.drain().collect::<Vec<_>>() {
            match event {
                Event::Recovered(result) => {
                    self.recovering = false;
                    self.refresh = catalog.root.clone();
                    match result {
                        Ok(issues) if !issues.is_empty() => {
                            self.error = Some(format!(
                                "Startup recovery requires attention: {}",
                                issues.join("\n")
                            ))
                        }
                        Err(error) => {
                            self.error = Some(format!("Startup recovery failed: {error}"))
                        }
                        _ => {}
                    }
                }
                Event::Running(identity) => {
                    if let Some(op) = self
                        .operations
                        .iter_mut()
                        .find(|o| o.job.identity.operation == identity.operation)
                    {
                        op.status = "Running".into()
                    }
                }
                Event::Finished(identity, result) => {
                    let mut failure = None;
                    match result {
                        Err(e) => {
                            if let Some(operation) = self.operations.iter().find(|op| op.job.identity.operation == identity.operation)
                                && let Action::ImportBrowser(request) = &operation.job.action {
                                self.browser_import.finish(request.token, Err(e.clone()));
                                self.refresh = catalog.root.clone();
                            }
                            failure = Some(e);
                        },
                        Ok(Output::BrowserImported { token, metadata }) => {
                            self.browser_import.finish(token, Ok(()));
                            self.updates.invalidate();
                            self.refresh = catalog.root.clone();
                            self.notice = Some(format!("{} imported and enabled in {}", metadata.effective_display_name(), identity.root.display()));
                        },
                        Ok(Output::Changed(issues)) => {
                            let toggled = self.operations.iter().find(|operation| operation.job.identity.operation == identity.operation)
                                .and_then(|operation| match operation.job.action { Action::Toggle {enabled} => Some(enabled), _ => None });
                            if catalog.root.as_ref() == Some(&identity.root)
                                && let Some(enabled) = toggled {
                                self.error = None;
                                let mut notice = if enabled {"Mod enabled"} else {"Mod disabled"}.to_string();
                                if !issues.is_empty() { notice.push_str(&format!(". {}", issues.join("\n"))); }
                                self.notice = Some(notice);
                            }
                            if toggled.is_some() {
                                if catalog.root.as_ref() == Some(&identity.root) {
                                    if catalog.loading {
                                        // Supersede an explicit scan that may predate this mutation.
                                        self.refresh = catalog.root.clone();
                                    } else if let (Some(EntryId::Managed(id)), Some(enabled)) = (&identity.entry, toggled) {
                                        catalog.set_enabled(id, enabled);
                                    }
                                }
                            } else {
                                self.updates.invalidate();
                                self.refresh = catalog.root.clone();
                            }
                            if !issues.is_empty() && toggled.is_none() {
                                failure = Some(issues.join("\n"))
                            }
                        }
                        Ok(Output::Imported { name, enable_error }) => {
                            self.updates.invalidate();
                            self.refresh = catalog.root.clone();
                            if let Some(error) = enable_error {
                                self.error = Some(format!("{name} was added but could not be enabled. Open Activity for details, then turn it on.\n{error}"));
                            } else {
                                self.notice = Some(format!("{name} added and enabled"));
                            }
                        }
                        Ok(Output::Candidates(candidates))
                            if identity.current(
                                catalog.root.as_ref(),
                                catalog.selected.as_ref(),
                                catalog.generation(),
                            ) =>
                        {
                            self.candidates = Some((identity.clone(), candidates));
                        }
                        Ok(Output::Updates { token, reports })
                            if catalog.root.as_ref() == Some(&identity.root)
                                && catalog.generation() == identity.generation =>
                        {
                            self.updates.accept(token, reports);
                        }
                        Ok(Output::Trash(entries))
                            if identity.current(
                                catalog.root.as_ref(),
                                catalog.selected.as_ref(),
                                catalog.generation(),
                            ) =>
                        {
                            self.candidates = None;
                            self.dialog = None;
                            self.trash_entries = Some((identity.clone(), entries));
                        }
                        _ => {}
                    }
                    if let Some(op) = self
                        .operations
                        .iter_mut()
                        .find(|o| o.job.identity.operation == identity.operation)
                    {
                        if let Some(error) = &failure
                            && identity.root == catalog.root.clone().unwrap_or_default() {
                            self.error = Some(format!("{} failed: {error}", op.job.action.label()));
                        }
                        op.error = failure.is_some();
                        op.status = failure.unwrap_or_else(|| "Finished".into());
                    }
                }
            }
        }
    }
    fn confirm(&mut self, catalog: &Catalog, action: Action, title: &str, lines: Vec<String>) {
        if let Some(identity) = self.identity(catalog) {
            self.dialog = Some(Dialog::Confirm {
                identity,
                action,
                title: title.into(),
                lines,
            })
        }
    }
    pub fn show(&mut self, ui: &mut egui::Ui, catalog: &Catalog) {
        self.toolbar(ui, catalog);
        self.selected_actions(ui, catalog);
        self.activity_header(ui, catalog.loading, false);
        self.notifications(ui.ctx());
        self.show_dialogs(ui.ctx(), catalog);
    }
    pub fn open_trash(&mut self, catalog: &Catalog) {
        if let Some(identity) = self.identity(catalog) { self.submit(identity, Action::ListTrash); }
    }
    pub fn toggle_selected(&mut self, catalog: &Catalog) {
        let Some(entry) = catalog.selected_mod().filter(|entry| entry.id.is_some()) else { return; };
        if catalog.loading || self.busy() { return; }
        let enabled = !entry.enabled;
        if let Some(identity) = self.identity(catalog) { self.submit(identity, Action::Toggle { enabled }); }
    }
    pub fn add_mods_button(&mut self, ui: &mut egui::Ui, catalog: &Catalog) {
        let colors = crate::ui::chrome_colors(ui);
        let ready = catalog.root.is_some() && !catalog.loading && !self.picker_open();
        ui.add_enabled_ui(ready, |ui| {
            ui.scope(|ui| {
                let v = ui.visuals_mut();
                for widget in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
                    widget.weak_bg_fill = colors.primary;
                    widget.bg_fill = widget.weak_bg_fill;
                    widget.fg_stroke.color = colors.primary_text;
                }
                ui.menu_image_text_button(crate::ui::Icon::Plus.image(colors.primary_text, 18.0), egui::RichText::new("Add mods").color(colors.primary_text).strong(), |ui| {
                    // Restore the content palette inside the menu.
                    let style = ui.ctx().style_of(ui.ctx().theme());
                    *ui.style_mut() = (*style).clone();
                    for (label, zip) in [("Import folder", false), ("Import ZIP", true)] {
                        let response = ui.button(label);
                        if !zip { self.import_rect = Some(response.rect); }
                        if response.clicked() && let Some(identity) = self.identity(catalog) {
                            ui.close();
                            self.start_picker(PickPurpose::Import(identity, zip), ui.ctx(), None);
                        }
                    }
                });
            });
        });
    }
    pub fn library_toolbar(&mut self, ui: &mut egui::Ui, catalog: &Catalog) {
        self.updates.sync(catalog, self.saved_settings.curseforge_api_key.as_deref());
        if ui.ctx().content_rect().height() >= 600.0 && catalog.mods.iter().any(|entry| entry.source_attachment.as_ref().is_some_and(|source| source.provider_id == "curseforge" && source.project_id.is_some())) {
            self.check_updates_button(ui, catalog);
        }
    }
    fn check_updates_button(&mut self, ui: &mut egui::Ui, catalog: &Catalog) {
        let ids: Vec<_> = catalog.mods.iter().filter(|entry| entry.source_attachment.as_ref()
            .is_some_and(|source| source.provider_id == "curseforge" && source.project_id.is_some()))
            .filter_map(|entry| entry.id.clone()).collect();
        let response = ui.add_enabled(catalog.root.is_some() && !catalog.loading && !self.updates.pending && !ids.is_empty(), egui::Button::new("Check updates"))
            .on_hover_text("Check mods with a confirmed CurseForge source");
        self.check_updates_rect = Some(response.rect);
        if response.clicked() { self.check_updates(catalog, ids); }
    }
    pub fn updates_content(&mut self, ui: &mut egui::Ui, catalog: &Catalog) -> Option<usize> {
        use ts4_mod_manager_core::mod_updates::UpdateStatus;
        self.updates.sync(catalog, self.saved_settings.curseforge_api_key.as_deref());
        ui.label(egui::RichText::new("Updates").size(32.0).strong());
        ui.add_space(16.0);
        self.check_updates_button(ui, catalog);
        self.update_summary(ui, false);
        ui.add_space(20.0);
        let mut selected = None;
        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut linked = 0;
            for (index, entry) in catalog.mods.iter().enumerate().filter(|(_, entry)| entry.source_attachment.as_ref()
                .is_some_and(|source| source.provider_id == "curseforge" && source.project_id.is_some())) {
                linked += 1;
                let status = match entry.id.as_ref().and_then(|id| self.updates.reports.get(id)) {
                    Some(Ok(report)) => match report.status {
                        UpdateStatus::Available(_) => "Update available",
                        UpdateStatus::NoNewerRelease => "Up to date",
                        UpdateStatus::Untracked => "Choose installed release",
                        UpdateStatus::LocalContentChanged => "Local files changed",
                        UpdateStatus::InstalledReleaseUnavailable => "Installed release unavailable",
                        UpdateStatus::Incomplete => "Check incomplete",
                    },
                    Some(Err(_)) => "Check failed",
                    None => "Not checked",
                };
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&entry.name).strong());
                    ui.label(status);
                    if ui.button("Review").clicked() { selected = Some(index); }
                });
                ui.separator();
            }
            if linked == 0 {
                ui.heading("No linked mods yet");
                ui.label("Open a mod and use Find source to link its CurseForge page.");
            }
        });
        selected
    }
    pub fn toolbar(&mut self, ui: &mut egui::Ui, catalog: &Catalog) {
        self.updates.sync(catalog, self.saved_settings.curseforge_api_key.as_deref());
        ui.horizontal_wrapped(|ui| {
            let ready = catalog.root.is_some() && !catalog.loading && self.picker.is_none();
            let import = ui.add_enabled(
                ready,
                egui::Button::new("Import folder")
                    .fill(ui.visuals().selection.bg_fill)
                    .stroke(ui.visuals().selection.stroke),
            );
            self.import_rect = Some(import.rect);
            if import.clicked()
                && let Some(identity) = self.identity(catalog)
            {
                self.start_picker(PickPurpose::Import(identity, false), ui.ctx(), None);
            }
            if ui
                .add_enabled(ready, egui::Button::new("Import ZIP"))
                .clicked()
                && let Some(identity) = self.identity(catalog)
            {
                self.start_picker(PickPurpose::Import(identity, true), ui.ctx(), None);
            }
            if ui
                .add_enabled(ready, egui::Button::new("Restore from trash"))
                .clicked()
                && let Some(id) = self.identity(catalog)
            {
                self.submit(id, Action::ListTrash)
            }
            let mod_ids: Vec<_> = catalog.mods.iter().filter(|entry| entry.source_attachment.as_ref()
                .is_some_and(|source| source.provider_id == "curseforge" && source.project_id.is_some()))
                .filter_map(|entry| entry.id.clone()).collect();
            let check = ui.add_enabled(ready && !self.updates.pending && !mod_ids.is_empty(), egui::Button::new("Check updates"))
                .on_hover_text("Check managed mods with a confirmed CurseForge source. Use Find source to link a mod first.");
            self.check_updates_rect = Some(check.rect);
            if check.clicked() { self.check_updates(catalog, mod_ids); }

        });
    }
    pub fn selected_actions(&mut self, ui: &mut egui::Ui, catalog: &Catalog) {
        if catalog.selected_mod().is_some() {
            ui.horizontal_wrapped(|ui| {
                if let Some(entry) = catalog.selected_mod().filter(|entry| {
                    catalog.root.is_some() && !catalog.loading && entry.id.is_some()
                }) && ui
                    .add(if entry.enabled {
                        egui::Button::new("Disable")
                    } else {
                        egui::Button::new("Enable").fill(ui.visuals().selection.bg_fill)
                    })
                    .clicked()
                    && let Some(id) = self.identity(catalog)
                {
                    self.submit(
                        id,
                        Action::Toggle {
                            enabled: !entry.enabled,
                        },
                    );
                }
                let button = ui.button("More actions");
                let menu = egui::Popup::menu(&button);
                if ui.layer_id().order == egui::Order::Foreground {
                    ui.ctx().set_sublayer(
                        ui.layer_id(),
                        egui::LayerId::new(egui::Order::Foreground, menu.get_id()),
                    );
                }
                menu.show(|ui| self.mod_menu(ui, catalog));
            });
        }
    }
    pub fn mod_menu(&mut self, ui: &mut egui::Ui, catalog: &Catalog) {
        let ready = catalog.root.is_some() && !catalog.loading;
        ui.add_enabled_ui(ready, |ui| {
            if let Some(entry) = catalog.selected_mod() {
                if let Some(managed_id) = &entry.id {
                            if ui.button(if entry.enabled { "Disable" } else { "Enable" }).clicked()
                                && let Some(id) = self.identity(catalog)
                            {
                                ui.close();
                            self.submit(id, Action::Toggle { enabled: !entry.enabled })
                            }
                            if ui.button("Rename").clicked()
                                && let Some(identity) = self.identity(catalog)
                            {
                                ui.close();
                                self.dialog = Some(Dialog::Rename(identity, entry.name.clone()))
                            }
                            ui.separator();
                            if ui.button("Find source").clicked()
                                && let Some(id) = self.identity(catalog)
                            {
                                ui.close();
                                self.submit(id, Action::Lookup(self.saved_settings.curseforge_api_key.clone()))
                            }
                            if ui.button("Attach URL").clicked()
                                && let Some(identity) = self.identity(catalog)
                            {
                                ui.close();
                                self.dialog = Some(Dialog::Manual(identity, String::new()))
                            }
                            if entry.source_url.is_some() && ui.button("Remove source").clicked() {
                                ui.close();
                                self.confirm(catalog, Action::RemoveSource, "Remove source", vec![entry.source_url.clone().unwrap()])
                            }
                            ui.separator();
                            if ui.button("Move to trash").clicked() {
                                ui.close();
                                self.confirm(
                                    catalog,
                                    Action::Trash,
                                    "Move managed mod to trash",
                                    vec![
                                        self.managed.join("mods").join(managed_id).display().to_string(),
                                        "Verified installed links will be removed. Files stay recoverable in trash.".into(),
                                    ],
                                )
                            }
                        } else if ui.button("Manage external mod").clicked() {
                            ui.close();
                            self.confirm(catalog, Action::Migrate { files: entry.files.clone() }, "Review external migration", std::iter::once("These external files will move into managed storage. Verified links will keep them installed in this game folder. Other files stay untouched.".into()).chain(entry.files.iter().map(|f| catalog.root.as_ref().unwrap().join("Mods").join(f).display().to_string())).collect())
                        }
            }
        });
    }
    pub fn set_color_mode(&mut self, ctx: &egui::Context, theme: Theme) {
        self.settings.theme = theme.clone();
        let mut saved = self.saved_settings.clone();
        saved.theme = theme;
        match saved.save(&self.settings_path) {
            Ok(()) => {
                crate::ui::apply_settings_theme(ctx, &saved);
                self.saved_settings = saved;
            },
            Err(error) => self.error = Some(error),
        }
    }
    pub fn review_open(&self) -> bool {
        self.dialog.is_some() || self.candidates.is_some() || self.trash_entries.is_some()
    }

    pub fn show_dialogs(&mut self, ctx: &egui::Context, catalog: &Catalog) {
        self.browser_import.sync(catalog.root.as_ref(), self.saved_settings.curseforge_api_key.as_deref());
        self.advance_download_queue(ctx, catalog);
        if self.browser_pending_start {
            match self.browser_import.phase {
                crate::browser_import::Phase::Ready => {
                    self.browser_pending_start = false;
                    if let Ok(url) = self.browser_import.start(ctx) {
                        ctx.open_url(egui::OpenUrl::new_tab(url));
                    }
                },
                crate::browser_import::Phase::Loading => {},
                _ => self.browser_pending_start = false,
            }
        }
        if self.candidates.as_ref().is_some_and(|(id, _)| {
            !id.current(
                catalog.root.as_ref(),
                catalog.selected.as_ref(),
                catalog.generation(),
            )
        }) {
            self.candidates = None;
        }
        self.confirm_rect = None;
        self.previews.begin_frame(ctx, catalog.generation());
        if let Some((identity, candidates)) = self.candidates.take() {
            let mut keep = true;
            let popup = crate::popup::show(ctx, "source-candidates", "Source candidates", |ui| {
                if candidates.is_empty() {
                    ui.label("No source matches found.");
                    ui.label("If you know the project URL, attach it manually and review it before saving.");
                    if ui.button("Attach a URL manually").clicked() {
                        self.dialog = Some(Dialog::Manual(identity.clone(), String::new()));
                        keep = false;
                    }
                }
                egui::ScrollArea::vertical().max_height(crate::popup::body_height(ctx)).show(ui, |ui| {
                    for candidate in &candidates {
                        ui.heading(&candidate.title);
                        ui.label(&candidate.source_url);
                        ui.label(format!("Confidence: {}%", candidate.confidence));
                        if candidate.confidence < 70 {
                            ui.colored_label(ui.visuals().error_fg_color, "Low confidence. Check this project carefully before attaching.");
                        }
                        for reason in &candidate.reasons {
                            ui.label(reason);
                        }
                        for evidence in &candidate.evidence {
                            ui.label(format!("{}: {}", evidence.kind, evidence.description));
                        }
                        if ui.button("Review attachment").clicked() {
                            let mut lines = vec![
                                candidate.title.clone(),
                                candidate.source_url.clone(),
                                format!("Confidence: {}%", candidate.confidence),
                            ];
                            if candidate.confidence < 70 {
                                lines.push("Low confidence: this candidate is not verified.".into());
                            }
                            lines.extend(candidate.reasons.clone());
                            lines.extend(candidate.evidence.iter().map(|e| e.description.clone()));
                            self.dialog = Some(Dialog::Confirm {
                                identity: identity.clone(),
                                action: Action::Attach(candidate.clone()),
                                title: "Attach source candidate".into(),
                                lines,
                            });
                            keep = false;
                        }
                        ui.separator();
                    }
                });
                if crate::popup::close(ui).clicked() {
                    keep = false;
                }
            });
            if keep && !popup.should_close() {
                self.candidates = Some((identity, candidates));
            }
            ctx.request_repaint();
            return;
        }
        if let Some((identity, entries)) = self.trash_entries.take().filter(|(id, _)| {
            id.current(
                catalog.root.as_ref(),
                catalog.selected.as_ref(),
                catalog.generation(),
            )
        }) {
            let mut keep = true;
            let popup = crate::popup::show(ctx, "restore-from-trash", "Restore from trash", |ui| {
                if entries.is_empty() {
                    ui.label("No recoverable entries in trash.");
                }
                egui::ScrollArea::vertical().max_height(crate::popup::body_height(ctx)).show(ui, |ui| {
                    for entry in &entries {
                        ui.label(&entry.path);
                        if let Some(path) = &entry.original_path {
                            ui.label(format!("Original location: {path}"));
                        }
                        if ui.button(format!("Review restore: {}", entry.name)).clicked() {
                            self.confirm(
                                catalog,
                                Action::Restore(entry.name.clone()),
                                "Restore mod",
                                vec![entry.path.clone(), "Restore checks destinations and refuses to overwrite files.".into()],
                            );
                            keep = false;
                        }
                    }
                });
                if crate::popup::close(ui).clicked() {
                    keep = false;
                }
            });
            if keep && !popup.should_close() {
                self.trash_entries = Some((identity, entries));
            }
            ctx.request_repaint();
            return;
        }
        let Some(mut dialog) = self.dialog.take() else {
            return;
        };
        let mut keep = true;
        let title = match &dialog {
            Dialog::Import { zip, .. } => {
                if *zip {
                    "Import ZIP"
                } else {
                    "Import folder"
                }
            }
            Dialog::Rename(..) => "Rename mod",
            Dialog::Manual(..) => "Attach manual source URL",
            Dialog::Confirm { title, .. } => title,
            Dialog::SettingsImport(..) => "Review imported settings",
        }
        .to_string();
        let popup = crate::popup::show(ctx, "management-review", &title, |ui| {
            let (label, enabled) = egui::ScrollArea::vertical()
                .max_height(crate::popup::body_height(ctx))
                .show(ui, |ui| {
                    ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                    let action = match &mut dialog {
                        Dialog::Import { identity, zip, path, name } => {
                            ui.label(if *zip { "ZIP path" } else { "Folder path" });
                            ui.label(egui::RichText::new(path.display().to_string()).color(ui.visuals().weak_text_color()));
                            ui.add_space(12.0);
                            ui.label("A copy goes into your managed library. The original stays where it is. It starts enabled in this game folder. You can turn it off from the library.");
                            ui.collapsing("Storage location", |ui| { ui.label(self.managed.join("mods").display().to_string()); });
                            ui.add_space(12.0);
                            crate::popup::text_field(ui, "Mod name", name);
                            let current = identity.current(catalog.root.as_ref(), catalog.selected.as_ref(), catalog.generation());
                            if !current {
                                ui.colored_label(ui.visuals().error_fg_color, "Game folder or selection changed. Cancel and choose the import again.");
                            }
                            ("Import", current && !path.as_os_str().is_empty() && !name.trim().is_empty())
                        }
                        Dialog::Rename(identity, name) => {
                            crate::popup::text_field(ui, "Display name", name);
                            ("Save name", identity.current(catalog.root.as_ref(), catalog.selected.as_ref(), catalog.generation()) && !name.trim().is_empty())
                        }
                        Dialog::Manual(identity, url) => {
                            ui.label("Manual attachments carry no verified match claim.");
                            crate::popup::text_field(ui, "Source URL", url);
                            ("Review URL attachment", identity.current(catalog.root.as_ref(), catalog.selected.as_ref(), catalog.generation()) && !url.trim().is_empty())
                        }
                        Dialog::Confirm { identity, action, lines, .. } => {
                            for line in lines { ui.label(line.as_str()); }
                            let current = identity.current(catalog.root.as_ref(), catalog.selected.as_ref(), catalog.generation());
                            if !current {
                                ui.colored_label(ui.visuals().error_fg_color, "Selection or catalog changed. Close and review again.");
                            }
                            (action.label(), current)
                        }
                        Dialog::SettingsImport(settings, key_included) => {
                            ui.label("Replaces your current draft. Save changes in Settings to apply it.");
                            ui.label(format!("Theme: {}", match settings.theme {
                                Theme::Light => "light", Theme::Dark => "dark", Theme::System => "system",
                            }));
                            for root in &settings.game_roots { ui.label(root); }
                            if let Some(selected) = &settings.selected_root { ui.label(format!("Selected folder: {selected}")); }
                            ui.label(if *key_included { "Includes an API key" } else { "API key omitted. Existing local key will be preserved." });
                            ("Preview settings", true)
                        }
                    };
                    if let Some(error) = &self.error {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                    action
                }).inner;
            let (approve, cancel) = crate::popup::actions(ui, label, enabled);
            if enabled {
                self.confirm_rect = Some(approve.rect);
            }
            if cancel.clicked() {
                keep = false;
            }
            if approve.clicked() {
                match &dialog {
                    Dialog::Import {
                        identity,
                        zip,
                        path,
                        name,
                    } => {
                        self.submit(
                            identity.clone(),
                            if *zip {
                                Action::ImportZip {
                                    path: path.clone(),
                                    name: name.trim().into(),
                                }
                            } else {
                                Action::ImportFolder {
                                    path: path.clone(),
                                    name: name.trim().into(),
                                }
                            },
                        );
                        keep = false;
                    }
                    Dialog::Rename(identity, name) => {
                        self.submit(identity.clone(), Action::Rename(name.clone()));
                        keep = false;
                    }
                    Dialog::Manual(_, url) => {
                        self.confirm(
                            catalog,
                            Action::ManualSource(url.trim().into()),
                            "Attach manual source URL",
                            vec![
                                url.trim().into(),
                                "Save this URL to the selected mod's metadata?".into(),
                            ],
                        );
                        keep = false;
                    }
                    Dialog::Confirm {
                        identity, action, ..
                    } => match self.approve(catalog, identity, action.clone()) {
                        Ok(_) => keep = false,
                        Err(error) => self.error = Some(error),
                    },
                    Dialog::SettingsImport(settings, _) => {
                        self.settings = settings.clone();
                        self.sync_imported_theme();
                        self.settings_page = true;
                        self.notice = Some("Backup loaded into draft. Save changes to apply it.".into());
                        self.error = None;
                        keep = false;
                    }
                }
            }
        });
        if keep && !popup.should_close() {
            self.dialog = Some(dialog);
        }
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn popup_keeps_actions_aligned_and_fits_a_small_window() {
        let fixture = tempfile::tempdir().unwrap();
        let mut catalog = Catalog::default();
        let generation = catalog.begin_scan(fixture.path().join("game"));
        catalog.accept_scan(generation, Ok(vec![]));
        let mut controls = Controls::new(
            fixture.path().join("managed"),
            fixture.path().into(),
            Settings::default(),
            || {},
        )
        .unwrap();
        let ctx = egui::Context::default();
        crate::ui::setup(&ctx);
        ctx.set_theme(egui::Theme::Light);
        ctx.all_styles_mut(|style| style.animation_time = 0.0);
        let identity = controls.identity(&catalog).unwrap();
        controls.picked(
            PickPurpose::Import(identity, true),
            Some(
                fixture
                    .path()
                    .join("long-directory".repeat(20))
                    .join("A long mod archive.zip"),
            ),
            &catalog,
        );
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(640.0, 480.0));
        let render = |controls: &mut Controls| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(screen),
                    ..Default::default()
                },
                |ui| controls.show(ui, &catalog),
            );
            output.textures_delta.clear();
            output
        };
        render(&mut controls);
        render(&mut controls);
        let output = render(&mut controls);
        let cancel = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) if text.galley.text() == "Cancel" => {
                    Some(text.pos + text.galley.rect.center().to_vec2())
                }
                _ => None,
            })
            .unwrap();
        let approve = controls.confirm_rect.unwrap();
        assert!((cancel.y - approve.center().y).abs() < 1.0);
        assert!(cancel.x < approve.left());
        let popup = ctx.read_response(egui::Id::new(("management-review", "Import ZIP"))).unwrap().rect;
        assert!(screen.contains_rect(popup));
        assert!(popup.center().distance(screen.center()) < 2.0);
        assert!(popup.contains_rect(approve));

        // A long review must not leave a large empty popup behind when the next review is short.
        controls.dialog = Some(Dialog::SettingsImport(Settings::default(), false));
        render(&mut controls);
        render(&mut controls);
        render(&mut controls);
        let popup = ctx.read_response(egui::Id::new(("management-review", "Review imported settings"))).unwrap().rect;
        assert!(
            popup.height() < 300.0,
            "short review must shrink to its content"
        );
    }

    #[test]
    fn clicking_library_behind_a_review_only_dismisses_the_popup() {
        let fixture = tempfile::tempdir().unwrap();
        let mut catalog = Catalog::default();
        let generation = catalog.begin_scan(fixture.path().join("game"));
        catalog.accept_scan(generation, Ok(vec![]));
        let mut controls = Controls::new(
            fixture.path().join("managed"),
            fixture.path().into(),
            Settings::default(),
            || {},
        )
        .unwrap();
        let identity = controls.identity(&catalog).unwrap();
        controls.picked(
            PickPurpose::Import(identity, true),
            Some(fixture.path().join("mod.zip")),
            &catalog,
        );
        let ctx = egui::Context::default();
        frame(&ctx, &mut controls, &catalog, vec![]);
        frame(&ctx, &mut controls, &catalog, vec![]);
        let pos = controls.import_rect.unwrap().center();
        frame(
            &ctx,
            &mut controls,
            &catalog,
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
        assert!(controls.dialog.is_none());
        assert!(
            controls.picker.is_none(),
            "backdrop click must not activate Import folder behind it"
        );
        assert!(controls.operations.is_empty());
    }
    #[test]
    fn reviews_are_modal_and_escape_cancels_without_submitting() {
        let fixture = tempfile::tempdir().unwrap();
        let catalog = Catalog::default();
        let mut controls = Controls::new(
            fixture.path().join("managed"),
            fixture.path().into(),
            Settings::default(),
            || {},
        )
        .unwrap();
        let ctx = egui::Context::default();
        crate::ui::setup(&ctx);
        for review in [
            Dialog::SettingsImport(Settings::default(), false),
            Dialog::Import {
                identity: Identity {
                    root: fixture.path().join("game"),
                    entry: None,
                    generation: 0,
                    operation: 0,
                },
                zip: true,
                path: fixture.path().join("A very long mod archive name.zip"),
                name: "A very long mod archive name".into(),
            },
        ] {
            controls.dialog = Some(review);
            frame(&ctx, &mut controls, &catalog, vec![]);
            frame(&ctx, &mut controls, &catalog, vec![]);
            assert!(
                ctx.memory(|memory| memory.top_modal_layer().is_some()),
                "review must block interaction with the library"
            );
            frame(
                &ctx,
                &mut controls,
                &catalog,
                vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Default::default(),
                }],
            );
            assert!(
                controls.dialog.is_none(),
                "Escape must dismiss without approval"
            );
            assert!(controls.operations.is_empty());
            assert!(!controls.settings_path.exists());
        }
    }

    #[test]
    fn source_and_trash_reviews_use_the_same_modal_dismissal() {
        let fixture = tempfile::tempdir().unwrap();
        let mut catalog = Catalog::default();
        let generation = catalog.begin_scan(fixture.path().join("game"));
        catalog.accept_scan(generation, Ok(vec![]));
        let mut controls = Controls::new(
            fixture.path().join("managed"),
            fixture.path().into(),
            Settings::default(),
            || {},
        )
        .unwrap();
        let ctx = egui::Context::default();
        for trash in [false, true] {
            let identity = controls.identity(&catalog).unwrap();
            if trash {
                controls.trash_entries = Some((identity, vec![]));
            } else {
                controls.candidates = Some((identity, vec![]));
            }
            frame(&ctx, &mut controls, &catalog, vec![]);
            frame(&ctx, &mut controls, &catalog, vec![]);
            assert!(ctx.memory(|memory| memory.top_modal_layer().is_some()));
            frame(
                &ctx,
                &mut controls,
                &catalog,
                vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Default::default(),
                }],
            );
            assert!(controls.trash_entries.is_none());
            assert!(controls.candidates.is_none());
            assert!(controls.operations.is_empty());
        }
    }
    #[test]
    fn picker_selection_requires_import_confirmation_and_rejects_a_changed_game_folder() {
        let dir = tempfile::tempdir().unwrap();
        let mut catalog = Catalog::default();
        let generation = catalog.begin_scan(dir.path().join("game"));
        catalog.accept_scan(generation, Ok(vec![]));
        let mut controls = Controls::new(
            dir.path().join("managed"),
            dir.path().into(),
            Settings::default(),
            || {},
        )
        .unwrap();
        let identity = controls.identity(&catalog).unwrap();
        let source = dir.path().join("My mod.zip");
        controls.picked(
            PickPurpose::Import(identity.clone(), true),
            Some(source.clone()),
            &catalog,
        );
        assert!(matches!(
            &controls.dialog,
            Some(Dialog::Import { zip: true, .. })
        ));
        assert!(
            matches!(&controls.dialog, Some(Dialog::Import { path, name, .. }) if path == &source && name == "My mod")
        );
        assert!(controls.operations.is_empty());
        controls.dialog = None;
        catalog.begin_scan(dir.path().join("other-game"));
        controls.picked(PickPurpose::Import(identity, true), Some(source), &catalog);
        assert!(controls.dialog.is_none());
        assert!(controls.operations.is_empty());
    }

    #[test]
    fn cancelled_picker_does_not_create_a_dialog_or_write_files() {
        let dir = tempfile::tempdir().unwrap();
        let mut controls = Controls::new(
            dir.path().join("managed"),
            dir.path().into(),
            Settings::default(),
            || {},
        )
        .unwrap();
        controls.picked(
            PickPurpose::SettingsExport(Settings::default()),
            None,
            &Catalog::default(),
        );
        assert!(controls.notice.is_none());
        assert!(controls.dialog.is_none());
        assert!(!controls.settings_path.exists());
    }

    #[test]
    fn settings_picker_previews_import_and_export_refuses_overwrite() {
        let dir = tempfile::tempdir().unwrap();
        let mut settings = Settings {
            curseforge_api_key: Some("fixture-secret".into()),
            ..Settings::default()
        };
        let mut controls = Controls::new(
            dir.path().join("managed"),
            dir.path().into(),
            settings.clone(),
            || {},
        )
        .unwrap();
        let path = dir.path().join("transfer.json");
        settings.curseforge_api_key = None;
        controls.picked(
            PickPurpose::SettingsExport(settings),
            Some(path.clone()),
            &Catalog::default(),
        );
        let bytes = std::fs::read(&path).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("fixture-secret"));
        controls.picked(
            PickPurpose::SettingsImport,
            Some(path.clone()),
            &Catalog::default(),
        );
        assert!(
            matches!(&controls.dialog, Some(Dialog::SettingsImport(settings, false)) if settings.curseforge_api_key.as_deref() == Some("fixture-secret"))
        );
        assert!(!controls.settings_path.exists());
        controls.picked(
            PickPurpose::SettingsExport(Settings::default()),
            Some(path.clone()),
            &Catalog::default(),
        );
        assert_eq!(std::fs::read(path).unwrap(), bytes);
        assert!(controls.error.as_deref().unwrap().contains("unused"));
    }
    #[test]
    fn review_text_fields_are_associated_with_visible_labels() {
        let fixture = tempfile::tempdir().unwrap();
        let mut catalog = Catalog::default();
        let generation = catalog.begin_scan(fixture.path().join("game"));
        catalog.accept_scan(generation, Ok(vec![]));
        let mut controls = Controls::new(fixture.path().join("managed"), fixture.path().into(), Settings::default(), || {}).unwrap();
        let identity = controls.identity(&catalog).unwrap();
        controls.dialog = Some(Dialog::Rename(identity, "Draft name".into()));
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut sizing = ctx.run_ui(egui::RawInput::default(), |ui| controls.show_dialogs(ui.ctx(), &catalog));
        sizing.textures_delta.clear();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| controls.show_dialogs(ui.ctx(), &catalog));
        output.textures_delta.clear();
        let tree = output.platform_output.accesskit_update.unwrap();
        assert!(tree.nodes.iter().any(|(_, node)| !node.labelled_by().is_empty()), "review fields need a label relationship");
    }

    #[test]
    fn header_mode_switch_does_not_save_unrelated_settings_drafts() {
        let fixture = tempfile::tempdir().unwrap();
        let mut controls = Controls::new(fixture.path().join("managed"), fixture.path().into(), Settings::default(), || {}).unwrap();
        controls.settings.game_roots.push(fixture.path().join("game").to_string_lossy().into());
        controls.settings.selected_root = controls.settings.game_roots.first().cloned();
        controls.settings.curseforge_api_key = Some("fixture-only".into());
        controls.set_color_mode(&egui::Context::default(), Theme::Light);
        let saved = Settings::load(&controls.settings_path).unwrap();
        assert!(saved.theme == Theme::Light);
        assert!(saved.game_roots.is_empty(), "header appearance must not commit a folder draft");
        assert!(saved.selected_root.is_none());
        assert!(saved.curseforge_api_key.is_none());
        assert_eq!(controls.settings.game_roots.len(), 1, "draft stays available for explicit save");
    }

    #[test]
    fn discard_restores_saved_values_but_keeps_the_header_mode_change() {
        let fixture = tempfile::tempdir().unwrap();
        let mut controls = Controls::new(fixture.path().join("managed"), fixture.path().into(), Settings::default(), || {}).unwrap();
        let ctx = egui::Context::default();
        assert!(!controls.settings_dirty());
        controls.settings.game_roots.push(fixture.path().join("game").to_string_lossy().into());
        controls.theme_edit = Some(crate::settings::CustomTheme::base());
        assert!(controls.settings_dirty());
        controls.set_color_mode(&ctx, Theme::Dark);
        controls.discard_preferences(&ctx);
        assert!(!controls.settings_dirty());
        assert!(controls.settings.game_roots.is_empty());
        assert!(controls.theme_edit.is_none());
        assert!(controls.settings.theme == Theme::Dark);
        controls.settings.curseforge_api_key = Some("fixture-only".into());
        controls.save_preferences(&ctx);
        assert!(!controls.settings_dirty());
        controls.settings.curseforge_api_key = None;
        controls.discard_preferences(&ctx);
        assert_eq!(controls.settings.curseforge_api_key.as_deref(), Some("fixture-only"));
    }

    #[test]
    fn importing_settings_previews_without_saving_or_applying() {
        let fixture = tempfile::tempdir().unwrap();
        let saved = Settings { theme: Theme::Light, ..Settings::default() };
        let mut controls = Controls::new(fixture.path().join("managed"), fixture.path().into(), saved.clone(), || {}).unwrap();
        saved.save(&controls.settings_path).unwrap();
        let ctx = egui::Context::default();
        crate::ui::setup(&ctx);
        crate::ui::apply_settings_theme(&ctx, &saved);
        controls.dialog = Some(Dialog::SettingsImport(Settings { theme: Theme::Dark, ..Settings::default() }, false));
        let mut render = |events| {
            let mut out = ctx.run_ui(egui::RawInput { events, ..Default::default() }, |ui| controls.show_dialogs(ui.ctx(), &Catalog::default()));
            out.textures_delta.clear();
            out
        };
        render(vec![]); render(vec![]);
        let out = render(vec![]);
        let pos = out.shapes.iter().find_map(|s| match &s.shape { egui::Shape::Text(t) if t.galley.text() == "Preview settings" => Some(t.pos + t.galley.size()/2.0), _ => None }).expect("import must offer a preview");
        render(vec![egui::Event::PointerMoved(pos), egui::Event::PointerButton {pos, button:egui::PointerButton::Primary,pressed:true,modifiers:Default::default()},egui::Event::PointerButton {pos, button:egui::PointerButton::Primary,pressed:false,modifiers:Default::default()}]);
        assert!(controls.settings.theme == Theme::Dark);
        assert!(Settings::load(&controls.settings_path).unwrap().theme == Theme::Light);
        assert_eq!(ctx.theme(), egui::Theme::Light);
        assert!(controls.settings_dirty());
        controls.save_preferences(&ctx);
        assert!(Settings::load(&controls.settings_path).unwrap().theme == Theme::Dark);
    }

    #[test]
    fn rendering_settings_does_not_create_an_empty_key_draft() {
        let fixture = tempfile::tempdir().unwrap();
        let mut controls = Controls::new(fixture.path().join("managed"), fixture.path().into(), Settings::default(), || {}).unwrap();
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| controls.settings_content(ui, &Catalog::default(), "Back to library"));
        output.textures_delta.clear();
        assert!(!controls.settings_dirty());
    }

    #[test]
    fn mode_switch_preserves_custom_theme_and_unsaved_editor_colors() {
        let fixture = tempfile::tempdir().unwrap();
        let mut theme = crate::settings::CustomTheme::base();
        theme.name = "My theme".into();
        theme.dark.accent = "#E8B7F4".into();
        let settings = Settings {
            theme: Theme::Dark,
            active_custom_theme: Some(theme.name.clone()),
            custom_themes: vec![theme.clone()],
            ..Settings::default()
        };
        let mut controls = Controls::new(fixture.path().join("managed"), fixture.path().into(), settings, || {}).unwrap();
        controls.theme_edit.as_mut().unwrap().dark.accent = "#123456".into();
        let ctx = egui::Context::default();
        controls.set_color_mode(&ctx, Theme::Light);
        assert_eq!(ctx.theme(), egui::Theme::Light);
        for mode in [egui::Theme::Light, egui::Theme::Dark] {
            let style = ctx.style_of(mode);
            let (background, accent) = if mode == egui::Theme::Dark {
                ([0x17, 0x19, 0x1E], [0xE8, 0xB7, 0xF4])
            } else {
                ([0xED, 0xF4, 0xF8], [0x12, 0x63, 0xCA])
            };
            assert_eq!(style.visuals.panel_fill, egui::Color32::from_rgb(background[0], background[1], background[2]));
            assert_eq!(style.visuals.hyperlink_color, egui::Color32::from_rgb(accent[0], accent[1], accent[2]));
        }
        assert_eq!(controls.settings.active_custom_theme.as_deref(), Some("My theme"));
        assert_eq!(controls.theme_edit.as_ref().unwrap().dark.accent, "#123456");
        assert_eq!(controls.theme_edit_original.as_deref(), Some("My theme"));
        let saved = Settings::load(&controls.settings_path).unwrap();
        assert!(saved.theme == Theme::Light);
        assert_eq!(saved.active_custom_theme.as_deref(), Some("My theme"));
        assert_eq!(saved.custom_themes[0].dark.accent, "#E8B7F4");
        controls.set_color_mode(&ctx, Theme::Dark);
        assert_eq!(ctx.theme(), egui::Theme::Dark);
        assert_eq!(controls.settings.active_custom_theme.as_deref(), Some("My theme"));
        controls.settings.theme = Theme::Light;
        assert_eq!(controls.theme_edit.as_ref().unwrap().dark.accent, "#123456");
        controls.use_default_colors(&ctx);
        controls.save_preferences(&ctx);
        let saved = Settings::load(&controls.settings_path).unwrap();
        assert!(saved.active_custom_theme.is_none());
        assert_eq!(saved.custom_themes[0].name, "My theme");
    }

    #[test]
    fn startup_recovery_keeps_window_busy_until_completion_is_polled() {
        let fixture = tempfile::tempdir().unwrap();
        let mut controls = Controls::new(
            fixture.path().join("managed"),
            fixture.path().to_path_buf(),
            Settings::default(),
            || {},
        )
        .unwrap();
        assert!(
            controls.busy(),
            "startup recovery must prevent blocking window shutdown"
        );
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while controls.busy() {
            controls.poll(&mut Catalog::default());
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
    use super::*;
    fn frame(
        ctx: &egui::Context,
        controls: &mut Controls,
        catalog: &Catalog,
        events: Vec<egui::Event>,
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1100.0, 800.0),
                )),
                events,
                ..Default::default()
            },
            |ui| {
                controls.show(ui, catalog);
            },
        );
        output.textures_delta.clear();
        output
    }
    #[test]
    fn more_actions_closes_popup_before_opening_rename_dialog() {
        let fixture = tempfile::tempdir().unwrap();
        let mut catalog = Catalog::default();
        let generation = catalog.begin_scan(fixture.path().join("game"));
        catalog.accept_scan(
            generation,
            Ok(vec![serde_json::from_value(serde_json::json!({
            "key": "fixture", "id": "fixture", "name": "Fixture mod", "files": ["mod.package"],
            "mod_files": ["mod.package"], "source": "managed", "enabled": false, "group_path": []
        })).unwrap()]),
        );
        catalog.select(0);
        let mut controls = Controls::new(
            fixture.path().join("managed"),
            fixture.path().to_path_buf(),
            Settings::default(),
            || {},
        )
        .unwrap();
        let ctx = egui::Context::default();
        crate::ui::setup(&ctx);
        let text_position = |output: &egui::FullOutput, label: &str| {
            output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) if text.galley.text() == label => {
                        Some(text.pos + text.galley.rect.center().to_vec2())
                    }
                    _ => None,
                })
                .unwrap_or_else(|| panic!("missing control {label}"))
        };
        let click = |pos| {
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
            ]
        };
        frame(&ctx, &mut controls, &catalog, vec![]);
        let output = frame(&ctx, &mut controls, &catalog, vec![]);
        frame(
            &ctx,
            &mut controls,
            &catalog,
            click(text_position(&output, "More actions")),
        );
        let output = frame(&ctx, &mut controls, &catalog, vec![]);
        frame(
            &ctx,
            &mut controls,
            &catalog,
            click(text_position(&output, "Rename")),
        );
        assert!(matches!(controls.dialog, Some(Dialog::Rename(_, _))));
        assert!(
            !egui::Popup::is_any_open(&ctx),
            "action popup must close when its dialog opens"
        );
    }
    #[test]
    fn dialog_confirm_click_enqueues_and_stale_selection_cannot_confirm() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("game");
        std::fs::create_dir_all(root.join("Mods")).unwrap();
        let mut catalog = Catalog::default();
        catalog.begin_scan(root.clone());
        let mut controls = Controls::new(
            dir.path().join("managed"),
            dir.path().to_path_buf(),
            Settings::default(),
            || {},
        )
        .unwrap();
        controls.confirm(
            &catalog,
            Action::RemoveSource,
            "Remove source",
            vec!["fixture".into()],
        );
        let ctx = egui::Context::default();
        frame(&ctx, &mut controls, &catalog, vec![]);
        frame(&ctx, &mut controls, &catalog, vec![]);
        let pos = controls.confirm_rect.unwrap().center();
        frame(
            &ctx,
            &mut controls,
            &catalog,
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
        assert_eq!(controls.operations.len(), 1);
        controls.confirm(&catalog, Action::RemoveSource, "Remove source", vec![]);
        catalog.begin_scan(dir.path().join("other"));
        frame(&ctx, &mut controls, &catalog, vec![]);
        assert!(controls.confirm_rect.is_none());
        assert_eq!(controls.operations.len(), 1);
    }
    #[test]
    fn toolbar_pointer_opens_import_without_submitting_unapproved_job() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("game");
        let mut catalog = Catalog::default();
        let generation = catalog.begin_scan(root);
        catalog.accept_scan(generation, Ok(vec![]));
        let mut controls = Controls::new(
            dir.path().join("managed"),
            dir.path().to_path_buf(),
            Settings::default(),
            || {},
        )
        .unwrap();
        let ctx = egui::Context::default();
        frame(&ctx, &mut controls, &catalog, vec![]);
        frame(&ctx, &mut controls, &catalog, vec![]);
        let pos = controls.import_rect.unwrap().center();
        frame(
            &ctx,
            &mut controls,
            &catalog,
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
        assert!(controls.picker.is_some());
        assert!(controls.dialog.is_none());
        assert!(controls.operations.is_empty());
        controls.picker_sender.take().unwrap().send(Some(dir.path().join("Chosen folder"))).unwrap();
        controls.poll(&mut catalog);
        assert!(matches!(controls.dialog, Some(Dialog::Import { zip: false, .. })));
        assert!(controls.operations.is_empty());
    }
    #[test]
    fn late_failed_review_keeps_its_history_identity_without_replacing_new_selection() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("old");
        std::fs::create_dir_all(root.join("Mods")).unwrap();
        let mut catalog = Catalog::default();
        catalog.begin_scan(root.clone());
        catalog.selected = Some(crate::catalog::EntryId::Managed("missing".into()));
        let mut controls = Controls::new(
            dir.path().join("managed"),
            dir.path().to_path_buf(),
            Settings::default(),
            || {},
        )
        .unwrap();
        let startup_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while controls.refresh.is_none() {
            controls.poll(&mut catalog);
            assert!(std::time::Instant::now() < startup_deadline);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        controls.refresh = None;
        let identity = controls.identity(&catalog).unwrap();
        controls.submit(identity.clone(), Action::Toggle { enabled: true });
        catalog.begin_scan(dir.path().join("new"));
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while controls.busy() {
            controls.poll(&mut catalog);
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(controls.dialog.is_none());
        assert!(controls.candidates.is_none());
        assert!(controls.refresh.is_none());
        assert!(catalog.error.is_none());
        assert!(controls.operations[0].error);
        assert_eq!(controls.operations[0].job.identity, identity);
    }
    #[test]
    fn manual_source_metadata_is_written_only_after_actual_confirmation_click() {
        let fixture = tempfile::tempdir().unwrap();
        let root = fixture.path().join("game");
        let managed = fixture.path().join("managed");
        let source = fixture.path().join("source");
        std::fs::create_dir_all(root.join("Mods")).unwrap();
        std::fs::create_dir_all(&source).unwrap();
        std::fs::write(source.join("test.package"), b"fixture").unwrap();
        let metadata = ts4_mod_manager_core::managed_storage::create_managed_mod(
            &managed,
            ts4_mod_manager_core::managed_storage::ImportRequest {
                name: "Fixture".into(),
                slug: None,
                source_dir: source,
            },
        )
        .unwrap();
        let mut catalog = Catalog::default();
        let generation = catalog.begin_scan(root);
        catalog.accept_scan(
            generation,
            Ok(
                crate::worker::scan(catalog.root.as_ref().unwrap(), &managed)
                    .unwrap()
                    .mods,
            ),
        );
        catalog.select(0);
        let mut controls = Controls::new(
            managed.clone(),
            fixture.path().to_path_buf(),
            Settings::default(),
            || {},
        )
        .unwrap();
        controls.confirm(
            &catalog,
            Action::ManualSource("https://example.org/fixture".into()),
            "Attach manual source URL",
            vec!["https://example.org/fixture".into()],
        );
        assert!(
            ts4_mod_manager_core::managed_storage::read_managed_mod(&managed, &metadata.mod_id)
                .unwrap()
                .source_url
                .is_none()
        );
        let context = egui::Context::default();
        frame(&context, &mut controls, &catalog, vec![]);
        frame(&context, &mut controls, &catalog, vec![]);
        let pos = controls.confirm_rect.unwrap().center();
        frame(
            &context,
            &mut controls,
            &catalog,
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
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while controls.busy() {
            controls.poll(&mut catalog);
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert_eq!(
            ts4_mod_manager_core::managed_storage::read_managed_mod(&managed, &metadata.mod_id)
                .unwrap()
                .source_url
                .as_deref(),
            Some("https://example.org/fixture")
        );
    }
    #[test]
    fn shared_managed_mutation_refreshes_the_current_root_after_navigation() {
        let fixture = tempfile::tempdir().unwrap();
        let root_a = fixture.path().join("a");
        let root_b = fixture.path().join("b");
        let source = fixture.path().join("source");
        for path in [root_a.join("Mods"), root_b.join("Mods"), source.clone()] {
            std::fs::create_dir_all(path).unwrap();
        }
        std::fs::write(source.join("test.package"), b"fixture").unwrap();
        let mut catalog = Catalog::default();
        catalog.begin_scan(root_a);
        let mut controls = Controls::new(
            fixture.path().join("managed"),
            fixture.path().to_path_buf(),
            Settings::default(),
            || {},
        )
        .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while controls.refresh.is_none() {
            controls.poll(&mut catalog);
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        controls.refresh = None;
        let identity = controls.identity(&catalog).unwrap();
        controls.submit(
            identity,
            Action::ImportFolder {
                path: source,
                name: "Fixture".into(),
            },
        );
        catalog.begin_scan(root_b.clone());
        while controls.busy() {
            controls.poll(&mut catalog);
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert_eq!(controls.refresh, Some(root_b));
        assert!(!controls.operations[0].error);
    }
}
