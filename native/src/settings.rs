use serde::{Deserialize, Serialize};
use std::{
    io::{Read, Write},
    path::Path,
};
const MAX_TRANSFER_BYTES: u64 = 1024 * 1024;
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    Dark,
    System,
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ThemeColors {
    pub accent: String,
    pub background: String,
    pub surface: String,
    pub text: String,
    pub muted_text: String,
    pub border: String,
}
impl ThemeColors {
    pub fn base(dark: bool) -> Self {
        let values = if dark {
            [
                "#9BC7FF", "#122C49", "#1D3A57", "#F4F8FB", "#BED0DF", "#405C75",
            ]
        } else {
            [
                "#1263CA", "#EDF4F8", "#FFFFFF", "#173452", "#52687C", "#D4E0E9",
            ]
        };
        Self {
            accent: values[0].into(),
            background: values[1].into(),
            surface: values[2].into(),
            text: values[3].into(),
            muted_text: values[4].into(),
            border: values[5].into(),
        }
    }
    pub fn fields(&mut self) -> [(&'static str, &mut String); 6] {
        [
            ("Accent", &mut self.accent),
            ("Background", &mut self.background),
            ("Surface", &mut self.surface),
            ("Text", &mut self.text),
            ("Muted text", &mut self.muted_text),
            ("Border", &mut self.border),
        ]
    }
    pub fn contrast_warnings(&self) -> Vec<String> {
        if !self.valid() { return vec![]; }
        let rgb = |value: &str| {
            let value = u32::from_str_radix(&value[1..], 16).expect("validated hex color");
            [(value >> 16) as u8, (value >> 8) as u8, value as u8]
        };
        let luminance = |color: [u8; 3]| {
            let linear = color.map(|channel| {
                let channel = f64::from(channel) / 255.0;
                if channel <= 0.04045 { channel / 12.92 } else { ((channel + 0.055) / 1.055).powf(2.4) }
            });
            linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722
        };
        let surface = rgb(&self.surface);
        let accent = rgb(&self.accent);
        let mut warnings = vec![];
        for (label, foreground, background) in [
            ("Text on background", rgb(&self.text), rgb(&self.background)),
            ("Text on surface", rgb(&self.text), surface),
            ("Muted text on background", rgb(&self.muted_text), rgb(&self.background)),
            ("Muted text on surface", rgb(&self.muted_text), surface),
            ("Links on surface", accent, surface),
        ].into_iter().chain([0.06, 0.13, 0.20].into_iter().flat_map(|ratio| {
            let fill = std::array::from_fn(|i| (f64::from(surface[i]) * (1.0 - ratio) + f64::from(accent[i]) * ratio) as u8);
            std::iter::once(("Text on controls", rgb(&self.text), fill)).chain((ratio == 0.06).then_some(("Muted text on controls", rgb(&self.muted_text), fill)))
        })) {
            let a = luminance(foreground);
            let b = luminance(background);
            let ratio = (a.max(b) + 0.05) / (a.min(b) + 0.05);
            if ratio < 4.5 { warnings.push(format!("{label}: {ratio:.1}:1 (needs 4.5:1)")); }
        }
        warnings.sort();
        warnings.dedup();
        warnings
    }
    pub fn valid(&self) -> bool {
        [
            &self.accent,
            &self.background,
            &self.surface,
            &self.text,
            &self.muted_text,
            &self.border,
        ]
        .iter()
        .all(|s| {
            s.len() == 7
                && s.starts_with('#')
                && s.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
        })
    }
}
#[derive(Clone, Serialize, PartialEq, Eq)]
pub struct CustomTheme {
    pub name: String,
    pub light: ThemeColors,
    pub dark: ThemeColors,
}

#[derive(Deserialize)]
#[serde(untagged, deny_unknown_fields)]
enum ThemeTransfer {
    Paired {
        name: String,
        light: ThemeColors,
        dark: ThemeColors,
    },
    Legacy {
        name: String,
        colors: ThemeColors,
    },
}

impl<'de> Deserialize<'de> for CustomTheme {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(match ThemeTransfer::deserialize(deserializer)? {
            ThemeTransfer::Paired { name, light, dark } => Self { name, light, dark },
            ThemeTransfer::Legacy { name, colors } => {
                let mut theme = Self::base();
                theme.name = name;
                let dark = u32::from_str_radix(colors.background.trim_start_matches('#'), 16)
                    .map(|rgb| {
                        let red = (rgb >> 16) & 255;
                        let green = (rgb >> 8) & 255;
                        let blue = rgb & 255;
                        299 * red + 587 * green + 114 * blue < 128_000
                    })
                    .unwrap_or(true);
                *theme.palette_mut(dark) = colors;
                theme
            }
        })
    }
}

impl CustomTheme {
    pub fn parse(raw: &str) -> Result<Self, String> {
        if raw.len() as u64 > MAX_TRANSFER_BYTES {
            return Err("Theme JSON exceeds 1 MiB".into());
        }
        let theme: Self = serde_json::from_str(raw).map_err(|_| "Invalid theme JSON")?;
        theme.validate()?;
        Ok(theme)
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty() || self.name.len() > 100 {
            return Err("Theme name must contain 1–100 characters".into());
        }
        if !self.light.valid() {
            return Err("Light colors must use #RRGGBB".into());
        }
        if !self.dark.valid() {
            return Err("Dark colors must use #RRGGBB".into());
        }
        Ok(())
    }
    pub fn base() -> Self {
        Self {
            name: "Custom theme".into(),
            light: ThemeColors::base(false),
            dark: ThemeColors::base(true),
        }
    }
    pub fn palette(&self, dark: bool) -> &ThemeColors {
        if dark { &self.dark } else { &self.light }
    }
    pub fn palette_mut(&mut self, dark: bool) -> &mut ThemeColors {
        if dark {
            &mut self.dark
        } else {
            &mut self.light
        }
    }
}
#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    #[serde(default = "checks_enabled")]
    pub check_app_updates: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub custom_themes: Vec<CustomTheme>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_custom_theme: Option<String>,
    pub version: u32,
    pub theme: Theme,
    pub game_roots: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curseforge_api_key: Option<String>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            check_app_updates: true,
            custom_themes: vec![],
            active_custom_theme: None,
            version: 1,
            theme: Theme::Light,
            game_roots: vec![],
            selected_root: None,
            curseforge_api_key: None,
        }
    }
}
fn checks_enabled() -> bool { true }
impl Settings {
    pub fn parse(raw: &str) -> Result<Self, String> {
        if raw.len() as u64 > MAX_TRANSFER_BYTES {
            return Err("Settings transfer exceeds 1 MiB".into());
        }
        let mut value: Self =
            serde_json::from_str(raw).map_err(|_| "Invalid settings JSON or schema".to_string())?;
        if value.version != 1 {
            return Err("Settings require version 1".into());
        }
        let mut names = std::collections::HashSet::new();
        for theme in &value.custom_themes {
            theme.validate()?;
            if !names.insert(&theme.name) {
                return Err("Custom theme names must be unique".into());
            }
        }
        if value
            .active_custom_theme
            .as_ref()
            .is_some_and(|name| !names.contains(name))
        {
            return Err("Active custom theme must exist in customThemes".into());
        }
        let mut roots = Vec::new();
        for root in &value.game_roots {
            let normalized = normalize_root(root)?;
            if !roots.contains(&normalized) {
                roots.push(normalized);
            }
        }
        value.game_roots = roots;
        value.selected_root = value
            .selected_root
            .as_deref()
            .map(normalize_root)
            .transpose()?;
        if value
            .selected_root
            .as_ref()
            .is_some_and(|root| !value.game_roots.contains(root))
        {
            return Err("Selected root must appear in gameRoots".into());
        }
        Ok(value)
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::default());
        }
        Self::read_transfer(path)
    }
    pub fn read_transfer(path: &Path) -> Result<Self, String> {
        if !std::fs::metadata(path)
            .map_err(|_| "Cannot read settings transfer")?
            .is_file()
        {
            return Err("Settings transfer must be a regular file".into());
        }
        let file = std::fs::File::open(path).map_err(|_| "Cannot read settings transfer")?;
        let mut bytes = Vec::new();
        file.take(MAX_TRANSFER_BYTES + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| "Cannot read settings transfer")?;
        if bytes.len() as u64 > MAX_TRANSFER_BYTES {
            return Err("Settings transfer exceeds 1 MiB".into());
        }
        let raw =
            std::str::from_utf8(&bytes).map_err(|_| "Settings transfer must be UTF-8 JSON")?;
        Self::parse(raw)
    }
    pub fn preserve_omitted_key(&mut self, current: &Self) {
        if self.curseforge_api_key.is_none() {
            self.curseforge_api_key = current.curseforge_api_key.clone();
        }
    }

    pub fn export(&self, include_key: bool) -> Result<String, String> {
        let mut copy = self.clone();
        if !include_key {
            copy.curseforge_api_key = None;
        }
        serde_json::to_string_pretty(&copy).map_err(|_| "Cannot serialize settings".into())
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.write_file(path, false)
    }
    pub fn save_new(&self, path: &Path) -> Result<(), String> {
        let mut transfer = self.clone();
        transfer.custom_themes.clear();
        transfer.active_custom_theme = None;
        transfer.write_file(path, true)
    }
    fn write_file(&self, path: &Path, require_new: bool) -> Result<(), String> {
        let serialized = Self::parse(&self.export(true)?)?.export(true)?;
        let parent = path.parent().ok_or("Settings path has no parent")?;
        std::fs::create_dir_all(parent).map_err(|_| "Cannot create settings directory")?;
        let mut temp =
            tempfile::NamedTempFile::new_in(parent).map_err(|_| "Cannot stage settings")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            temp.as_file()
                .set_permissions(std::fs::Permissions::from_mode(0o600))
                .map_err(|_| "Cannot restrict settings permissions")?;
        }
        temp.write_all(serialized.as_bytes())
            .map_err(|_| "Cannot write settings")?;
        temp.as_file()
            .sync_all()
            .map_err(|_| "Cannot sync settings")?;
        if require_new {
            temp.persist_noclobber(path)
                .map_err(|_| "Settings export requires an unused writable file path")?;
        } else {
            temp.persist(path).map_err(|_| "Cannot publish settings")?;
        }
        std::fs::File::open(parent)
            .and_then(|f| f.sync_all())
            .map_err(|_| "Cannot sync settings directory")?;
        Ok(())
    }
}

fn normalize_root(root: &str) -> Result<String, String> {
    if root.contains('\0') {
        return Err("Game folders must not contain NUL characters".into());
    }
    let path = Path::new(root.trim());
    if !path.is_absolute() {
        return Err("Game folders must be nonempty absolute paths".into());
    }
    let mut normalized = std::path::PathBuf::new();
    for component in path.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            component => normalized.push(component.as_os_str()),
        }
    }
    Ok(normalized.to_string_lossy().into_owned())
}
