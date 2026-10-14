use std::path::PathBuf;

pub struct Options {
    pub home: PathBuf,
    pub root: Option<PathBuf>,
    pub managed_root: PathBuf,
    pub settings_page: bool,
    pub inspect: bool,
    pub screenshot: Option<PathBuf>,
    pub query: String,
    pub size: [f32; 2],
    pub dark: bool,
    pub help: bool,
    pub verify_workflows: bool,
    pub version: bool,
    pub app_update_receipt: Option<PathBuf>,
}

impl Options {
    pub fn parse(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut args = args.into_iter();
        let mut home = std::env::var_os("HOME").map(PathBuf::from);
        let mut root = None;
        let mut managed = None;
        let mut settings_page = false;
        let mut inspect = false;
        let mut screenshot = None;
        let mut query = String::new();
        let mut size: [f32; 2] = [1200.0, 800.0];
        let mut dark = false;
        let mut help = false;
        let mut verify_workflows = false;
        let mut version = false;
        let mut app_update_receipt = None;
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--version" | "-V" => version = true,
                "--verify-workflows" => verify_workflows = true,
                "--settings" => settings_page = true,
                "--inspect" => inspect = true,
                "--dark" => dark = true,
                "--help" | "-h" => help = true,
                "--home" | "--root" | "--managed-root" | "--screenshot" | "--query" | "--size" | "--app-update-receipt" => {
                    let value = args
                        .next()
                        .filter(|value| !value.starts_with("--"))
                        .ok_or_else(|| format!("Missing value for {arg}"))?;
                    match arg.as_str() {
                        "--app-update-receipt" => app_update_receipt = Some(PathBuf::from(value)),
                        "--home" => home = Some(value.into()),
                        "--root" => root = Some(PathBuf::from(value)),
                        "--managed-root" => managed = Some(PathBuf::from(value)),
                        "--screenshot" => screenshot = Some(PathBuf::from(value)),
                        "--query" => query = value,
                        "--size" => {
                            let (width, height) =
                                value.split_once('x').ok_or("Size must be WIDTHxHEIGHT")?;
                            size = [
                                width.parse().map_err(|_| "Invalid width")?,
                                height.parse().map_err(|_| "Invalid height")?,
                            ];
                            if !size[0].is_finite()
                                || !size[1].is_finite()
                                || !(640.0..=3840.0).contains(&size[0])
                                || !(480.0..=2160.0).contains(&size[1])
                            {
                                return Err("Size must be between 640x480 and 3840x2160".into());
                            }
                        }
                        _ => unreachable!(),
                    }
                }
                _ => return Err(format!("Unknown option: {arg}. Use --help for usage.")),
            }
        }
        if !help && !version && (inspect || screenshot.is_some()) && root.is_none() {
            return Err("--inspect and --screenshot require --root".into());
        }
        let home = if version { home.unwrap_or_default() } else { home.ok_or("HOME is missing. Provide --home.")? };
        let managed_root = managed.unwrap_or_else(|| home.join(".local/share/sims4-mod-manager"));
        Ok(Self {
            home,
            root,
            managed_root,
            settings_page,
            inspect,
            screenshot,
            query,
            size,
            dark,
            help,
            verify_workflows,
            version,
            app_update_receipt,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parse(args: &[&str]) -> Result<Options, String> {
        Options::parse(args.iter().map(|value| value.to_string()))
    }
    #[test]
    fn opens_settings_directly_for_native_capture() {
        assert!(parse(&["--settings"]).unwrap().settings_page);
    }
    #[test]
    fn accepts_version_for_appimage_update_validation() {
        assert!(parse(&["--version"]).unwrap().version);
    }
    #[test]
    fn validates_cli_before_running_or_reading_catalog() {
        assert!(parse(&["--inspect"]).is_err());
        assert!(parse(&["--root"]).is_err());
        assert!(parse(&["--unknown"]).is_err());
        assert!(parse(&["--size", "NaNx800"]).is_err());
        assert!(parse(&["--size", "10x10"]).is_err());
        let options = parse(&[
            "--home",
            "/tmp/fixture",
            "--root",
            "/tmp/Café",
            "--inspect",
            "--query",
            "script",
            "--size",
            "900x600",
        ])
        .expect("args");
        assert_eq!(options.root, Some(PathBuf::from("/tmp/Café")));
        assert_eq!(
            options.managed_root,
            PathBuf::from("/tmp/fixture/.local/share/sims4-mod-manager")
        );
        assert_eq!(options.query, "script");
        assert_eq!(options.size, [900.0, 600.0]);
        assert!(options.inspect);
    }
}
