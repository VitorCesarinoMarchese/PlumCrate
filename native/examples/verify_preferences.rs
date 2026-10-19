//! Capture real Settings with isolated theme fixtures. Never writes the user's settings.
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use ts4_mod_manager_native::{
    catalog::Catalog,
    controls::{Controls, PreferencesSection},
    settings::{CustomTheme, Settings, Theme},
    ui::{self, View},
};
const CASES: [(&str, bool, [f32; 2], bool, PreferencesSection); 10] = [
    (
        "settings-user",
        false,
        [1876., 996.],
        true,
        PreferencesSection::Appearance,
    ),
    (
        "settings-light",
        false,
        [1200., 800.],
        false,
        PreferencesSection::Appearance,
    ),
    (
        "settings-dark",
        true,
        [1200., 800.],
        true,
        PreferencesSection::Appearance,
    ),
    (
        "settings-narrow",
        false,
        [680., 760.],
        true,
        PreferencesSection::Appearance,
    ),
    (
        "settings-minimum",
        false,
        [640., 480.],
        true,
        PreferencesSection::Appearance,
    ),
    (
        "settings-connections",
        false,
        [1200., 800.],
        false,
        PreferencesSection::Connections,
    ),
    (
        "settings-folders",
        false,
        [1200., 800.],
        false,
        PreferencesSection::Folders,
    ),
    (
        "settings-backup",
        false,
        [1200., 800.],
        false,
        PreferencesSection::Backup,
    ),
    (
        "settings-opposing",
        false,
        [1876., 996.],
        true,
        PreferencesSection::Appearance,
    ),
    (
        "settings-opposing-inverse",
        true,
        [1876., 996.],
        true,
        PreferencesSection::Appearance,
    ),
];
struct Capture {
    dir: PathBuf,
    home: tempfile::TempDir,
    controls: Controls,
    catalog: Catalog,
    view: View,
    case: usize,
    frames: usize,
    requested: bool,
    staged: Instant,
    started: Instant,
}
impl Capture {
    fn stage(&mut self, ctx: &egui::Context) {
        let (_, dark, size, edit, section) = CASES[self.case];
        let mut theme = CustomTheme::base();
        theme.name = "Earth".into();
        theme.light.accent = "#8A6F3E".into();
        theme.light.background = "#F3F0E8".into();
        theme.light.text = "#282828".into();
        theme.light.muted_text = "#6F746F".into();
        theme.light.border = "#A99460".into();
        if self.case >= 8 {
            for colors in [&mut theme.light, &mut theme.dark] {
                colors.background = if dark { "#000000" } else { "#FFFFFF" }.into();
                colors.surface = if dark { "#FFFFFF" } else { "#000000" }.into();
                colors.text = "#000000".into();
                colors.muted_text = "#666666".into();
            }
        }
        let settings = Settings {
            theme: if dark { Theme::Dark } else { Theme::Light },
            active_custom_theme: Some(theme.name.clone()),
            custom_themes: vec![theme],
            ..Default::default()
        };
        ui::apply_settings_theme(ctx, &settings);
        self.controls = Controls::new(
            self.home.path().join("managed"),
            self.home.path().into(),
            settings,
            || {},
        )
        .unwrap();
        self.controls.settings_page = true;
        self.controls.set_preferences_section(section);
        if edit {
            self.controls.edit_theme_colors(dark);
        }
        if self.case == 0 {
            self.controls.settings.theme = Theme::Dark;
        }
        self.view = View::default();
        for command in [
            egui::ViewportCommand::MinInnerSize(size.into()),
            egui::ViewportCommand::MaxInnerSize(size.into()),
            egui::ViewportCommand::InnerSize(size.into()),
        ] {
            ctx.send_viewport_cmd(command);
        }
        self.frames = 0;
        self.requested = false;
        self.staged = Instant::now();
    }
}
impl eframe::App for Capture {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.controls.poll(&self.catalog);
        self.view
            .show_with_controls(ui, &mut self.catalog, &mut self.controls);
        let screenshot = ctx.input(|i| {
            i.events.iter().find_map(|e| {
                if let egui::Event::Screenshot { image, .. } = e {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let Some(image) = screenshot {
            assert_eq!(image.size, CASES[self.case].2.map(|v| v as usize));
            let bytes: Vec<_> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
            image::save_buffer(
                self.dir.join(format!("{}.png", CASES[self.case].0)),
                &bytes,
                image.size[0] as u32,
                image.size[1] as u32,
                image::ColorType::Rgba8,
            )
            .unwrap();
            assert!(
                !self
                    .home
                    .path()
                    .join(".config/ts4-mod-manager/settings.json")
                    .exists()
            );
            self.case += 1;
            if self.case == CASES.len() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                return;
            }
            self.stage(&ctx);
        } else if !self.requested
            && self.frames >= 5
            && self.staged.elapsed() >= Duration::from_millis(350)
            && ctx.content_rect().size() == egui::Vec2::from(CASES[self.case].2)
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            self.requested = true;
        }
        self.frames += 1;
        assert!(self.started.elapsed() < Duration::from_secs(60));
        ctx.request_repaint_after(Duration::from_millis(40));
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let dir = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .ok_or("Provide capture directory")?,
    );
    std::fs::create_dir_all(&dir)?;
    let home = tempfile::tempdir()?;
    let controls = Controls::new(
        home.path().join("managed"),
        home.path().into(),
        Settings::default(),
        || {},
    )?;
    eframe::run_native(
        "Settings verification",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size(CASES[0].2)
                .with_resizable(false),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(move |cc| {
            ui::setup(&cc.egui_ctx);
            let mut app = Capture {
                dir,
                home,
                controls,
                catalog: Catalog::default(),
                view: View::default(),
                case: 0,
                frames: 0,
                requested: false,
                staged: Instant::now(),
                started: Instant::now(),
            };
            app.stage(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )?;
    println!("10 production Settings captures; fixture settings never saved");
    Ok(())
}
