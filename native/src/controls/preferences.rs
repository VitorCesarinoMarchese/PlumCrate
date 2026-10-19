use super::*;
use crate::settings::CustomTheme;
use egui::RichText;

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum PreferencesSection {
    #[default]
    Appearance,
    Folders,
    Connections,
    Updates,
    Backup,
}
#[derive(Default)]
pub(super) struct PreferencesView {
    pub(super) section: PreferencesSection,
    editor_open: bool,
    edit_dark: bool,
}

impl Controls {
    pub fn set_preferences_section(&mut self, section: super::PreferencesSection) {
        self.preferences.section = section;
    }
    pub fn edit_theme_colors(&mut self, dark: bool) {
        self.preferences.editor_open = self.theme_edit.is_some();
        self.preferences.edit_dark = dark;
    }
    pub fn settings_dirty(&self) -> bool {
        self.settings != self.saved_settings
            || self.theme_edit.as_ref().is_some_and(|draft| {
                self.saved_settings
                    .custom_themes
                    .iter()
                    .find(|theme| Some(&theme.name) == self.theme_edit_original.as_ref())
                    != Some(draft)
            })
    }
    fn edited_settings(&self) -> Result<Settings, String> {
        let mut next = self.settings.clone();
        if let Some(theme) = &self.theme_edit {
            theme.validate()?;
            if next
                .custom_themes
                .iter()
                .any(|t| t.name == theme.name && self.theme_edit_original.as_ref() != Some(&t.name))
            {
                return Err("That theme name is already in use. Choose another name.".into());
            }
            if let Some(original) = &self.theme_edit_original {
                next.custom_themes.retain(|t| &t.name != original);
            }
            next.active_custom_theme = Some(theme.name.clone());
            next.custom_themes.push(theme.clone());
        }
        Ok(next)
    }
    pub(super) fn sync_imported_theme(&mut self) {
        self.sync_theme_editor();
        self.preferences.editor_open = false;
    }
    fn sync_theme_editor(&mut self) {
        self.theme_edit_original = self.settings.active_custom_theme.clone();
        self.theme_edit = self
            .settings
            .custom_themes
            .iter()
            .find(|theme| Some(&theme.name) == self.theme_edit_original.as_ref())
            .cloned();
    }
    fn stage_theme(&mut self) -> bool {
        match self.edited_settings() {
            Ok(next) => {
                self.settings = next;
                self.sync_theme_editor();
                self.error = None;
                true
            }
            Err(error) => {
                self.error = Some(error);
                false
            }
        }
    }
    fn select_theme(&mut self, name: Option<String>) {
        if self.settings.active_custom_theme == name {
            return;
        }
        if !self.stage_theme() {
            return;
        }
        self.settings.active_custom_theme = name;
        self.sync_theme_editor();
        self.preferences.editor_open = false;
    }
    fn remove_theme(&mut self) {
        self.error = None;
        if let Some(name) = self.settings.active_custom_theme.take() {
            self.settings
                .custom_themes
                .retain(|theme| theme.name != name);
        }
        self.theme_edit = None;
        self.theme_edit_original = None;
        self.preferences.editor_open = false;
        self.notice = Some(
            "Theme removed from draft. Save changes to keep this, or discard to restore it.".into(),
        );
    }
    pub(super) fn discard_preferences(&mut self, ctx: &egui::Context) {
        self.settings = self.saved_settings.clone();
        self.sync_theme_editor();
        self.theme_json.clear();
        self.error = None;
        self.preferences.editor_open = false;
        self.notice = Some("Changes discarded".into());
        crate::ui::apply_settings_theme(ctx, &self.settings);
    }
    pub(super) fn use_default_colors(&mut self, _: &egui::Context) {
        self.select_theme(None);
    }
    pub(super) fn save_preferences(&mut self, ctx: &egui::Context) {
        let next = match self.edited_settings() {
            Ok(next) => next,
            Err(error) => {
                self.error = Some(error);
                return;
            }
        };
        match next.save(&self.settings_path) {
            Ok(()) => {
                self.saved_settings = next.clone();
                self.settings = next;
                self.sync_theme_editor();
                crate::ui::apply_settings_theme(ctx, &self.settings);
                self.error = None;
                self.notice = Some("Settings saved".into());
            }
            Err(error) => self.error = Some(error),
        }
    }
    fn changed_sections(&self) -> Vec<&'static str> {
        let mut changed = vec![];
        if self.settings.theme != self.saved_settings.theme
            || self.settings.active_custom_theme != self.saved_settings.active_custom_theme
            || self.settings.custom_themes != self.saved_settings.custom_themes
            || self.theme_edit.as_ref().is_some_and(|draft| {
                self.saved_settings
                    .custom_themes
                    .iter()
                    .find(|t| Some(&t.name) == self.theme_edit_original.as_ref())
                    != Some(draft)
            })
        {
            changed.push("Appearance");
        }
        if self.settings.game_roots != self.saved_settings.game_roots
            || self.settings.selected_root != self.saved_settings.selected_root
        {
            changed.push("Folders");
        }
        if self.settings.curseforge_api_key != self.saved_settings.curseforge_api_key {
            changed.push("Connections");
        }
        if self.settings.check_app_updates != self.saved_settings.check_app_updates {
            changed.push("App updates");
        }
        changed
    }
    pub fn settings_content(&mut self, ui: &mut egui::Ui, catalog: &Catalog, back_label: &str) {
        let ctx = ui.ctx().clone();
        let dirty = self.settings_dirty();
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("Settings")
                    .size(32.0)
                    .strong()
                    .color(ui.visuals().text_color()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let colors = crate::ui::chrome_colors(ui);
                if ui
                    .add_enabled(
                        dirty,
                        egui::Button::new(RichText::new("Save changes").color(colors.primary_text))
                            .fill(colors.primary),
                    )
                    .clicked()
                {
                    self.save_preferences(&ctx);
                }
                if ui
                    .add_enabled(dirty, egui::Button::new("Discard changes"))
                    .clicked()
                {
                    self.discard_preferences(&ctx);
                }
                if ui
                    .button(back_label)
                    .on_hover_text(
                        "Leaving Settings keeps your draft until you save or discard it.",
                    )
                    .clicked()
                {
                    self.settings_page = false;
                }
            });
        });
        let changes = self.changed_sections();
        ui.label(
            RichText::new(if changes.is_empty() {
                "Saved".into()
            } else {
                format!("Not saved · {}", changes.join(", "))
            })
            .strong()
            .color(ui.visuals().text_color()),
        );
        ui.label(
            RichText::new("Changes apply only after Save changes.")
                .color(ui.visuals().weak_text_color()),
        );
        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            for (section, label) in [
                (PreferencesSection::Appearance, "Appearance"),
                (PreferencesSection::Folders, "Folders"),
                (PreferencesSection::Connections, "Connections"),
                (PreferencesSection::Updates, "App updates"),
                (PreferencesSection::Backup, "Backup"),
            ] {
                crate::ui::choice_value(ui, &mut self.preferences.section, section, label);
            }
        });
        ui.add_space(12.0);
        egui::ScrollArea::vertical()
            .id_salt(("preferences-section", self.preferences.section as u8))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.set_max_width(ui.available_width().min(1120.0));
                match self.preferences.section {
                    PreferencesSection::Appearance => self.appearance(ui),
                    PreferencesSection::Folders => self.folder_preferences(ui, catalog),
                    PreferencesSection::Connections => self.connection_preferences(ui),
                    PreferencesSection::Updates => self
                        .app_updates
                        .settings(ui, &mut self.settings.check_app_updates),
                    PreferencesSection::Backup => self.backup_preferences(ui),
                }
            });
    }
    fn appearance(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("Window appearance")
                    .strong()
                    .color(ui.visuals().text_color()),
            );
            for (theme, label) in [
                (Theme::Light, "Light"),
                (Theme::Dark, "Dark"),
                (Theme::System, "System"),
            ] {
                crate::ui::choice_value(ui, &mut self.settings.theme, theme, label).on_hover_text(
                    if label == "System" {
                        "Follow your desktop's light or dark mode after saving."
                    } else {
                        "Preview this appearance below. Save changes to apply it."
                    },
                );
            }
        });
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new("Theme")
                    .strong()
                    .color(ui.visuals().text_color()),
            );
            if crate::ui::choice_label(ui, self.settings.active_custom_theme.is_none(), "Built-in")
                .clicked()
            {
                self.use_default_colors(ui.ctx());
            }
            let themes: Vec<_> = self
                .settings
                .custom_themes
                .iter()
                .map(|theme| theme.name.clone())
                .collect();
            for name in themes {
                let selected = self.settings.active_custom_theme.as_ref() == Some(&name);
                let label = if selected {
                    self.theme_edit
                        .as_ref()
                        .map(|t| t.name.as_str())
                        .unwrap_or(&name)
                } else {
                    &name
                };
                if crate::ui::choice_label(ui, selected, label).clicked() {
                    self.select_theme(Some(name));
                }
            }
            ui.separator();
            if self.settings.active_custom_theme.is_some() {
                if ui.button("Edit colors").clicked() {
                    self.preferences.editor_open = !self.preferences.editor_open;
                    self.preferences.edit_dark = self.settings.theme == Theme::Dark
                        || (self.settings.theme == Theme::System && ui.visuals().dark_mode);
                }
                if ui
                    .button("Remove theme")
                    .on_hover_text("Remove this theme when you save. Discard changes restores it.")
                    .clicked()
                {
                    self.remove_theme();
                }
            }
            if ui.button("New theme").clicked() && self.stage_theme() {
                let mut theme = CustomTheme::base();
                let mut n = 1;
                while self
                    .settings
                    .custom_themes
                    .iter()
                    .any(|t| t.name == format!("Custom theme {n}"))
                {
                    n += 1;
                }
                theme.name = format!("Custom theme {n}");
                self.settings.active_custom_theme = Some(theme.name.clone());
                self.settings.custom_themes.push(theme.clone());
                self.theme_edit_original = Some(theme.name.clone());
                self.theme_edit = Some(theme);
                self.preferences.editor_open = true;
                self.preferences.edit_dark = self.settings.theme == Theme::Dark;
            }
        });
        ui.add_space(16.0);
        let dark = if self.preferences.editor_open {
            self.preferences.edit_dark
        } else {
            self.settings.theme == Theme::Dark
                || (self.settings.theme == Theme::System && ui.visuals().dark_mode)
        };
        let theme = self.theme_edit.clone().or_else(|| {
            self.settings
                .custom_themes
                .iter()
                .find(|t| Some(&t.name) == self.settings.active_custom_theme.as_ref())
                .cloned()
        });
        let palette = theme
            .as_ref()
            .map(|t| t.palette(dark).clone())
            .unwrap_or_else(|| crate::settings::ThemeColors::base(dark));
        if self.preferences.editor_open && ui.available_width() >= 900.0 {
            let left = (ui.available_width() * 0.51).min(560.0);
            let right = ui.available_width() - left - 24.0;
            ui.horizontal_top(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(left, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| self.theme_editor(ui),
                );
                ui.add_space(14.0);
                ui.allocate_ui_with_layout(
                    egui::vec2(right, 0.0),
                    egui::Layout::top_down(egui::Align::Min),
                    |ui| theme_preview(ui, &palette, dark, self.settings_dirty()),
                );
            });
        } else {
            if self.preferences.editor_open {
                self.theme_editor(ui);
                ui.add_space(20.0);
            }
            theme_preview(ui, &palette, dark, self.settings_dirty());
        }
        ui.add_space(22.0);
        ui.collapsing("Import theme", |ui| {
            ui.label("Paste a theme exported from this app.");
            ui.add(
                egui::TextEdit::multiline(&mut self.theme_json)
                    .text_color(crate::ui::readable_text(
                        ui.visuals().text_color(),
                        &[ui.visuals().text_edit_bg_color()],
                    ))
                    .desired_width(ui.available_width())
                    .desired_rows(4),
            );
            if ui.button("Preview imported theme").clicked() && self.stage_theme() {
                match CustomTheme::parse(&self.theme_json) {
                    Ok(theme) => {
                        self.theme_edit_original = self
                            .settings
                            .custom_themes
                            .iter()
                            .find(|t| t.name == theme.name)
                            .map(|t| t.name.clone());
                        self.theme_edit = Some(theme);
                        self.stage_theme();
                        self.preferences.editor_open = true;
                        self.error = None;
                    }
                    Err(error) => self.error = Some(error),
                }
            }
        });
    }
    fn theme_editor(&mut self, ui: &mut egui::Ui) {
        let Some(theme) = &mut self.theme_edit else {
            return;
        };
        ui.horizontal(|ui| {
            let name = ui.label("Theme name");
            ui.add_sized(
                [(ui.available_width() - 10.0).min(360.0), 36.0],
                egui::TextEdit::singleline(&mut theme.name)
                    .text_color(crate::ui::readable_text(
                        ui.visuals().text_color(),
                        &[ui.visuals().text_edit_bg_color()],
                    ))
                    .margin(egui::vec2(10.0, 9.0)),
            )
            .labelled_by(name.id);
        });
        ui.add_space(8.0);
        ui.horizontal_wrapped(|ui| {
            crate::ui::choice_value(ui, &mut self.preferences.edit_dark, false, "Light colors");
            crate::ui::choice_value(ui, &mut self.preferences.edit_dark, true, "Dark colors");
        });
        ui.label(
            RichText::new("Editing colors only affects the preview.")
                .color(ui.visuals().weak_text_color()),
        );
        ui.add_space(8.0);
        let palette = theme.palette_mut(self.preferences.edit_dark);
        for (label, value) in palette.fields() {
            let (label, purpose) = match label {
                "Accent" => (
                    "Buttons & links",
                    "Primary buttons, links and enabled switches.",
                ),
                "Background" => ("Page background", "The main area behind your content."),
                "Surface" => ("Cards & menus", "Cards, sidebar, menus and dialogs."),
                "Text" => ("Main text", "Titles, names and regular text."),
                "Muted text" => (
                    "Secondary text",
                    "Descriptions and less prominent information.",
                ),
                _ => ("Outlines", "Lines around controls and panels."),
            };
            ui.horizontal(|ui| {
                let response = ui.add_sized(
                    [146.0, 36.0],
                    egui::Label::new(label).halign(egui::Align::Min),
                );
                response.clone().on_hover_text(purpose);
                ui.add_sized(
                    [124.0, 36.0],
                    egui::TextEdit::singleline(value)
                        .text_color(crate::ui::readable_text(
                            ui.visuals().text_color(),
                            &[ui.visuals().text_edit_bg_color()],
                        ))
                        .desired_width(124.0)
                        .char_limit(7)
                        .margin(egui::vec2(10.0, 9.0)),
                )
                .labelled_by(response.id);
                if let Some(color) = crate::ui::hex_color(value) {
                    let mut rgb = [color.r(), color.g(), color.b()];
                    if ui
                        .color_edit_button_srgb(&mut rgb)
                        .labelled_by(response.id)
                        .changed()
                    {
                        *value = format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]);
                    }
                } else {
                    ui.label("#RRGGBB");
                }
            });
        }
        ui.add_space(12.0);
        if !palette.contrast_warnings().is_empty() {
            ui.label("Text is adjusted automatically to stay readable.");
        }
        ui.horizontal_wrapped(|ui| {
            if ui.button("Reset these colors").clicked() {
                *theme.palette_mut(self.preferences.edit_dark) =
                    crate::settings::ThemeColors::base(self.preferences.edit_dark);
            }
            if ui
                .add_enabled(
                    theme.validate().is_ok(),
                    egui::Button::new("Copy theme JSON"),
                )
                .clicked()
            {
                match serde_json::to_string_pretty(theme) {
                    Ok(json) => {
                        ui.ctx().copy_text(json.clone());
                        self.theme_json = json;
                    }
                    Err(_) => self.error = Some("Cannot export theme".into()),
                }
            }
        });
        if let Err(error) = theme.validate() {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }
    }
    fn folder_preferences(&mut self, ui: &mut egui::Ui, catalog: &Catalog) {
        ui.heading("Game folders");
        ui.label("Choose the game folder to open next time the app starts.");
        ui.add_space(16.0);
        for root in self.settings.game_roots.clone() {
            if crate::ui::choice_label(
                ui,
                self.settings.selected_root.as_ref() == Some(&root),
                &root,
            )
            .clicked()
            {
                self.settings.selected_root = Some(root);
            }
        }
        if let Some(root) = &catalog.root {
            if ui
                .button("Add current folder")
                .on_hover_text(root.display().to_string())
                .clicked()
            {
                let path = root.to_string_lossy().into_owned();
                if !self.settings.game_roots.contains(&path) {
                    self.settings.game_roots.push(path.clone());
                }
                self.settings.selected_root = Some(path);
            }
        } else {
            ui.label("Open a game folder from the library to add it here.");
        }
    }
    fn connection_preferences(&mut self, ui: &mut egui::Ui) {
        ui.heading("CurseForge");
        ui.label("Connect mod search and update checks with your own API key.");
        ui.add_space(16.0);
        let label = ui.label("CurseForge API key");
        let mut key = self.settings.curseforge_api_key.clone().unwrap_or_default();
        if ui
            .add_sized(
                [ui.available_width().min(480.0), 36.0],
                egui::TextEdit::singleline(&mut key)
                    .text_color(crate::ui::readable_text(
                        ui.visuals().text_color(),
                        &[ui.visuals().text_edit_bg_color()],
                    ))
                    .password(true)
                    .margin(egui::vec2(10.0, 9.0)),
            )
            .labelled_by(label.id)
            .changed()
        {
            self.settings.curseforge_api_key = (!key.is_empty()).then_some(key);
        }
        ui.label(
            if self.settings.curseforge_api_key != self.saved_settings.curseforge_api_key {
                "Save changes to use this key."
            } else if self.saved_settings.curseforge_api_key.is_some() {
                "Key saved on this device."
            } else {
                "No key saved yet."
            },
        );
        ui.add_space(12.0);
        ui.label("Linking a source to a mod still needs your confirmation.");
    }
    fn backup_preferences(&mut self, ui: &mut egui::Ui) {
        ui.heading("Settings backup");
        ui.label("Export saved settings or review a backup before importing it.");
        if self.settings_dirty() {
            ui.label("Save changes before exporting your latest settings.");
        }
        ui.add_space(16.0);
        ui.checkbox(&mut self.include_key, "Include API key in export");
        ui.horizontal_wrapped(|ui| {
            let ready = self.picker.is_none();
            if ui
                .add_enabled(ready, egui::Button::new("Export settings"))
                .clicked()
            {
                let mut exported = self.saved_settings.clone();
                if !self.include_key {
                    exported.curseforge_api_key = None;
                }
                self.start_picker(PickPurpose::SettingsExport(exported), ui.ctx(), None);
            }
            if ui
                .add_enabled(ready, egui::Button::new("Import settings"))
                .clicked()
            {
                self.start_picker(PickPurpose::SettingsImport, ui.ctx(), None);
            }
        });
        ui.add_space(12.0);
        ui.label("Custom themes use Copy theme JSON in the theme editor.");
    }
}

fn theme_preview(
    ui: &mut egui::Ui,
    palette: &crate::settings::ThemeColors,
    dark: bool,
    dirty: bool,
) {
    ui.label(
        RichText::new(if dirty {
            "Preview · Not applied"
        } else {
            "Preview"
        })
        .size(20.0)
        .strong()
        .color(ui.visuals().text_color()),
    );
    ui.add_space(10.0);
    let Some(v) = crate::ui::custom_palette_visuals(ui.visuals(), palette) else {
        ui.label("Enter a valid color to update the preview.");
        return;
    };
    let chrome = crate::ui::custom_palette_chrome(palette, &v);
    egui::Frame::new()
        .fill(v.panel_fill)
        .corner_radius(12)
        .inner_margin(20)
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width().min(380.0));
            ui.scope(|ui| {
                ui.style_mut().visuals = v;
                ui.label(
                    RichText::new("Your mods")
                        .size(25.0)
                        .strong()
                        .color(ui.visuals().text_color()),
                );
                ui.add_space(16.0);
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(120.0, 36.0), egui::Sense::hover());
                ui.painter().rect_filled(rect, 9, chrome.primary);
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "Add mods",
                    egui::FontId::proportional(14.0),
                    chrome.primary_text,
                );
                ui.add_space(16.0);
                egui::Frame::new()
                    .fill(ui.visuals().extreme_bg_color)
                    .corner_radius(10)
                    .inner_margin(16)
                    .show(ui, |ui| {
                        crate::ui::text_on_fill(ui, ui.visuals().extreme_bg_color);
                        ui.set_min_width(ui.available_width().min(320.0));
                        ui.label(
                            RichText::new("Example mod")
                                .strong()
                                .color(ui.visuals().text_color()),
                        );
                        ui.label(
                            RichText::new("Secondary text").color(ui.visuals().weak_text_color()),
                        );
                    });
                ui.add_space(12.0);
                ui.label(if dark {
                    "Dark version"
                } else {
                    "Light version"
                });
            });
        });
}
