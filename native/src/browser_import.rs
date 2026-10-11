use egui::Context;
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    thread::{self, Thread},
    time::{Duration, Instant},
};
use ts4_mod_manager_core::{
    browser_import::{
        BrowserProject, BrowserRelease, DownloadWatch, ImportPermit, MatchedDownload,
        load_browser_project,
    },
    curseforge_client::{CurseForgeClient, UreqCurseForgeTransport},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Idle,
    Loading,
    Review,
    Waiting,
    Queued,
    Installing,
    Complete,
    Failed,
    Cancelled,
}

#[derive(Clone)]
pub struct ReadyImport {
    pub root: PathBuf,
    pub project: BrowserProject,
    pub release: BrowserRelease,
    pub matched: MatchedDownload,
    pub permit: ImportPermit,
    pub token: u64,
}

#[derive(Clone, PartialEq, Eq)]
struct Scope {
    root: PathBuf,
    key: [u8; 32],
}
impl Scope {
    fn new(root: PathBuf, saved_key: &str) -> Self {
        Self {
            root,
            key: Sha256::digest(saved_key.trim().as_bytes()).into(),
        }
    }
}

#[derive(Clone)]
struct Approved {
    root: PathBuf,
    project: BrowserProject,
    release: BrowserRelease,
    download_dir: PathBuf,
}

enum Event {
    Project(u64, Result<BrowserProject, String>),
    Download(u64, Result<MatchedDownload, String>),
}

pub struct BrowserImport {
    pub phase: Phase,
    pub project: Option<BrowserProject>,
    pub chosen: Option<u64>,
    pub download_dir: PathBuf,
    pub error: Option<String>,
    token: u64,
    scope: Option<Scope>,
    scope_valid: bool,
    approved: Option<Approved>,
    sender: Sender<Event>,
    receiver: Receiver<Event>,
    permit: Option<ImportPermit>,
    wake: Option<Thread>,
}

impl BrowserImport {
    pub fn new(home: &Path) -> Self {
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|path| path.is_absolute())
            .unwrap_or_else(|| home.join(".config"));
        let (sender, receiver) = mpsc::channel();
        Self {
            phase: Phase::Idle,
            project: None,
            chosen: None,
            download_dir: downloads_directory(home, &config.join("user-dirs.dirs")),
            error: None,
            token: 0,
            scope: None,
            scope_valid: false,
            approved: None,
            sender,
            receiver,
            permit: None,
            wake: None,
        }
    }

    pub fn prepare(&mut self, ctx: &Context, root: PathBuf, saved_key: &str, project_id: u64) {
        if !self.cancel() {
            return;
        }
        let token = self.begin_scope(root, saved_key);
        let key = saved_key.trim().to_owned();
        let sender = self.sender.clone();
        let ctx = ctx.clone();
        if let Err(error) = thread::Builder::new()
            .name("browser-import-releases".into())
            .spawn(move || {
                let result = CurseForgeClient::new(
                    Some(&key),
                    UreqCurseForgeTransport::with_budget(Duration::from_secs(20)),
                )
                .map_err(|_| "Could not load releases. Check the saved CurseForge key.".to_owned())
                .and_then(|client| {
                    load_browser_project(&client, project_id).map_err(|_| {
                        "Could not load verified releases from CurseForge. Try again later."
                            .to_owned()
                    })
                });
                let _ = sender.send(Event::Project(token, result));
                ctx.request_repaint();
            })
        {
            self.fail(format!("Could not start the release lookup: {error}"));
        }
    }

    pub fn prepare_loaded(&mut self, root: PathBuf, saved_key: &str, project: BrowserProject) {
        if !self.cancel() {
            return;
        }
        self.begin_scope(root, saved_key);
        self.loaded_project(project);
    }

    fn loaded_project(&mut self, project: BrowserProject) {
        self.chosen = project.releases.first().map(|file| file.release.file_id);
        self.project = Some(project);
        self.phase = Phase::Review;
        self.error = None;
    }

    fn begin_scope(&mut self, root: PathBuf, saved_key: &str) -> u64 {
        self.token = self.token.wrapping_add(1);
        self.scope = Some(Scope::new(root, saved_key));
        self.scope_valid = true;
        self.approved = None;
        self.permit = None;
        self.project = None;
        self.chosen = None;
        self.error = None;
        self.phase = Phase::Loading;
        self.token
    }

    pub fn sync(&mut self, root: Option<&PathBuf>, saved_key: Option<&str>) {
        let valid = match (&self.scope, root, saved_key) {
            (Some(scope), Some(root), Some(key)) => scope == &Scope::new(root.clone(), key),
            (None, _, _) => true,
            _ => false,
        };
        if !valid {
            self.scope_valid = false;
            if matches!(self.phase, Phase::Loading | Phase::Review) || self.running() {
                self.cancel();
            }
        }
        if self.running()
            && self
                .permit
                .as_ref()
                .is_some_and(ImportPermit::is_publishing)
        {
            self.phase = Phase::Installing;
        }
    }

    pub fn poll(&mut self) -> Option<ReadyImport> {
        if self.running()
            && self
                .permit
                .as_ref()
                .is_some_and(ImportPermit::is_publishing)
        {
            self.phase = Phase::Installing;
        }
        while let Ok(event) = self.receiver.try_recv() {
            match event {
                Event::Project(token, result)
                    if token == self.token && self.phase == Phase::Loading && self.scope_valid =>
                {
                    match result {
                        Ok(project) => self.loaded_project(project),
                        Err(error) => self.fail(error),
                    }
                }
                Event::Download(token, result)
                    if token == self.token && self.phase == Phase::Waiting && self.scope_valid =>
                {
                    if self.permit.as_ref().is_none_or(ImportPermit::is_cancelled) {
                        self.phase = Phase::Cancelled;
                        continue;
                    }
                    match result {
                        Ok(matched) => {
                            let approved = self.approved.as_ref()?;
                            let permit = self.permit.as_ref()?.clone();
                            self.phase = Phase::Queued;
                            return Some(ReadyImport {
                                root: approved.root.clone(),
                                project: approved.project.clone(),
                                release: approved.release.clone(),
                                matched,
                                permit,
                                token,
                            });
                        }
                        Err(error) => self.fail(error),
                    }
                }
                _ => {}
            }
        }
        None
    }

    pub fn start(&mut self, ctx: &Context) -> Result<String, String> {
        if self.phase != Phase::Review || !self.scope_valid {
            return Err("Review and confirm a release before starting a browser download.".into());
        }
        let project = self.project.as_ref().ok_or("No project selected.")?;
        let chosen = self.chosen.ok_or("Choose a release first.")?;
        let release = project
            .releases
            .iter()
            .find(|file| file.release.file_id == chosen)
            .ok_or("The chosen release is unavailable.")?;
        let approved = Approved {
            root: self
                .scope
                .as_ref()
                .ok_or("No game folder selected.")?
                .root
                .clone(),
            project: project.clone(),
            release: release.clone(),
            download_dir: self.download_dir.clone(),
        };
        self.watch(ctx, approved)
    }

    fn watch(&mut self, ctx: &Context, approved: Approved) -> Result<String, String> {
        if !approved.download_dir.is_absolute() {
            return Err("Choose an absolute Downloads folder.".into());
        }
        let mut watch = DownloadWatch::new(&approved.download_dir, &approved.release)
            .map_err(|_| "Choose an existing real Downloads folder without symlinks.".to_owned())?;
        let navigation = approved.release.navigation_url(&approved.project);
        let permit = ImportPermit::default();
        self.token = self.token.wrapping_add(1);
        let token = self.token;
        let worker_permit = permit.clone();
        let sender = self.sender.clone();
        let ctx = ctx.clone();
        let handle = thread::Builder::new()
            .name("browser-download-watch".into())
            .spawn(move || {
                let deadline = Instant::now() + Duration::from_secs(10 * 60);
                while !worker_permit.is_cancelled() {
                    let result = match watch.poll(&worker_permit) {
                        Ok(Some(matched)) => Some(Ok(matched)),
                        Ok(None) if Instant::now() >= deadline => Some(Err(
                            "No matching completed download found in ten minutes. Retry or choose the correct Downloads folder.".into(),
                        )),
                        Ok(None) => None,
                        Err(_) if worker_permit.is_cancelled() => break,
                        Err(_) => Some(Err("Could not read the Downloads folder. Check its permissions and retry.".into())),
                    };
                    if let Some(result) = result {
                        let _ = sender.send(Event::Download(token, result));
                        ctx.request_repaint();
                        break;
                    }
                    thread::park_timeout(Duration::from_millis(250));
                }
            })
            .map_err(|error| format!("Could not start the download watch: {error}"))?;
        self.wake = Some(handle.thread().clone());
        self.project = Some(approved.project.clone());
        self.chosen = Some(approved.release.release.file_id);
        self.download_dir = approved.download_dir.clone();
        self.approved = Some(approved);
        self.permit = Some(permit);
        self.error = None;
        self.phase = Phase::Waiting;
        Ok(navigation)
    }

    pub fn token(&self) -> u64 {
        self.token
    }

    pub fn is_review_open(&self) -> bool {
        matches!(self.phase, Phase::Loading | Phase::Review)
    }

    pub fn cancel(&mut self) -> bool {
        if self.permit.as_ref().is_some_and(|permit| !permit.cancel()) && self.running() {
            self.phase = Phase::Installing;
            return false;
        }
        if let Some(wake) = self.wake.take() {
            wake.unpark();
        }
        if matches!(self.phase, Phase::Loading | Phase::Review) || self.running() {
            self.token = self.token.wrapping_add(1);
            self.phase = Phase::Cancelled;
            self.error = None;
        }
        true
    }

    pub fn running(&self) -> bool {
        matches!(
            self.phase,
            Phase::Waiting | Phase::Queued | Phase::Installing
        )
    }

    pub fn finish(&mut self, token: u64, result: Result<(), String>) {
        if token != self.token || !self.running() {
            return;
        }
        if let Some(wake) = self.wake.take() {
            wake.unpark();
        }
        match result {
            Ok(()) => {
                self.phase = Phase::Complete;
                self.error = None;
            }
            Err(error) => self.fail(error),
        }
    }

    pub fn can_retry(&self) -> bool {
        matches!(self.phase, Phase::Failed | Phase::Cancelled) && self.scope_valid && self.approved.is_some()
    }
    pub fn retry(&mut self, ctx: &Context) -> Result<String, String> {
        if !self.can_retry() {
            return Err("This approval is no longer active. Review the release again.".into());
        }
        let approved = self
            .approved
            .clone()
            .ok_or("Review and confirm a release first.")?;
        self.watch(ctx, approved)
    }

    fn fail(&mut self, error: String) {
        if let Some(permit) = &self.permit {
            permit.cancel();
        }
        if let Some(wake) = self.wake.take() {
            wake.unpark();
        }
        self.phase = Phase::Failed;
        self.error = Some(error);
    }
}

impl Drop for BrowserImport {
    fn drop(&mut self) {
        if let Some(permit) = &self.permit {
            permit.cancel();
        }
        if let Some(wake) = self.wake.take() {
            wake.unpark();
        }
    }
}

fn downloads_directory(home: &Path, config_file: &Path) -> PathBuf {
    let parsed = std::fs::read_to_string(config_file)
        .ok()
        .and_then(|config| {
            config.lines().find_map(|line| {
                let (name, value) = line.trim().split_once('=')?;
                if name.trim() != "XDG_DOWNLOAD_DIR" {
                    return None;
                }
                parse_download_directory(home, value.trim())
            })
        });
    parsed.unwrap_or_else(|| home.join("Downloads"))
}

fn parse_download_directory(home: &Path, value: &str) -> Option<PathBuf> {
    let value = value.strip_prefix('"')?.strip_suffix('"')?;
    let mut literal = String::new();
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        match ch {
            '\\' => {
                let escaped = chars.next()?;
                if !matches!(escaped, '\\' | '"' | '$') {
                    return None;
                }
                literal.push(escaped);
            }
            '`' | '"' | '\n' | '\r' => return None,
            _ => literal.push(ch),
        }
    }
    let path = if let Some(relative) = literal
        .strip_prefix("$HOME/")
        .or_else(|| literal.strip_prefix("${HOME}/"))
    {
        if relative.contains('$') {
            return None;
        }
        home.join(relative.trim_start_matches('/'))
    } else if literal == "$HOME" || literal == "${HOME}" {
        home.to_path_buf()
    } else {
        if literal.contains('$') {
            return None;
        }
        PathBuf::from(literal)
    };
    path.is_absolute().then_some(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use ts4_mod_manager_core::curseforge_client::{
        CurseForgeRequest, CurseForgeTransport, SourceLookupError,
    };

    struct Provider;
    impl CurseForgeTransport for Provider {
        fn get(&self, request: &CurseForgeRequest) -> Result<(u16, String), SourceLookupError> {
            let response = if request.url.ends_with("/42") {
                json!({"data": {"id":42,"gameId":78062,"name":"Fixture","authors":[{"name":"Author"}],"links":{"websiteUrl":"https://www.curseforge.com/sims4/mods/fixture"}}})
            } else {
                json!({"data":[{"id":100,"modId":42,"fileName":"mod.package","displayName":"Version 1","fileDate":"2026-10-01T00:00:00Z","releaseType":1,"isAvailable":true,"fileStatus":4,"gameVersions":[],"fileLength":7,"hashes":[{"algo":1,"value":"582681c2eae02b3f3d399c0c26d321560f6c567a"}]}],"pagination":{"index":0,"pageSize":50,"resultCount":1,"totalCount":1}})
            };
            Ok((200, response.to_string()))
        }
    }

    fn review(home: &Path) -> BrowserImport {
        let mut import = BrowserImport::new(home);
        import.download_dir = home.join("Downloads");
        std::fs::create_dir_all(&import.download_dir).unwrap();
        let token = import.begin_scope(home.join("game"), "fixture");
        let project = load_browser_project(
            &CurseForgeClient::new(Some("fixture"), Provider).unwrap(),
            42,
        )
        .unwrap();
        import
            .sender
            .send(Event::Project(token, Ok(project)))
            .unwrap();
        assert!(import.poll().is_none());
        assert_eq!(import.phase, Phase::Review);
        import
    }

    fn ready(import: &mut BrowserImport) -> ReadyImport {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            if let Some(ready) = import.poll() {
                return ready;
            }
            assert!(Instant::now() < deadline, "watch did not find the fixture");
            thread::sleep(Duration::from_millis(10));
        }
    }

    #[test]
    fn literal_xdg_downloads_configuration_never_executes_shell_syntax() {
        let home = Path::new("/home/fixture");
        assert_eq!(
            parse_download_directory(home, "\"$HOME/Downloads with spaces\""),
            Some(home.join("Downloads with spaces"))
        );
        assert_eq!(
            parse_download_directory(home, "\"${HOME}/Baixados\""),
            Some(home.join("Baixados"))
        );
        assert_eq!(
            parse_download_directory(home, "\"/mnt/downloads\""),
            Some(PathBuf::from("/mnt/downloads"))
        );
        for invalid in [
            "\"$(touch /tmp/should-not-exist)\"",
            "\"`touch /tmp/should-not-exist`\"",
            "\"$OTHER/downloads\"",
            "\"relative/folder\"",
        ] {
            assert!(parse_download_directory(home, invalid).is_none());
        }
        let temp = tempfile::tempdir().unwrap();
        let config = temp.path().join("user-dirs.dirs");
        std::fs::write(&config, "# Comment\nXDG_DOWNLOAD_DIR=\"$HOME/Baixados\"\n").unwrap();
        assert_eq!(downloads_directory(home, &config), home.join("Baixados"));
        std::fs::write(&config, "XDG_DOWNLOAD_DIR=\"$(false)\"\n").unwrap();
        assert_eq!(downloads_directory(home, &config), home.join("Downloads"));
    }

    #[test]
    fn scope_changes_cancel_but_unrelated_selection_and_key_drafts_do_not() {
        let home = tempfile::tempdir().unwrap();
        let mut import = review(home.path());
        let game = home.path().join("game");
        import.sync(Some(&game), Some("fixture"));
        assert_eq!(import.phase, Phase::Review);
        let previous = import.token();
        import.sync(Some(&game), Some("new-saved-key"));
        assert_eq!(import.phase, Phase::Cancelled);
        assert_ne!(previous, import.token());
        assert!(import.retry(&Context::default()).is_err());
        let mut import = review(home.path());
        import.sync(Some(&home.path().join("other-game")), Some("fixture"));
        assert_eq!(import.phase, Phase::Cancelled);
    }

    #[test]
    fn cached_project_replaces_old_work_without_authorizing_an_import() {
        let home = tempfile::tempdir().unwrap();
        let mut import = review(home.path());
        std::fs::write(import.download_dir.join("mod.package"), b"package").unwrap();
        import.start(&Context::default()).unwrap();
        let old = ready(&mut import);
        let mut cached = old.project.clone();
        cached.title = "Cached replacement".into();
        import.prepare_loaded(home.path().join("other-game"), "fixture", cached);
        assert!(old.permit.is_cancelled());
        assert_eq!(import.phase, Phase::Review);
        assert_eq!(import.chosen, Some(100));
        assert_ne!(import.token(), old.token);
        import
            .sender
            .send(Event::Project(old.token, Ok(old.project)))
            .unwrap();
        assert!(import.poll().is_none());
        assert_eq!(import.project.as_ref().unwrap().title, "Cached replacement");
        assert_eq!(import.phase, Phase::Review);
        assert!(!import.running());
        assert!(import.permit.is_none());
        assert!(import.retry(&Context::default()).is_err());
        assert_eq!(
            std::fs::read(import.download_dir.join("mod.package")).unwrap(),
            b"package"
        );
    }

    #[test]
    fn stale_provider_reply_cannot_restore_a_cancelled_review() {
        let home = tempfile::tempdir().unwrap();
        let mut import = review(home.path());
        let token = import.token();
        let project = import.project.clone().unwrap();
        import.cancel();
        import
            .sender
            .send(Event::Project(token, Ok(project)))
            .unwrap();
        import.poll();
        assert_eq!(import.phase, Phase::Cancelled);
        assert!(!import.running());
    }

    #[test]
    fn completed_matching_download_queues_once_and_preserves_original() {
        let home = tempfile::tempdir().unwrap();
        let mut import = review(home.path());
        let original = import.download_dir.join("renamed (1).package");
        std::fs::write(&original, b"package").unwrap();
        let navigation = import.start(&Context::default()).unwrap();
        assert_eq!(
            navigation,
            "https://www.curseforge.com/sims4/mods/fixture/files/100"
        );
        let ready = ready(&mut import);
        assert_eq!(ready.root, home.path().join("game"));
        assert_eq!(ready.release.release.file_id, 100);
        assert_eq!(
            ready.project.source_url,
            "https://www.curseforge.com/sims4/mods/fixture"
        );
        assert_eq!(import.phase, Phase::Queued);
        assert!(import.poll().is_none());
        assert_eq!(std::fs::read(original).unwrap(), b"package");
        import.finish(ready.token.wrapping_sub(1), Ok(()));
        assert_eq!(import.phase, Phase::Queued);
        import.finish(ready.token, Ok(()));
        assert_eq!(import.phase, Phase::Complete);
    }

    #[test]
    fn retry_keeps_the_approved_destination_release_and_folder() {
        let home = tempfile::tempdir().unwrap();
        let mut import = review(home.path());
        std::fs::write(import.download_dir.join("mod.package"), b"package").unwrap();
        import.start(&Context::default()).unwrap();
        let first = ready(&mut import);
        import.finish(first.token, Err("fixture failure".into()));
        assert!(
            first.permit.is_cancelled(),
            "failed attempts must lose their publication authority"
        );
        import.chosen = Some(999);
        import.download_dir = home.path().join("unapproved-folder");
        import.retry(&Context::default()).unwrap();
        assert_eq!(import.chosen, Some(100));
        assert_eq!(import.download_dir, home.path().join("Downloads"));
        let next = ready(&mut import);
        assert_ne!(next.token, first.token);
        assert_eq!(next.root, first.root);
        assert_eq!(next.release.release.file_id, 100);
        assert!(!next.permit.is_cancelled());
    }

    #[test]
    fn root_changes_and_drop_revoke_queued_import_permissions() {
        let home = tempfile::tempdir().unwrap();
        let mut import = review(home.path());
        std::fs::write(import.download_dir.join("mod.package"), b"package").unwrap();
        import.start(&Context::default()).unwrap();
        let queued = ready(&mut import);
        import.sync(Some(&home.path().join("other-game")), Some("fixture"));
        assert!(queued.permit.is_cancelled());
        assert_eq!(import.phase, Phase::Cancelled);
        import.finish(queued.token, Ok(()));
        assert_eq!(import.phase, Phase::Cancelled);
        let mut import = review(home.path());
        import.start(&Context::default()).unwrap();
        let queued = ready(&mut import);
        drop(import);
        assert!(queued.permit.is_cancelled());
    }

    #[test]
    fn publication_finishes_in_the_approved_game_after_scope_changes() {
        let home = tempfile::tempdir().unwrap();
        let mut import = review(home.path());
        std::fs::write(import.download_dir.join("mod.package"), b"package").unwrap();
        import.start(&Context::default()).unwrap();
        let queued = ready(&mut import);
        let game_mods = queued.root.join("Mods");
        std::fs::create_dir_all(&game_mods).unwrap();
        ts4_mod_manager_core::browser_import::import_browser_download(
            &home.path().join("managed"),
            &game_mods,
            &queued.project,
            &queued.release,
            &queued.matched,
            &queued.permit,
        )
        .unwrap();
        assert!(queued.permit.is_publishing());
        import.sync(
            Some(&home.path().join("other-game")),
            Some("changed-saved-key"),
        );
        assert_eq!(import.phase, Phase::Installing);
        assert!(!import.cancel());
        let current_token = import.token();
        let mut cached = queued.project.clone();
        cached.title = "Must not replace publishing work".into();
        import.prepare_loaded(home.path().join("other-game"), "fixture", cached);
        assert_eq!(import.token(), current_token);
        assert_eq!(import.phase, Phase::Installing);
        assert_eq!(import.project.as_ref().unwrap().title, "Fixture");
        assert_eq!(
            std::fs::read(game_mods.join("mod.package")).unwrap(),
            b"package"
        );
        assert!(import.retry(&Context::default()).is_err());
        import.finish(queued.token, Ok(()));
        assert_eq!(import.phase, Phase::Complete);
    }

    #[test]
    fn late_download_event_is_ignored_after_cancellation() {
        let home = tempfile::tempdir().unwrap();
        let mut import = review(home.path());
        std::fs::write(import.download_dir.join("mod.package"), b"package").unwrap();
        import.start(&Context::default()).unwrap();
        let event = import
            .receiver
            .recv_timeout(Duration::from_secs(3))
            .unwrap();
        import.cancel();
        import.sender.send(event).unwrap();
        assert!(import.poll().is_none());
        assert_eq!(import.phase, Phase::Cancelled);
    }

    #[test]
    fn cancellation_before_confirmation_never_starts_a_watch() {
        let home = tempfile::tempdir().unwrap();
        let mut import = review(home.path());
        assert!(import.cancel());
        assert!(import.start(&Context::default()).is_err());
        assert!(import.retry(&Context::default()).is_err());
        assert!(import.poll().is_none());
    }

    #[test]
    fn symlink_download_directory_is_rejected_before_a_watch_starts() {
        let home = tempfile::tempdir().unwrap();
        let mut import = review(home.path());
        let linked = home.path().join("linked");
        std::os::unix::fs::symlink(&import.download_dir, &linked).unwrap();
        import.download_dir = linked;
        assert!(import.start(&Context::default()).is_err());
        assert_eq!(import.phase, Phase::Review);
    }
}
