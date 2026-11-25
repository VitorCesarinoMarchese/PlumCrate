use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{ErrorCode, ManagerError};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum InstanceSource {
    Native,
    Steam,
    Custom,
}

impl InstanceSource {
    fn as_str(&self) -> &'static str {
        match self {
            InstanceSource::Native => "native",
            InstanceSource::Steam => "steam",
            InstanceSource::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GameInstance {
    pub id: String,
    pub path: PathBuf,
    pub source: InstanceSource,
}

impl GameInstance {
    pub fn new(path: PathBuf, source: InstanceSource) -> Self {
        let id = format!("{}:{}", source.as_str(), path.display());
        Self { id, path, source }
    }
}

pub fn required_steamapps_roots(home: &Path) -> Vec<PathBuf> {
    vec![
        home.join(".steam/steam/steamapps"),
        home.join(".local/share/Steam/steamapps"),
        home.join(".steam/root/steamapps"),
        home.join(".var/app/com.valvesoftware.Steam/.steam/steamapps"),
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps"),
    ]
}

pub fn parse_libraryfolders_vdf(content: &str) -> Vec<PathBuf> {
    let mut out = vec![];

    for line in content.lines() {
        if !line.contains("\"path\"") {
            continue;
        }

        let mut parts = line.split('"');
        let mut found_path_key = false;
        let mut candidate: Option<&str> = None;

        while let Some(part) = parts.next() {
            if part == "path" {
                found_path_key = true;
                continue;
            }

            if found_path_key && !part.trim().is_empty() {
                candidate = Some(part);
                break;
            }
        }

        let Some(raw_part) = candidate else {
            continue;
        };

        let raw = raw_part.replace("\\\\", "/");
        if raw.is_empty() {
            continue;
        }

        out.push(PathBuf::from(raw).join("steamapps"));
    }

    out
}

pub fn collect_steamapps_roots(home: &Path) -> Vec<PathBuf> {
    let mut roots = BTreeSet::new();

    for root in required_steamapps_roots(home) {
        roots.insert(root.clone());
        let vdf = root.join("libraryfolders.vdf");
        if let Ok(content) = fs::read_to_string(vdf) {
            for extra in parse_libraryfolders_vdf(&content) {
                roots.insert(extra);
            }
        }
    }

    roots.into_iter().collect()
}

pub fn detect_game_instances(home: &Path) -> Vec<GameInstance> {
    let mut instances = vec![];

    let native = home.join("Documents/Electronic Arts/The Sims 4");
    if native.is_dir() {
        instances.push(GameInstance::new(native, InstanceSource::Native));
    }

    for steamapps in collect_steamapps_roots(home) {
        let compatdata = steamapps.join("compatdata");
        if !compatdata.is_dir() {
            continue;
        }

        let Ok(entries) = fs::read_dir(&compatdata) else {
            continue;
        };

        for app in entries.flatten() {
            let pfx_users = app.path().join("pfx/drive_c/users");
            if !pfx_users.is_dir() {
                continue;
            }

            let Ok(user_entries) = fs::read_dir(pfx_users) else {
                continue;
            };

            for user in user_entries.flatten() {
                let sims = user
                    .path()
                    .join("Documents/Electronic Arts/The Sims 4");
                if sims.is_dir() {
                    instances.push(GameInstance::new(sims, InstanceSource::Steam));
                }
            }
        }
    }

    instances.sort_by(|a, b| a.path.cmp(&b.path));
    instances.dedup_by(|a, b| a.path == b.path && a.source == b.source);
    instances
}

pub fn validate_custom_instance(path: &Path) -> Result<GameInstance, ManagerError> {
    if !path.exists() {
        return Err(ManagerError::new(
            ErrorCode::NotFound,
            format!("Path not found: {}", path.display()),
        ));
    }

    if !path.is_dir() {
        return Err(ManagerError::new(
            ErrorCode::InvalidPath,
            format!("Path is not directory: {}", path.display()),
        ));
    }

    let mods = path.join("Mods");
    if !mods.is_dir() {
        return Err(ManagerError::new(
            ErrorCode::InvalidPath,
            format!("Missing Mods directory: {}", mods.display()),
        ));
    }

    #[cfg(unix)]
    crate::fs_scope::Directory::open(&mods).map_err(|error| {
        ManagerError::new(error.code, format!(
            "Cannot use {}. Choose a game folder with a readable Mods directory and no symbolic links in its path. {}",
            mods.display(), error.message,
        ))
    })?;

    Ok(GameInstance::new(path.to_path_buf(), InstanceSource::Custom))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::{
        collect_steamapps_roots, detect_game_instances, parse_libraryfolders_vdf,
        required_steamapps_roots, validate_custom_instance, InstanceSource,
    };

    #[test]
    fn detects_native_path_when_present() {
        let home = TempDir::new().expect("tempdir");
        let native = home
            .path()
            .join("Documents/Electronic Arts/The Sims 4");
        fs::create_dir_all(&native).expect("create native");

        let instances = detect_game_instances(home.path());
        assert!(instances.iter().any(|i| i.path == native && i.source == InstanceSource::Native));
    }

    #[test]
    fn exposes_required_steam_root_locations() {
        let home = TempDir::new().expect("tempdir");
        let roots = required_steamapps_roots(home.path());

        assert_eq!(roots.len(), 5);
        assert!(roots
            .iter()
            .any(|p| p.ends_with(".steam/steam/steamapps")));
        assert!(roots
            .iter()
            .any(|p| p.ends_with(".local/share/Steam/steamapps")));
        assert!(roots.iter().any(|p| p.ends_with(".steam/root/steamapps")));
        assert!(roots
            .iter()
            .any(|p| p.ends_with(".var/app/com.valvesoftware.Steam/.steam/steamapps")));
        assert!(roots.iter().any(|p|
            p.ends_with(".var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps")));
    }

    #[test]
    fn parses_libraryfolders_vdf_custom_libraries() {
        let vdf = r#"
"libraryfolders"
{
  "0"
  {
    "path" "\\home\\tept\\Games\\SteamLibrary"
  }
  "1"
  {
    "path" "\\mnt\\ssd\\Steam2"
  }
}
"#;

        let paths = parse_libraryfolders_vdf(vdf);
        assert_eq!(paths.len(), 2);
        assert!(paths.iter().any(|p| p.ends_with("home/tept/Games/SteamLibrary/steamapps")));
        assert!(paths.iter().any(|p| p.ends_with("mnt/ssd/Steam2/steamapps")));
    }

    #[test]
    fn detects_proton_paths_under_compatdata() {
        let home = TempDir::new().expect("tempdir");
        let steamapps = home.path().join(".steam/steam/steamapps");
        let proton = steamapps.join(
            "compatdata/1222670/pfx/drive_c/users/steamuser/Documents/Electronic Arts/The Sims 4",
        );
        fs::create_dir_all(&proton).expect("create proton");

        let instances = detect_game_instances(home.path());
        assert!(instances
            .iter()
            .any(|i| i.path == proton && i.source == InstanceSource::Steam));
    }

    #[test]
    fn returns_multiple_instances_with_correct_sources() {
        let home = TempDir::new().expect("tempdir");
        let native = home
            .path()
            .join("Documents/Electronic Arts/The Sims 4");
        let proton = home.path().join(
            ".steam/steam/steamapps/compatdata/1222670/pfx/drive_c/users/u/Documents/Electronic Arts/The Sims 4",
        );

        fs::create_dir_all(&native).expect("native");
        fs::create_dir_all(&proton).expect("proton");

        let instances = detect_game_instances(home.path());

        assert_eq!(instances.len(), 2);
        assert!(instances
            .iter()
            .any(|i| i.path == native && i.source == InstanceSource::Native));
        assert!(instances
            .iter()
            .any(|i| i.path == proton && i.source == InstanceSource::Steam));
    }

    #[cfg(unix)]
    #[test]
    fn custom_instance_rejects_symlinked_mods_before_opening_the_library() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("game");
        let external = temp.path().join("library");
        fs::create_dir(&root).unwrap();
        fs::create_dir(&external).unwrap();
        std::os::unix::fs::symlink(external, root.join("Mods")).unwrap();
        assert!(validate_custom_instance(&root).unwrap_err().message.contains("symbolic links"));
    }

    #[test]
    fn validates_custom_path_and_rejects_invalid() {
        let home = TempDir::new().expect("tempdir");
        let missing = home.path().join("nope");
        let err = validate_custom_instance(&missing).expect_err("must fail");
        assert_eq!(err.code.as_str(), "NOT_FOUND");

        let bad = home.path().join("Custom/The Sims 4");
        fs::create_dir_all(&bad).expect("create bad dir");
        let err = validate_custom_instance(&bad).expect_err("must fail");
        assert_eq!(err.code.as_str(), "INVALID_PATH");

        let ok = bad.join("Mods");
        fs::create_dir_all(&ok).expect("create mods");
        let validated = validate_custom_instance(&bad).expect("valid custom");
        assert_eq!(validated.source, InstanceSource::Custom);
        assert_eq!(validated.path, bad);
    }

    #[test]
    fn collects_custom_steam_roots_from_vdf() {
        let home = TempDir::new().expect("tempdir");
        let default_root = home.path().join(".steam/steam/steamapps");
        fs::create_dir_all(&default_root).expect("create steamapps");

        let vdf = default_root.join("libraryfolders.vdf");
        fs::write(
            &vdf,
            r#"
"libraryfolders"
{
  "0" { "path" "\\home\\tept\\Games\\MyLib" }
}
"#,
        )
        .expect("write vdf");

        let roots = collect_steamapps_roots(home.path());
        assert!(roots
            .iter()
            .any(|p| p.ends_with("home/tept/Games/MyLib/steamapps")));
    }
}
