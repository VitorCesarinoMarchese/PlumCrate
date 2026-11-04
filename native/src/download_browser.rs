//! A browser process and profile owned by one approved download session.

use std::{
    path::Path,
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
};

#[derive(Clone, Default)]
pub struct DownloadBrowser(Arc<Mutex<Session>>);
#[derive(Default)]
struct Session {
    closed: bool,
    launched: bool,
    process: Option<(Child, tempfile::TempDir)>,
}
impl Session {
    fn close(&mut self) {
        self.closed = true;
        if let Some((mut child, profile)) = self.process.take() {
            // This PID came from our own spawn with a unique profile. Never signal by name.
            let _ = child.kill();
            let _ = child.wait();
            drop(profile);
        }
    }
}
impl Drop for Session {
    fn drop(&mut self) { self.close(); }
}
impl DownloadBrowser {
    pub fn close(&self) {
        if let Ok(mut state) = self.0.lock() { state.close(); }
    }
    fn launch_with(&self, launch: impl FnOnce(&Path) -> Result<Child, String>) -> Result<(), String> {
        {
            let mut state = self.0.lock().map_err(|_| "Download window unavailable")?;
            if state.closed || state.launched { return Ok(()); }
            state.launched = true;
        }
        // Browser discovery and startup may stall. Never hold the lifecycle lock while launching.
        let profile = tempfile::Builder::new().prefix("plumcrate-browser-").tempdir().map_err(|e| e.to_string())?;
        let mut child = launch(profile.path())?;
        let mut state = self.0.lock().map_err(|_| "Download window unavailable")?;
        if state.closed {
            let _ = child.kill();
            let _ = child.wait();
        } else {
            state.process = Some((child, profile));
        }
        Ok(())
    }
    #[cfg(test)]
    pub(crate) fn is_closed(&self) -> bool { self.0.lock().unwrap().closed }
    pub fn open(&self, url: &str, downloads: &Path) -> Result<(), String> {
        self.launch_with(|profile| {
            configure_profile(profile, downloads)?;
            let default = Command::new("xdg-settings").args(["get", "default-web-browser"])
                .output().ok().map(|output| String::from_utf8_lossy(&output.stdout).to_lowercase()).unwrap_or_default();
            let firefox_first = default.contains("firefox");
            let mut browsers = vec!["chromium", "chromium-browser", "google-chrome", "google-chrome-stable", "firefox"];
            if firefox_first { browsers.rotate_right(1); }
            for browser in browsers {
                match Command::new(browser).args(browser_args(browser, profile, url))
                    .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn() {
                    Ok(mut child) => {
                        std::thread::sleep(std::time::Duration::from_millis(300));
                        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
                            return Err(format!("The separate download browser exited ({status}). Install a native Chromium or Firefox browser and retry."));
                        }
                        return Ok(child);
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                    Err(error) => return Err(format!("Could not launch {browser}: {error}")),
                }
            }
            Err("Automatic window closing requires a native Chromium or Firefox browser. Install one and retry.".into())
        })
    }
}
fn configure_profile(profile: &Path, downloads: &Path) -> Result<(), String> {
    let write = || -> std::io::Result<()> {
        std::fs::create_dir_all(profile.join("Default"))?;
        std::fs::write(profile.join("Default/Preferences"), serde_json::json!({
            "download": {"default_directory": downloads, "prompt_for_download": false,
                "directory_upgrade": true}, "browser": {"check_default_browser": false}
        }).to_string())?;
        let directory = serde_json::to_string(downloads)?;
        std::fs::write(profile.join("user.js"), format!(
            "user_pref(\"browser.download.folderList\", 2);\nuser_pref(\"browser.download.dir\", {directory});\nuser_pref(\"browser.download.useDownloadDir\", true);\nuser_pref(\"browser.shell.checkDefaultBrowser\", false);\nuser_pref(\"browser.aboutwelcome.enabled\", false);\n"
        ))
    };
    write().map_err(|error| format!("Could not configure download browser: {error}"))
}
fn browser_args(browser: &str, profile: &Path, url: &str) -> Vec<String> {
    if browser == "firefox" {
        vec!["--no-remote".into(), "--profile".into(), profile.to_string_lossy().into_owned(), "--new-window".into(), url.into()]
    } else {
        vec![format!("--user-data-dir={}", profile.display()), "--no-first-run".into(),
            "--no-default-browser-check".into(), "--new-window".into(), format!("--app={url}")]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn closing_a_session_reaps_only_its_own_process() {
        let session = DownloadBrowser::default();
        let unrelated = Command::new("sleep").arg("30").spawn().unwrap();
        let mut unrelated = unrelated;
        let mut pid = 0;
        session.launch_with(|profile| {
            assert!(profile.is_dir());
            let child = Command::new("sleep").arg("30").spawn().unwrap();
            pid = child.id();
            Ok(child)
        }).unwrap();
        assert!(std::path::Path::new(&format!("/proc/{pid}")).exists());
        session.close();
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
        assert!(unrelated.try_wait().unwrap().is_none());
        unrelated.kill().unwrap();
        unrelated.wait().unwrap();
        session.close();
    }

    #[test]
    fn completion_before_browser_launch_never_opens_a_late_window() {
        let session = DownloadBrowser::default();
        session.close();
        session.launch_with(|_| panic!("closed session launched a browser")).unwrap();
    }

    #[test]
    fn completion_during_launch_closes_the_late_process() {
        let session = DownloadBrowser::default();
        let mut pid = 0;
        session.launch_with(|_| {
            session.close();
            let child = Command::new("sleep").arg("30").spawn().unwrap();
            pid = child.id();
            Ok(child)
        }).unwrap();
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    }

    #[test]
    #[ignore = "launches a real browser window with an offline blank page"]
    fn real_browser_window_is_owned_and_reaped() {
        let session = DownloadBrowser::default();
        let downloads = tempfile::tempdir().unwrap();
        session.open("about:blank", downloads.path()).unwrap();
        let pid = session.0.lock().unwrap().process.as_ref().unwrap().0.id();
        assert!(std::path::Path::new(&format!("/proc/{pid}")).exists());
        session.close();
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    }

    #[test]
    fn isolated_profiles_configure_the_approved_download_folder() {
        let profile = tempfile::tempdir().unwrap();
        let downloads = profile.path().join("Downloads with spaces");
        configure_profile(profile.path(), &downloads).unwrap();
        let prefs: serde_json::Value = serde_json::from_slice(&std::fs::read(profile.path().join("Default/Preferences")).unwrap()).unwrap();
        assert_eq!(prefs["download"]["default_directory"], downloads.to_str().unwrap());
        assert_eq!(prefs["download"]["prompt_for_download"], false);
        let firefox = std::fs::read_to_string(profile.path().join("user.js")).unwrap();
        assert!(firefox.contains(&serde_json::to_string(&downloads).unwrap()));
        let args = browser_args("chromium", profile.path(), "https://example.com/download");
        assert!(args.iter().any(|arg| arg.starts_with("--user-data-dir=")));
        assert!(args.iter().any(|arg| arg == "--app=https://example.com/download"));
        let args = browser_args("firefox", profile.path(), "https://example.com/download");
        assert_eq!(args[0], "--no-remote");
        assert!(args.iter().any(|arg| arg == "--profile"));
    }
}
