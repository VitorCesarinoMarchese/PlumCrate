//! AppImage self-updates. Mod updates remain in `updates` and the core.
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

mod ui;
pub use ui::AppUpdates;

const REPOSITORY: &str = "VitorCesarinoMarchese/TS4ModManager";
const MAX_IMAGE_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize)]
struct Asset {
    name: String,
    size: u64,
    browser_download_url: String,
    digest: Option<String>,
}

#[derive(Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}

#[derive(Clone, Debug)]
struct Release {
    version: String,
    notes: String,
    asset: Asset,
    sha256: Option<String>,
    checksum: Option<Asset>,
}

fn select_release(raw: &str, current: &str, arch: &str) -> Result<Option<Release>, String> {
    let current = Version::parse(current).map_err(|_| "Invalid app version")?;
    let releases: Vec<GithubRelease> =
        serde_json::from_str(raw).map_err(|_| "Invalid GitHub release response")?;
    let mut newer = releases
        .into_iter()
        .filter_map(|release| {
            let version = Version::parse(release.tag_name.trim_start_matches('v')).ok()?;
            (!release.draft
                && version.cmp_precedence(&current).is_gt()
                && version.build.is_empty()
                && (!current.pre.is_empty() || (!release.prerelease && version.pre.is_empty())))
            .then_some((version, release))
        })
        .collect::<Vec<_>>();
    newer.sort_by(|a, b| b.0.cmp_precedence(&a.0));
    for (version, release) in newer {
        let name = format!("ts4-mod-manager-native-{version}-{arch}.AppImage");
        let Some(asset) = release
            .assets
            .iter()
            .find(|asset| asset.name == name)
            .cloned()
        else {
            continue;
        };
        let base = format!("https://github.com/{REPOSITORY}/releases");
        let validate_asset = |asset: &Asset| {
            asset.browser_download_url
                == format!("{base}/download/{}/{}", release.tag_name, asset.name)
        };
        if !validate_asset(&asset) || release.html_url != format!("{base}/tag/{}", release.tag_name)
        {
            return Err("The release contains an untrusted download or release URL".into());
        }
        if !(20..=MAX_IMAGE_BYTES).contains(&asset.size) {
            return Err("The release AppImage has an invalid size".into());
        }
        let sha256 = match asset.digest.as_deref() {
            Some(digest) => Some(valid_digest(digest.strip_prefix("sha256:").unwrap_or(""))?),
            None => None,
        };
        let checksum = release
            .assets
            .iter()
            .find(|item| item.name == format!("{name}.sha256"))
            .cloned();
        if sha256.is_none()
            && !checksum
                .as_ref()
                .is_some_and(|item| validate_asset(item) && item.size <= 4096)
        {
            return Err(
                "This release has no SHA-256 checksum. Download it manually from GitHub.".into(),
            );
        }
        return Ok(Some(Release {
            version: version.to_string(),
            notes: release.html_url,
            asset,
            sha256,
            checksum,
        }));
    }
    Ok(None)
}

fn valid_digest(raw: &str) -> Result<String, String> {
    if raw.len() == 64 && raw.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(raw.to_ascii_lowercase())
    } else {
        Err("The release SHA-256 checksum is invalid".into())
    }
}

fn parse_checksum(raw: &str, name: &str) -> Result<String, String> {
    let (digest, filename) = raw
        .trim_end_matches(['\r', '\n'])
        .split_once("  ")
        .ok_or("Invalid release checksum file")?;
    if filename != name {
        return Err("The checksum names a different AppImage".into());
    }
    valid_digest(digest)
}

fn allowed_host(url: &url::Url) -> bool {
    url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.port_or_known_default() == Some(443)
        && matches!(
            url.host_str(),
            Some(
                "api.github.com"
                    | "github.com"
                    | "release-assets.githubusercontent.com"
                    | "objects.githubusercontent.com"
            )
        )
}

fn get(raw_url: &str) -> Result<ureq::Response, String> {
    let agent = ureq::AgentBuilder::new()
        .redirects(0)
        .timeout(Duration::from_secs(60))
        .build();
    let mut url = url::Url::parse(raw_url).map_err(|_| "Invalid update URL")?;
    for _ in 0..6 {
        if !allowed_host(&url) {
            return Err("Refused an update redirect outside GitHub release hosts".into());
        }
        let response = agent
            .get(url.as_str())
            .set(
                "User-Agent",
                concat!("TS4ModManager/", env!("CARGO_PKG_VERSION")),
            )
            .set(
                "Accept",
                if url.host_str() == Some("api.github.com") {
                    "application/vnd.github+json"
                } else {
                    "application/octet-stream"
                },
            )
            .set("X-GitHub-Api-Version", "2022-11-28")
            .call()
            .map_err(|error| match error {
                ureq::Error::Status(403 | 429, _) => {
                    "GitHub's request limit was reached. Try again later.".to_string()
                }
                _ => {
                    "Could not contact GitHub for app updates. Check your connection and try again."
                        .to_string()
                }
            })?;
        if response.status() == 200 {
            return Ok(response);
        }
        if !matches!(response.status(), 301 | 302 | 303 | 307 | 308) {
            return Err("GitHub returned an unexpected response".into());
        }
        url = url
            .join(
                response
                    .header("Location")
                    .ok_or("Update redirect has no destination")?,
            )
            .map_err(|_| "Invalid update redirect")?;
    }
    Err("Too many update download redirects".into())
}

fn bounded_text(reader: impl Read, limit: u64) -> Result<String, String> {
    let mut bytes = Vec::new();
    reader
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "Could not read the release response")?;
    if bytes.len() as u64 > limit {
        return Err("The release response is too large".into());
    }
    String::from_utf8(bytes).map_err(|_| "Invalid release response encoding".into())
}

fn check() -> Result<Option<Release>, String> {
    let response = get(&format!(
        "https://api.github.com/repos/{REPOSITORY}/releases?per_page=100"
    ))?;
    select_release(
        &bounded_text(response.into_reader(), 2 * 1024 * 1024)?,
        env!("CARGO_PKG_VERSION"),
        std::env::consts::ARCH,
    )
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Stamp {
    device: u64,
    inode: u64,
    size: u64,
    modified: (i64, i64),
    mode: u32,
}
fn stamp(path: &Path) -> Result<Stamp, String> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "Cannot read the installed AppImage")?;
    if !metadata.is_file() {
        return Err("The AppImage must be a regular file, not a symlink".into());
    }
    Ok(Stamp {
        device: metadata.dev(),
        inode: metadata.ino(),
        size: metadata.len(),
        modified: (metadata.mtime(), metadata.mtime_nsec()),
        mode: metadata.mode(),
    })
}

#[derive(Clone, Debug)]
struct Target {
    path: PathBuf,
    original: Stamp,
}
impl Target {
    fn open(path: PathBuf) -> Result<Self, String> {
        if !path.is_absolute() {
            return Err("The AppImage path must be absolute".into());
        }
        let original = stamp(&path)?;
        verify_header(File::open(&path).map_err(|_| "Cannot read the AppImage")?)?;
        if original.mode & 0o111 == 0 || original.mode & 0o222 == 0 {
            return Err(
                "Move the AppImage to a writable folder and make it executable before updating"
                    .into(),
            );
        }
        Ok(Self {
            path: fs::canonicalize(path).map_err(|_| "Cannot resolve the AppImage path")?,
            original,
        })
    }
    fn detect() -> Result<Self, String> {
        if !cfg!(target_os = "linux") {
            return Err("In-app updates require a Linux AppImage".into());
        }
        let path = std::env::var_os("APPIMAGE")
            .ok_or("In-app updates require an AppImage. Update this installation manually.")?;
        let appdir = std::env::var_os("APPDIR")
            .ok_or("The AppImage runtime did not provide its app directory")?;
        let expected =
            fs::canonicalize(PathBuf::from(appdir).join("usr/bin/ts4-mod-manager-native"))
                .map_err(|_| "Cannot identify the running AppImage")?;
        if fs::canonicalize(std::env::current_exe().map_err(|_| "Cannot identify this executable")?)
            .map_err(|_| "Cannot resolve this executable")?
            != expected
        {
            return Err("The running app is outside the AppImage. Update it manually.".into());
        }
        Self::open(path.into())
    }
    fn unchanged(&self) -> Result<(), String> {
        if stamp(&self.path)? != self.original {
            return Err(
                "The installed AppImage changed. Close and reopen the app before updating.".into(),
            );
        }
        Ok(())
    }
}

fn verify_header(mut reader: impl Read) -> Result<(), String> {
    let mut header = [0; 20];
    reader
        .read_exact(&mut header)
        .map_err(|_| "The download is not an AppImage")?;
    let machine = match std::env::consts::ARCH {
        "x86_64" => 62,
        "aarch64" => 183,
        _ => return Err("Unsupported AppImage architecture".into()),
    };
    if &header[..4] != b"\x7fELF"
        || header[4..6] != [2, 1]
        || &header[8..11] != b"AI\x02"
        || u16::from_le_bytes([header[18], header[19]]) != machine
    {
        return Err("The download is not a type-2 AppImage for this computer".into());
    }
    Ok(())
}

#[derive(Debug)]
struct Staged {
    target: Target,
    payload: tempfile::TempPath,
    digest: String,
}
fn stage_download(
    target: Target,
    release: &Release,
    digest: &str,
    mut reader: impl Read,
    mut progress: impl FnMut(u64, u64),
) -> Result<Staged, String> {
    target.unchanged()?;
    let mut file = tempfile::Builder::new()
        .prefix(".ts4-download-")
        .suffix(".AppImage")
        .tempfile_in(target.path.parent().ok_or("AppImage path has no parent")?)
        .map_err(|_| "Cannot stage the update. Check folder permissions and free space.")?;
    let mut hash = Sha256::new();
    let mut received = 0;
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = reader
            .read(&mut buffer)
            .map_err(|_| "The update download was interrupted. Try again.")?;
        if count == 0 {
            break;
        }
        received += count as u64;
        if received > release.asset.size {
            return Err("The download exceeds its published size".into());
        }
        file.write_all(&buffer[..count])
            .map_err(|_| "Cannot write the update. Check free space.")?;
        hash.update(&buffer[..count]);
        progress(received, release.asset.size);
    }
    if received != release.asset.size || format!("{:x}", hash.finalize()) != digest {
        return Err(
            "The update failed SHA-256 or size verification. The installed app was left unchanged."
                .into(),
        );
    }
    verify_header(File::open(file.path()).map_err(|_| "Cannot verify the downloaded AppImage")?)?;
    file.as_file()
        .set_permissions(fs::Permissions::from_mode(target.original.mode & 0o777))
        .map_err(|_| "Cannot make the update executable")?;
    file.as_file()
        .sync_all()
        .map_err(|_| "Cannot sync the downloaded AppImage")?;
    Ok(Staged {
        target,
        payload: file.into_temp_path(),
        digest: digest.into(),
    })
}

fn download(
    target: Target,
    release: &Release,
    progress: impl FnMut(u64, u64),
) -> Result<Staged, String> {
    let digest = match &release.sha256 {
        Some(digest) => digest.clone(),
        None => {
            let checksum = release.checksum.as_ref().ok_or("Release has no checksum")?;
            parse_checksum(
                &bounded_text(get(&checksum.browser_download_url)?.into_reader(), 4096)?,
                &release.asset.name,
            )?
        }
    };
    let staged = stage_download(
        target,
        release,
        &digest,
        get(&release.asset.browser_download_url)?.into_reader(),
        progress,
    )?;
    probe_version(&staged.payload, &release.version)?;
    Ok(staged)
}

fn probe_version(path: &Path, version: &str) -> Result<(), String> {
    use std::process::{Command, Stdio};
    let mut child = Command::new(path)
        .args(["--appimage-extract-and-run", "--version"])
        .env_remove("APPIMAGE")
        .env_remove("APPDIR")
        .env_remove("OWD")
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| "The downloaded AppImage could not run on this system")?;
    let start = Instant::now();
    loop {
        if child
            .try_wait()
            .map_err(|_| "Cannot check the downloaded AppImage")?
            .is_some()
        {
            break;
        }
        if start.elapsed() > Duration::from_secs(30) {
            let _ = child.kill();
            let _ = child.wait();
            return Err("The downloaded AppImage's version check timed out".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let output = child
        .wait_with_output()
        .map_err(|_| "Cannot read the downloaded app version")?;
    if !output.status.success()
        || String::from_utf8_lossy(&output.stdout).trim()
            != format!("ts4-mod-manager-native {version}")
    {
        return Err("The downloaded AppImage reports the wrong version or cannot start".into());
    }
    Ok(())
}

#[derive(Debug)]
struct Installed {
    path: PathBuf,
    backup: PathBuf,
    replacement: Stamp,
    previous: Stamp,
}
impl Staged {
    fn install(self) -> Result<Installed, String> {
        self.target.unchanged()?;
        let replacement = stamp(&self.payload)?;
        let mut file = File::open(&self.payload).map_err(|_| "The staged update is missing")?;
        let mut hash = Sha256::new();
        std::io::copy(&mut file, &mut hash).map_err(|_| "Cannot verify the staged update")?;
        if format!("{:x}", hash.finalize()) != self.digest {
            return Err("The staged update changed. Download it again.".into());
        }
        let directory = tempfile::Builder::new()
            .prefix(".ts4-previous-")
            .permissions(fs::Permissions::from_mode(0o700))
            .tempdir_in(self.target.path.parent().unwrap())
            .map_err(|_| "Cannot create an AppImage backup folder")?;
        let backup = directory.path().join("previous.AppImage");
        fs::hard_link(&self.target.path, &backup)
            .map_err(|_| "Cannot preserve the previous AppImage. The update was cancelled.")?;
        File::open(directory.path())
            .and_then(|folder| folder.sync_all())
            .map_err(|_| "Cannot sync the AppImage backup; cancelled the update")?;
        self.target.unchanged()?;
        self.payload
            .persist(&self.target.path)
            .map_err(|_| "Cannot replace the AppImage. The previous app is still installed.")?;
        let _ = directory.keep();
        let installed = Installed {
            path: self.target.path,
            backup,
            replacement,
            previous: self.target.original,
        };
        if let Err(error) =
            File::open(installed.path.parent().unwrap()).and_then(|parent| parent.sync_all())
        {
            installed.rollback()?;
            return Err(format!(
                "Cannot sync the update; restored the previous AppImage: {error}"
            ));
        }
        Ok(installed)
    }
}
impl Installed {
    fn rollback(&self) -> Result<(), String> {
        if stamp(&self.path)? != self.replacement || stamp(&self.backup)? != self.previous {
            return Err(format!(
                "An AppImage file changed during restart. Your files were preserved; the backup is {}",
                self.backup.display()
            ));
        }
        fs::rename(&self.backup, &self.path).map_err(|_| {
            format!(
                "Could not restore the previous app. Your backup is {}",
                self.backup.display()
            )
        })
    }
    fn restart(&self, arguments: Vec<std::ffi::OsString>) -> Result<(), String> {
        let receipt = self.backup.with_file_name("started");
        if receipt.exists() {
            self.rollback()?;
            return Err("An unexpected startup receipt exists; restored the previous app".into());
        }
        let mut arguments = arguments.into_iter();
        let mut relaunch = Vec::new();
        while let Some(argument) = arguments.next() {
            if argument == "--app-update-receipt" {
                arguments.next();
            } else {
                relaunch.push(argument);
            }
        }
        let result = std::process::Command::new(&self.path)
            .arg("--appimage-extract-and-run")
            .args(relaunch)
            .arg("--app-update-receipt")
            .arg(&receipt)
            .env_remove("APPIMAGE")
            .env_remove("APPDIR")
            .env_remove("OWD")
            .spawn();
        if let Ok(mut child) = result {
            let start = Instant::now();
            loop {
                if fs::read(&receipt).is_ok_and(|bytes| bytes == b"started") {
                    return Ok(());
                }
                if !matches!(child.try_wait(), Ok(None))
                    || start.elapsed() >= Duration::from_secs(30)
                {
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            let _ = child.kill();
            let _ = child.wait();
        }
        self.rollback()?;
        Err("The updated app could not start; restored the previous AppImage. Try downloading again.".into())
    }
}

/// Acknowledge a rendered window without overwriting any existing file.
pub fn acknowledge(receipt: &Path) -> Result<(), String> {
    acknowledge_at(receipt, &Target::detect()?.path)
}

fn acknowledge_at(receipt: &Path, appimage: &Path) -> Result<(), String> {
    use std::os::unix::fs::OpenOptionsExt;
    let directory = receipt.parent().ok_or("Invalid app update receipt")?;
    let metadata =
        fs::symlink_metadata(directory).map_err(|_| "Missing app update receipt folder")?;
    if receipt.file_name().is_none_or(|name| name != "started")
        || !directory
            .file_name()
            .is_some_and(|name| name.to_string_lossy().starts_with(".ts4-previous-"))
        || directory.parent() != appimage.parent()
        || !metadata.is_dir()
        || metadata.mode() & 0o777 != 0o700
        || !directory.join("previous.AppImage").is_file()
    {
        return Err("Refused an app update receipt outside its backup folder".into());
    }
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(receipt)
        .map_err(|_| "Cannot create app update startup receipt")?;
    file.write_all(b"started")
        .and_then(|_| file.sync_all())
        .map_err(|_| "Cannot acknowledge app update startup".into())
}

#[cfg(test)]
mod tests;
