use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::Duration,
};
use plumcrate_core::{
    browser_import::{BrowserProject, load_browser_project},
    curseforge_browse::BrowseProject,
    curseforge_client::{CurseForgeClient, SourceLookupError, UreqCurseForgeTransport},
    curseforge_details::{self, DescriptionDocument, ProjectReference, SafeUrl},
};

#[derive(Clone, Debug, Default)]
pub enum Load<T> {
    #[default]
    Idle,
    Loading,
    Ready(T),
    Failed(String),
}
#[derive(Clone, Debug)]
pub enum ReviewRequest {
    ProjectId(u64),
    Loaded {
        project: BrowserProject,
        file_id: u64,
    },
}
#[derive(Default)]
pub struct Details {
    pub description: Load<DescriptionDocument>,
    pub project: Load<BrowserProject>,
    pub chosen: Option<u64>,
    pub references: BTreeMap<u64, Load<ProjectReference>>,
    pub author_projects: BTreeMap<SafeUrl, Load<ProjectReference>>,
    scope: Option<(u64, [u8; 32])>,
    token: u64,
    reference_file: Option<u64>,
    epoch: Arc<AtomicU64>,
    file_epoch: Arc<AtomicU64>,
    jobs: Option<Jobs>,
}
fn digest(key: Option<&str>) -> [u8; 32] {
    Sha256::digest(key.unwrap_or_default().trim().as_bytes()).into()
}
impl Details {
    pub fn begin(&mut self, project_id: u64, key: Option<&str>) -> u64 {
        self.clear();
        self.scope = Some((project_id, digest(key)));
        self.description = Load::Loading;
        self.project = Load::Loading;
        self.token
    }
    pub fn clear(&mut self) {
        self.token = self.token.wrapping_add(1);
        self.epoch.store(self.token, Ordering::Release);
        self.file_epoch.store(0, Ordering::Release);
        self.scope = None;
        self.description = Load::Idle;
        self.project = Load::Idle;
        self.chosen = None;
        self.reference_file = None;
        self.references.clear();
        self.author_projects.clear();
    }
    pub fn sync(
        &mut self,
        ctx: &egui::Context,
        selected: Option<&BrowseProject>,
        key: Option<&str>,
    ) {
        let scope = selected.map(|p| (p.id, digest(key)));
        if scope != self.scope {
            if let Some(project) = selected {
                self.begin(project.id, key);
                self.start_sections(ctx, project, key, true, true);
            } else {
                self.clear();
            }
        }
        let events = self
            .jobs
            .as_ref()
            .map(|jobs| jobs.receiver.try_iter().collect::<Vec<_>>())
            .unwrap_or_default();
        for event in events {
            match event {
                Event::Description(token, result) => {
                    self.accept_description(token, result);
                }
                Event::Project(token, result) => {
                    self.accept_project(token, result);
                }
                Event::Reference(token, file, id, result) => {
                    self.accept_reference(token, file, id, result);
                }
                Event::Author(token, url, result)
                    if token == self.token && self.scope.is_some() =>
                {
                    self.author_projects.insert(url, outcome(result));
                }
                Event::Author(..) => {}
            }
        }
        self.sync_references(ctx, key);
    }
    fn jobs(&mut self, ctx: &egui::Context) -> Option<&Jobs> {
        if self.jobs.is_none() {
            self.jobs = Jobs::start(ctx.clone(), self.epoch.clone(), self.file_epoch.clone()).ok();
        }
        self.jobs.as_ref()
    }
    fn start_sections(
        &mut self,
        ctx: &egui::Context,
        project: &BrowseProject,
        key: Option<&str>,
        description: bool,
        releases: bool,
    ) {
        let Some(key) = key.map(str::trim).filter(|s| !s.is_empty()) else {
            if description {
                self.description = Load::Failed(
                    "Save your CurseForge key in Settings to load the description.".into(),
                );
            }
            if releases {
                self.project = Load::Failed(
                    "Save your CurseForge key in Settings to load releases and dependencies."
                        .into(),
                );
            }
            return;
        };
        let token = self.token;
        let Some(jobs) = self.jobs(ctx) else {
            self.description =
                Load::Failed("Could not start the project lookup. Try again.".into());
            self.project = Load::Failed("Could not start the release lookup. Try again.".into());
            return;
        };
        if description {
            let _ = jobs.descriptions.send(Request {
                token,
                key: key.into(),
                kind: RequestKind::Description(
                    project.id,
                    project
                        .source_url
                        .clone()
                        .unwrap_or_else(|| "https://www.curseforge.com/".into()),
                ),
            });
        }
        if releases {
            let _ = jobs.projects.send(Request {
                token,
                key: key.into(),
                kind: RequestKind::Project(project.id),
            });
        }
    }
    pub fn retry_description(
        &mut self,
        ctx: &egui::Context,
        project: &BrowseProject,
        key: Option<&str>,
    ) {
        self.description = Load::Loading;
        self.start_sections(ctx, project, key, true, false);
    }
    pub fn retry_project(
        &mut self,
        ctx: &egui::Context,
        project: &BrowseProject,
        key: Option<&str>,
    ) {
        self.project = Load::Loading;
        self.start_sections(ctx, project, key, false, true);
    }
    pub fn retry_references(&mut self) {
        self.reference_file = None;
    }
    pub fn inspect_author(&mut self, ctx: &egui::Context, url: SafeUrl, key: Option<&str>) {
        let Some(key) = key.map(str::trim).filter(|s| !s.is_empty()) else {
            return;
        };
        let token = self.token;
        if let Some(jobs) = self.jobs(ctx) {
            let _ = jobs.references.send(Request {
                token,
                key: key.into(),
                kind: RequestKind::Author(url.clone()),
            });
            self.author_projects.insert(url, Load::Loading);
        }
    }
    pub fn accept_description(
        &mut self,
        token: u64,
        result: Result<DescriptionDocument, SourceLookupError>,
    ) -> bool {
        if token != self.token || self.scope.is_none() {
            return false;
        }
        self.description = outcome(result);
        true
    }
    pub fn accept_project(&mut self, token: u64, result: Result<BrowserProject, String>) -> bool {
        if token != self.token || self.scope.is_none() {
            return false;
        }
        self.project = match result {
            Ok(project) => {
                self.chosen = project.releases.first().map(|r| r.release.file_id);
                Load::Ready(project)
            }
            Err(error) => Load::Failed(error),
        };
        self.reference_file = None;
        true
    }
    pub fn accept_reference(
        &mut self,
        token: u64,
        file: u64,
        id: u64,
        result: Result<ProjectReference, SourceLookupError>,
    ) -> bool {
        if token != self.token || self.scope.is_none() || self.chosen != Some(file) {
            return false;
        }
        self.references.insert(id, outcome(result));
        true
    }
    fn sync_references(&mut self, ctx: &egui::Context, key: Option<&str>) {
        if self.reference_file == self.chosen {
            return;
        }
        self.reference_file = self.chosen;
        self.references.clear();
        self.file_epoch
            .store(self.chosen.unwrap_or_default(), Ordering::Release);
        let Load::Ready(project) = &self.project else {
            return;
        };
        let Some(file) = project
            .releases
            .iter()
            .find(|f| Some(f.release.file_id) == self.chosen)
        else {
            return;
        };
        let file_id = file.release.file_id;
        let mut ids = file
            .dependencies
            .relations
            .iter()
            .map(|r| r.project_id)
            .collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        if ids.is_empty() {
            return;
        }
        for id in &ids {
            self.references.insert(*id, Load::Loading);
        }
        let Some(key) = key.filter(|s| !s.trim().is_empty()) else {
            for value in self.references.values_mut() {
                *value = Load::Failed("Save your CurseForge key to inspect this project.".into());
            }
            return;
        };
        let token = self.token;
        if let Some(jobs) = self.jobs(ctx) {
            let _ = jobs.references.send(Request {
                token,
                key: key.into(),
                kind: RequestKind::References(file_id, ids),
            });
        }
    }
}
fn outcome<T>(result: Result<T, SourceLookupError>) -> Load<T> {
    match result {
        Ok(value) => Load::Ready(value),
        Err(error) => Load::Failed(
            match error {
                SourceLookupError::Unauthorized => {
                    "CurseForge rejected the saved key. Check Settings."
                }
                SourceLookupError::RateLimited => {
                    "CurseForge is receiving too many requests. Wait and try again."
                }
                SourceLookupError::NoUsableEvidence => {
                    "No exact CurseForge project matched this author link."
                }
                _ => "Could not read this information from CurseForge. Try again.",
            }
            .into(),
        ),
    }
}
struct Request {
    token: u64,
    key: String,
    kind: RequestKind,
}
enum RequestKind {
    Description(u64, String),
    Project(u64),
    References(u64, Vec<u64>),
    Author(SafeUrl),
}
enum Event {
    Description(u64, Result<DescriptionDocument, SourceLookupError>),
    Project(u64, Result<BrowserProject, String>),
    Reference(u64, u64, u64, Result<ProjectReference, SourceLookupError>),
    Author(u64, SafeUrl, Result<ProjectReference, SourceLookupError>),
}
struct Jobs {
    descriptions: mpsc::Sender<Request>,
    projects: mpsc::Sender<Request>,
    references: mpsc::Sender<Request>,
    receiver: mpsc::Receiver<Event>,
}
impl Jobs {
    fn start(
        ctx: egui::Context,
        epoch: Arc<AtomicU64>,
        file_epoch: Arc<AtomicU64>,
    ) -> std::io::Result<Self> {
        let (sender, receiver) = mpsc::channel();
        let mut channels = Vec::new();
        for index in 0..3 {
            let (requests, pending) = mpsc::channel::<Request>();
            let (sender, ctx, epoch, file_epoch) = (
                sender.clone(),
                ctx.clone(),
                epoch.clone(),
                file_epoch.clone(),
            );
            std::thread::Builder::new()
                .name(format!("curseforge-detail-{index}"))
                .spawn(move || {
                    while let Ok(mut request) = pending.recv() {
                        if index != 2 {
                            for newer in pending.try_iter() {
                                request = newer;
                            }
                        }
                        if request.token != epoch.load(Ordering::Acquire) {
                            continue;
                        }
                        let client = CurseForgeClient::new(
                            Some(&request.key),
                            UreqCurseForgeTransport::with_budget(Duration::from_secs(20)),
                        )
                        .expect("request contains saved key");
                        let token = request.token;
                        match request.kind {
                            RequestKind::Description(id, url) => {
                                let _ = sender.send(Event::Description(
                                    token,
                                    curseforge_details::load_description(&client, id, &url),
                                ));
                            }
                            RequestKind::Project(id) => {
                                let _ = sender.send(Event::Project(
                                    token,
                                    load_browser_project(&client, id).map_err(|_| {
                                        "Could not load verified releases. Try again.".into()
                                    }),
                                ));
                            }
                            RequestKind::References(file, ids) => {
                                for id in ids {
                                    if token != epoch.load(Ordering::Acquire)
                                        || file != file_epoch.load(Ordering::Acquire)
                                    {
                                        break;
                                    }
                                    let _ = sender.send(Event::Reference(
                                        token,
                                        file,
                                        id,
                                        curseforge_details::load_project_reference(&client, id),
                                    ));
                                    ctx.request_repaint();
                                }
                            }
                            RequestKind::Author(url) => {
                                let result =
                                    curseforge_details::resolve_author_project(&client, &url);
                                let _ = sender.send(Event::Author(token, url, result));
                            }
                        }
                        ctx.request_repaint();
                    }
                })?;
            channels.push(requests);
        }
        Ok(Self {
            descriptions: channels.remove(0),
            projects: channels.remove(0),
            references: channels.remove(0),
            receiver,
        })
    }
}
