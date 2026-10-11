use crate::catalog::EntryId;
use std::{path::PathBuf, sync::mpsc, thread::JoinHandle};
use ts4_mod_manager_core::{
    managed_storage as storage, source_candidates::SourceCandidate, toggle,
    mod_updates::{Release, UpdateReport},
};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Identity {
    pub operation: u64,
    pub root: PathBuf,
    pub entry: Option<EntryId>,
    pub generation: u64,
}
impl Identity {
    pub fn current(
        &self,
        root: Option<&PathBuf>,
        entry: Option<&EntryId>,
        generation: u64,
    ) -> bool {
        Some(&self.root) == root && self.entry.as_ref() == entry && self.generation == generation
    }
}
#[derive(Clone)]
pub enum Action {
    ImportFolder { path: PathBuf, name: String },
    ImportZip { path: PathBuf, name: String },
    ReviewToggle { enabled: bool },
    Toggle { enabled: bool, review: String },
    Migrate { files: Vec<String> },
    Rename(String),
    ManualSource(String),
    Attach(SourceCandidate),
    RemoveSource,
    Trash,
    ListTrash,
    Restore(String),
    Lookup(Option<String>),
    CheckUpdates {
        mod_ids: Vec<String>,
        api_key: Option<String>,
        epoch: std::sync::Arc<std::sync::atomic::AtomicU64>,
        token: u64,
    },
    RecordRelease { report: Box<UpdateReport>, release: Release, token: u64 },
    ImportBrowser(Box<crate::browser_import::ReadyImport>),
}
impl Action {
    pub fn read_only(&self) -> bool {
        matches!(self, Self::Lookup(_) | Self::CheckUpdates { .. })
    }
    pub fn label(&self) -> &str {
        match self {
            Self::ImportFolder { .. } => "Import folder",
            Self::ImportZip { .. } => "Import ZIP",
            Self::ReviewToggle { enabled: true } | Self::Toggle { enabled: true, .. } => "Enable",
            Self::ReviewToggle { enabled: false } | Self::Toggle { enabled: false, .. } => {
                "Disable"
            }
            Self::Migrate { .. } => "Manage external mod",
            Self::Rename(_) => "Rename",
            Self::ManualSource(_) | Self::Attach(_) => "Attach source",
            Self::RemoveSource => "Remove source",
            Self::Trash => "Move to trash",
            Self::ListTrash => "Read trash",
            Self::Restore(_) => "Restore",
            Self::Lookup(_) => "Find source",
            Self::CheckUpdates { .. } => "Check updates",
            Self::RecordRelease { .. } => "Record installed release",
            Self::ImportBrowser(_) => "Import and enable browser download",
        }
    }
}
#[derive(Clone)]
pub struct Job {
    pub identity: Identity,
    pub managed: PathBuf,
    pub trash: PathBuf,
    pub action: Action,
}
pub enum Output {
    Changed(Vec<String>),
    Review(toggle::DryRunResult),
    Candidates(Vec<SourceCandidate>),
    Trash(Vec<ts4_mod_manager_core::lifecycle::TrashEntry>),
    Updates { token: u64, reports: Vec<(String, Result<UpdateReport, String>)> },
    BrowserImported { token: u64, metadata: Box<storage::ModMetadata> },
}
pub enum Event {
    Recovered(Result<Vec<String>, String>),
    Running(Identity),
    Finished(Identity, Result<Output, String>),
}
pub struct Mutations {
    sender: Option<mpsc::Sender<Job>>,
    lookups: Option<mpsc::Sender<Job>>,
    results: mpsc::Receiver<Event>,
    thread: Option<JoinHandle<()>>,
}
impl Mutations {
    pub fn start(wake: impl Fn() + Send + Sync + 'static) -> std::io::Result<Self> {
        Self::start_worker(None, wake)
    }
    pub fn start_with_recovery(
        managed: PathBuf,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> std::io::Result<Self> {
        Self::start_worker(Some(managed), wake)
    }
    fn start_worker(
        recovery: Option<PathBuf>,
        wake: impl Fn() + Send + Sync + 'static,
    ) -> std::io::Result<Self> {
        Self::start_executor(recovery, wake, execute)
    }
    fn start_executor(
        recovery: Option<PathBuf>,
        wake: impl Fn() + Send + Sync + 'static,
        execute: impl Fn(&Job) -> Result<Output, String> + Send + Sync + 'static,
    ) -> std::io::Result<Self> {
        let wake = std::sync::Arc::new(wake);
        let execute = std::sync::Arc::new(execute);
        let (sender, inbox) = mpsc::channel::<Job>();
        let (out, results) = mpsc::channel();
        let (lookup_sender, lookup_inbox) = mpsc::channel::<Job>();
        let lookup_out = out.clone();
        let lookup_wake = wake.clone();
        let lookup_execute = execute.clone();
        // Source requests are read-only. Closing the app must not wait on DNS or provider IO.
        std::thread::Builder::new().name("source-lookups".into()).spawn(move || {
            while let Ok(job) = lookup_inbox.recv() {
                if lookup_out.send(Event::Running(job.identity.clone())).is_err() { break; }
                lookup_wake();
                let result = lookup_execute(&job);
                if lookup_out.send(Event::Finished(job.identity, result)).is_err() { break; }
                lookup_wake();
            }
        })?;
        let thread = std::thread::Builder::new()
            .name("approved-management-fifo".into())
            .spawn(move || {
                if let Some(managed) = recovery {
                    let result = ts4_mod_manager_core::operation::recover_pending(&managed)
                        .map(|issues| issues.into_iter().map(|i| i.message).collect())
                        .map_err(|e| format!("{}: {}", e.code.as_str(), e.message));
                    let _ = out.send(Event::Recovered(result));
                    wake();
                }
                while let Ok(job) = inbox.recv() {
                    let _ = out.send(Event::Running(job.identity.clone()));
                    wake();
                    let result = execute(&job);
                    let _ = out.send(Event::Finished(job.identity, result));
                    wake();
                }
            })?;
        Ok(Self {
            sender: Some(sender),
            lookups: Some(lookup_sender),
            results,
            thread: Some(thread),
        })
    }
    pub fn submit(&self, job: Job) -> Result<(), String> {
        let sender = if job.action.read_only() { &self.lookups } else { &self.sender };
        sender.as_ref()
            .ok_or("Management worker stopped")?
            .send(job)
            .map_err(|_| "Management worker stopped".into())
    }
    pub fn drain(&self) -> impl Iterator<Item = Event> + '_ {
        self.results.try_iter()
    }
}
impl Drop for Mutations {
    fn drop(&mut self) {
        self.lookups.take();
        self.sender.take();
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
fn id(job: &Job) -> Result<&str, String> {
    match job.identity.entry.as_ref() {
        Some(EntryId::Managed(id)) => Ok(id),
        _ => Err("Select a managed mod first".into()),
    }
}
fn changed() -> Result<Output, String> {
    Ok(Output::Changed(vec![]))
}
pub fn execute(job: &Job) -> Result<Output, String> {
    let mods = job.identity.root.join("Mods");
    let managed = &job.managed;
    let error = |e: ts4_mod_manager_core::error::ManagerError| {
        format!("{}: {}", e.code.as_str(), e.message)
    };
    match &job.action {
        Action::ImportFolder { path, name } => {
            storage::create_managed_mod(
                managed,
                storage::ImportRequest {
                    name: name.clone(),
                    slug: None,
                    source_dir: path.clone(),
                },
            )
            .map_err(error)?;
            changed()
        }
        Action::ImportZip { path, name } => {
            ts4_mod_manager_core::archive_import::import_archive_to_managed(
                managed,
                path,
                name.clone(),
                None,
            )
            .map_err(error)?;
            changed()
        }
        Action::ReviewToggle { enabled } => Ok(Output::Review(
            toggle::dry_run_toggle(managed, &mods, id(job)?, *enabled).map_err(error)?,
        )),
        Action::Toggle { enabled, review } => {
            let _authority = ts4_mod_manager_core::operation::acquire(managed).map_err(error)?;
            let current =
                toggle::dry_run_toggle(managed, &mods, id(job)?, *enabled).map_err(error)?;
            if !current.can_apply
                || serde_json::to_string(&current).map_err(|e| e.to_string())? != *review
            {
                return Err(
                    "Files changed since review. Review the paths again before applying.".into(),
                );
            }
            let result = toggle::apply_toggle(managed, &mods, id(job)?, *enabled).map_err(error)?;
            if !result.applied {
                return Err(result
                    .issues
                    .iter()
                    .map(|i| i.message.clone())
                    .collect::<Vec<_>>()
                    .join("\n"));
            }
            Ok(Output::Changed(
                result.issues.into_iter().map(|i| i.message).collect(),
            ))
        }
        Action::Migrate { files } => {
            let _authority = ts4_mod_manager_core::operation::acquire(managed).map_err(error)?;
            let Some(EntryId::External(key)) = &job.identity.entry else {
                return Err("Select an external mod".into());
            };
            let current = ts4_mod_manager_core::mod_scan::scan_mods(&mods, managed).map_err(error)?
                .into_iter()
                .find(|m| m.key == *key)
                .ok_or("External mod no longer exists")?;
            if current.files != *files {
                return Err("External files changed. Review them again.".into());
            }
            let result =
                ts4_mod_manager_core::external_migration::migrate_external_mod(managed, &mods, key)
                    .map_err(error)?;
            Ok(Output::Changed(
                result.issues.into_iter().map(|i| i.message).collect(),
            ))
        }
        Action::Rename(name) => {
            storage::set_custom_display_name(managed, id(job)?, name.clone()).map_err(error)?;
            changed()
        }
        Action::ManualSource(url) => {
            let parsed = url::Url::parse(url).map_err(|_| "Enter a valid http or https URL")?;
            if !["http", "https"].contains(&parsed.scheme()) {
                return Err("Enter a valid http or https URL".into());
            }
            if !parsed.username().is_empty() || parsed.password().is_some() {
                return Err("Source URLs must not include credentials".into());
            }
            storage::set_source_url(managed, id(job)?, url.clone(), None, None, None, None)
                .map_err(error)?;
            changed()
        }
        Action::Attach(c) => {
            let attachment = storage::SourceAttachmentMetadata {
                provider_id: "curseforge".into(),
                project_id: c.project_id,
                file_id: c.file_id,
                source_url: c.source_url.clone(),
                title: c.title.clone(),
                author: c.author.clone(),
                confidence: Some(c.confidence),
                reasons: c.reasons.clone(),
                evidence: c
                    .evidence
                    .iter()
                    .map(|e| storage::SourceEvidence {
                        kind: e.kind.clone(),
                        description: e.description.clone(),
                        weight: e.weight,
                    })
                    .collect(),
                attached_by: "user".into(),
                attached_at: chrono::Utc::now().to_rfc3339(),
                installed_release: None,
            };
            storage::set_source_url(
                managed,
                id(job)?,
                c.source_url.clone(),
                Some("curseforge".into()),
                None,
                c.preview_url.clone(),
                Some(attachment),
            )
            .map_err(error)?;
            changed()
        }
        Action::RemoveSource => {
            storage::remove_source_url(managed, id(job)?).map_err(error)?;
            changed()
        }
        Action::ImportBrowser(request) => {
            let metadata = ts4_mod_manager_core::browser_import::import_browser_download(managed, &request.root.join("Mods"), &request.project, &request.release, &request.matched, &request.permit)
                .map_err(|error| error.message)?;
            Ok(Output::BrowserImported { token: request.token, metadata: Box::new(metadata) })
        }
        Action::RecordRelease { report, release, .. } => {
            ts4_mod_manager_core::mod_updates::confirm_installed_release(managed, id(job)?, report, release)
                .map_err(error)?;
            changed()
        }
        Action::CheckUpdates { mod_ids, api_key, epoch, token } => {
            use std::sync::atomic::Ordering;
            use ts4_mod_manager_core::curseforge_client::{CurseForgeClient, UreqCurseForgeTransport};
            let started = std::time::Instant::now();
            let client = CurseForgeClient::new(api_key.as_deref(), CachedTransport {
                inner: UreqCurseForgeTransport::default(),
                responses: std::cell::RefCell::default(),
            });
            let mut reports = Vec::new();
            for mod_id in mod_ids {
                if epoch.load(Ordering::Acquire) != *token { break; }
                let report = if started.elapsed() > std::time::Duration::from_secs(25) {
                    Err("Check timed out. Check this mod separately or try again.".into())
                } else {
                    match &client {
                        Ok(client) => ts4_mod_manager_core::mod_updates::check_mod_updates(managed, mod_id, client)
                            .map_err(update_error),
                        Err(_) => Err("Save a valid CurseForge API key in Settings, then try again.".into()),
                    }
                };
                reports.push((mod_id.clone(), report));
            }
            Ok(Output::Updates { token: *token, reports })
        }
        Action::Trash => {
            storage::read_managed_mod(managed, id(job)?).map_err(error)?;
            let result = ts4_mod_manager_core::lifecycle::uninstall_managed_mod(
                managed,
                &mods,
                &job.trash,
                id(job)?,
            )
            .map_err(error)?;
            Ok(Output::Changed(
                result.issues.into_iter().map(|i| i.message).collect(),
            ))
        }
        Action::ListTrash => Ok(Output::Trash(
            ts4_mod_manager_core::lifecycle::list_trash_entries(&job.trash).map_err(error)?,
        )),
        Action::Restore(name) => {
            ts4_mod_manager_core::lifecycle::restore_trashed_mod(managed, &mods, &job.trash, name)
                .map_err(error)?;
            changed()
        }
        Action::Lookup(key) => Ok(Output::Candidates(
            ts4_mod_manager_core::source_lookup::find_source_candidates(
                managed,
                &mods,
                id(job)?,
                key.as_deref(),
            )
            .map_err(error)?,
        )),
    }
}

fn update_error(error: ts4_mod_manager_core::error::ManagerError) -> String {
    use ts4_mod_manager_core::error::ErrorCode;
    match error.code {
        ErrorCode::SourceMissingApiKey | ErrorCode::SourceUnauthorized => "CurseForge rejected the API key. Save a valid key in Settings, then try again.",
        ErrorCode::SourceRateLimited => "CurseForge is limiting requests. Wait a moment, then try again.",
        ErrorCode::SourceNetwork => "Could not reach CurseForge. Check your connection and try again.",
        ErrorCode::SourceProviderUnavailable => "CurseForge is unavailable. Try again later.",
        ErrorCode::SourceInvalidResponse => "CurseForge returned unexpected project data. Check the saved source with Find source, then try again.",
        ErrorCode::SourceNoUsableEvidence => "Use Find source to attach a CurseForge project before checking updates.",
        _ => "Could not read this mod's local files. Refresh the library and check again.",
    }.into()
}

struct CachedTransport {
    inner: ts4_mod_manager_core::curseforge_client::UreqCurseForgeTransport,
    responses: std::cell::RefCell<std::collections::HashMap<String, (u16, String)>>,
}
impl ts4_mod_manager_core::curseforge_client::CurseForgeTransport for CachedTransport {
    fn get(&self, request: &ts4_mod_manager_core::curseforge_client::CurseForgeRequest)
        -> Result<(u16, String), ts4_mod_manager_core::curseforge_client::SourceLookupError> {
        if let Some(response) = self.responses.borrow().get(&request.url) { return Ok(response.clone()); }
        let response = self.inner.get(request)?;
        if response.0 == 200 { self.responses.borrow_mut().insert(request.url.clone(), response.clone()); }
        Ok(response)
    }
}

#[cfg(test)]
mod lookup_worker_tests {
    use super::*;
    use std::{sync::{Arc, Mutex}, time::Duration};

    #[test]
    fn stalled_lookup_does_not_block_local_jobs_or_shutdown() {
        verify_read_only_queue(Action::Lookup(None));
    }

    #[test]
    fn stalled_update_check_does_not_block_local_jobs_or_shutdown() {
        verify_read_only_queue(Action::CheckUpdates { mod_ids: vec!["fixture".into()], api_key: None, epoch: Arc::default(), token: 0 });
    }

    fn verify_read_only_queue(action: Action) {
        let (entered, started) = mpsc::channel();
        let (release, stalled) = mpsc::channel();
        let stalled = Arc::new(Mutex::new(stalled));
        let worker = Mutations::start_executor(None, || {}, move |job| {
            if job.action.read_only() {
                entered.send(()).unwrap();
                stalled.lock().unwrap().recv().unwrap();
            }
            Ok(Output::Changed(vec![]))
        }).unwrap();
        let make_job = |operation, action| Job {
            identity: Identity {operation, root: "fixture".into(), entry: None, generation: 1},
            managed: "managed".into(), trash: "trash".into(), action,
        };
        worker.submit(make_job(1, action)).unwrap();
        started.recv_timeout(Duration::from_secs(1)).unwrap();
        worker.submit(make_job(2, Action::ListTrash)).unwrap();
        let deadline = std::time::Instant::now() + Duration::from_secs(1);
        loop {
            if worker.drain().any(|event| matches!(event, Event::Finished(id, Ok(_)) if id.operation == 2)) { break; }
            assert!(std::time::Instant::now() < deadline, "local job blocked by source lookup");
            std::thread::yield_now();
        }
        let (closed, done) = mpsc::channel();
        std::thread::spawn(move || {drop(worker); closed.send(()).unwrap();});
        let result = done.recv_timeout(Duration::from_secs(1));
        release.send(()).unwrap();
        result.expect("shutdown must not wait for a read-only lookup");
    }
}

#[cfg(test)]
mod update_error_tests {
    #[test]
    fn provider_errors_explain_the_recovery_without_showing_keys() {
        use ts4_mod_manager_core::error::{ErrorCode, ManagerError};
        let message = super::update_error(ManagerError::new(ErrorCode::SourceUnauthorized, "sensitive provider payload"));
        assert!(message.contains("Settings"));
        assert!(!message.contains("sensitive"));
        assert!(super::update_error(ManagerError::new(ErrorCode::SourceNoUsableEvidence, "missing")).contains("Find source"));
    }
}
