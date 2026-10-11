use crate::catalog::EntryId;
use crate::catalog::{Catalog, ModFilter};
use egui::{Color32, RichText};
use std::path::PathBuf;
use ts4_mod_manager_core::path_detection::GameInstance;

fastframe_icons::icons! {
    pub enum Icon {
        prefix: "ts4-native-",
        directory: "../icons/",
        Search => lucide "search",
        Refresh => lucide "refresh-cw",
        Moon => lucide "moon",
        Sun => lucide "sun",
        Back => lucide "arrow-left",
        Copy => lucide "copy",
        Folder => "folder",
        File => "file",
        Check => lucide "check",
        Grid => "layout-grid",
        List => "list",
        Library => "library",
        Archive => "archive",
    }
}

pub fn setup(ctx: &egui::Context) {
    egui_extras::install_image_loaders(ctx);
    fastframe_icons::install::<Icon>(ctx);
    let mut fonts = fastframe_fonts::FontSetup::default().definitions();
    let rendering = fastframe_text::detect();
    rendering.apply_to(&mut fonts);
    ctx.set_fonts(fonts);
    base_style(ctx);
}

fn base_style(ctx: &egui::Context) {
    let rendering = fastframe_text::detect();
    ctx.all_styles_mut(|style| {
        let dark = style.visuals.dark_mode;
        style.visuals.panel_fill = if dark {
            Color32::from_rgb(19, 19, 21)
        } else {
            Color32::from_rgb(246, 244, 239)
        };
        style.visuals.extreme_bg_color = if dark {
            Color32::from_rgb(27, 27, 30)
        } else {
            Color32::WHITE
        };
        style.visuals.override_text_color = Some(if dark {
            Color32::from_rgb(245, 243, 238)
        } else {
            Color32::from_rgb(31, 30, 27)
        });
        style.visuals.weak_text_color = Some(if dark {
            Color32::from_rgb(179, 177, 171)
        } else {
            Color32::from_rgb(101, 96, 86)
        });
        style.visuals.selection.bg_fill = if dark {
            Color32::from_rgb(61, 52, 35)
        } else {
            Color32::from_rgb(235, 221, 185)
        };
        style.visuals.selection.stroke.color = if dark {
            Color32::from_rgb(232, 207, 149)
        } else {
            Color32::from_rgb(83, 61, 20)
        };
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(12.0, 8.0);
        style.spacing.interact_size.y = 34.0;
        style.visuals.widgets.noninteractive.bg_stroke.color = if dark {
            Color32::from_rgb(57, 57, 61)
        } else {
            Color32::from_rgb(218, 213, 201)
        };
        style.visuals.widgets.noninteractive.fg_stroke.color = if dark {
            Color32::from_rgb(179, 177, 171)
        } else {
            Color32::from_rgb(101, 96, 86)
        };
        for widget in [
            &mut style.visuals.widgets.inactive,
            &mut style.visuals.widgets.hovered,
            &mut style.visuals.widgets.active,
        ] {
            widget.corner_radius = egui::CornerRadius::same(6);
        }
        style.visuals.widgets.inactive.weak_bg_fill = if dark {
            Color32::from_rgb(37, 37, 40)
        } else {
            Color32::from_rgb(233, 229, 220)
        };
        rendering.apply_to_visuals(&mut style.visuals);
    });
}

pub fn hex_color(value: &str) -> Option<Color32> {
    if value.len() != 7
        || !value.starts_with('#')
        || !value.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
    {
        return None;
    }
    let rgb = u32::from_str_radix(&value[1..], 16).ok()?;
    Some(Color32::from_rgb(
        (rgb >> 16) as u8,
        (rgb >> 8) as u8,
        rgb as u8,
    ))
}
pub fn apply_settings_theme(ctx: &egui::Context, settings: &crate::settings::Settings) {
    use crate::settings::Theme;
    base_style(ctx);
    ctx.set_theme(match settings.theme {
        Theme::Light => egui::ThemePreference::Light,
        Theme::Dark => egui::ThemePreference::Dark,
        Theme::System => egui::ThemePreference::System,
    });
    if let Some(theme) = settings
        .custom_themes
        .iter()
        .find(|t| settings.active_custom_theme.as_ref() == Some(&t.name))
    {
        let blend = |a: Color32, b: Color32, ratio: f32| {
            Color32::from_rgb(
                (a.r() as f32 * (1.0 - ratio) + b.r() as f32 * ratio) as u8,
                (a.g() as f32 * (1.0 - ratio) + b.g() as f32 * ratio) as u8,
                (a.b() as f32 * (1.0 - ratio) + b.b() as f32 * ratio) as u8,
            )
        };
        ctx.all_styles_mut(|style| {
            let colors = theme.palette(style.visuals.dark_mode);
            let Some([accent, background, surface, text, muted, border]) = [
                &colors.accent,
                &colors.background,
                &colors.surface,
                &colors.text,
                &colors.muted_text,
                &colors.border,
            ]
            .map(|s| hex_color(s))
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .and_then(|v| <Vec<Color32> as TryInto<[Color32; 6]>>::try_into(v).ok()) else {
                return;
            };
            let v = &mut style.visuals;
            v.panel_fill = background;
            v.extreme_bg_color = surface;
            v.window_fill = surface;
            v.faint_bg_color = surface;
            v.code_bg_color = surface;
            v.override_text_color = Some(text);
            v.weak_text_color = Some(muted);
            v.selection.bg_fill = blend(surface, accent, 0.2);
            v.selection.stroke.color = text;
            v.hyperlink_color = accent;
            v.window_stroke.color = border;
            v.widgets.noninteractive.bg_stroke.color = border;
            v.widgets.noninteractive.fg_stroke.color = muted;
            for (widget, amount) in [
                (&mut v.widgets.inactive, 0.06),
                (&mut v.widgets.hovered, 0.13),
                (&mut v.widgets.active, 0.20),
                (&mut v.widgets.open, 0.13),
            ] {
                widget.bg_fill = blend(surface, accent, amount);
                widget.weak_bg_fill = widget.bg_fill;
                widget.bg_stroke.color = border;
                widget.fg_stroke.color = text;
            }
        });
    }
}

fn cover_uv(source: egui::Vec2, destination: egui::Vec2) -> egui::Rect {
    let source_aspect = source.x / source.y;
    let destination_aspect = destination.x / destination.y;
    let crop = if source_aspect > destination_aspect {
        egui::vec2(destination_aspect / source_aspect, 1.0)
    } else {
        egui::vec2(1.0, source_aspect / destination_aspect)
    };
    egui::Rect::from_center_size(egui::pos2(0.5, 0.5), crop)
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum LibraryLayout {
    #[default]
    Cards,
    List,
}

#[derive(Default)]
pub struct View {
    pub root_input: String,
    pub instances: Vec<GameInstance>,
    pub scan_ms: Option<f64>,
    pub stats: RenderStats,
    detail_only: bool,
    editing_root: bool,
    copied: Option<(EntryId, f64)>,
    previews: crate::preview::Previews,
    layout: LibraryLayout,
}

#[derive(Default)]
pub struct RenderStats {
    pub logo: Option<egui::Rect>,
    pub theme_button: Option<egui::Rect>,
    pub rows: usize,
    pub files: usize,
    pub search: Option<egui::Rect>,
    pub first_row: Option<egui::Rect>,
    pub back: Option<egui::Rect>,
    pub copy: Option<egui::Rect>,
    pub photos: usize,
    pub missing_photos: usize,
    pub list_switch: Option<egui::Rect>,
}

impl View {
    pub fn show(&mut self, ui: &mut egui::Ui, catalog: &mut Catalog) -> Option<PathBuf> {
        self.show_inner(ui, catalog, None)
    }
    pub fn show_with_controls(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &mut Catalog,
        controls: &mut crate::controls::Controls,
    ) -> Option<PathBuf> {
        self.show_inner(ui, catalog, Some(controls))
    }
    fn show_inner(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &mut Catalog,
        mut controls: Option<&mut crate::controls::Controls>,
    ) -> Option<PathBuf> {
        self.previews.begin_frame(ui.ctx(), catalog.generation());
        self.stats = RenderStats::default();
        let mut scan_root = None;
        let narrow = ui.available_width() < 1040.0;
        let foreground = ui.visuals().text_color();
        egui::Panel::top("catalog-header")
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(ui.visuals().panel_fill)
                    .inner_margin(egui::Margin::symmetric(24, 10)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let logo = ui
                        .add(
                            egui::Image::new(egui::include_image!("../assets/logo.svg"))
                                .fit_to_exact_size(egui::vec2(160.0, 160.0 * 482.0 / 2148.0))
                                .alt_text("Sims 4 Mod Manager"),
                        )
                        .on_hover_text("Sims 4 Mod Manager");
                    self.stats.logo = Some(logo.rect);
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(c) = controls.as_deref_mut()
                            && ui.selectable_label(c.settings_page, "Settings").clicked()
                        {
                            c.settings_page = !c.settings_page;
                        }
                        let dark = ui.visuals().dark_mode;
                        let icon = if dark { Icon::Sun } else { Icon::Moon };
                        let label = if dark { "Light" } else { "Dark" };
                        let theme_button = ui.add(egui::Button::image_and_text(
                            icon.image(foreground, 16.0),
                            label,
                        ));
                        self.stats.theme_button = Some(theme_button.rect);
                        if theme_button.clicked() {
                            if let Some(c) = controls.as_deref_mut() {
                                c.set_color_mode(
                                    ui.ctx(),
                                    if dark {
                                        crate::settings::Theme::Light
                                    } else {
                                        crate::settings::Theme::Dark
                                    },
                                );
                            } else {
                                ui.ctx().set_theme(if dark {
                                    egui::Theme::Light
                                } else {
                                    egui::Theme::Dark
                                });
                            }
                        }
                    });
                });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add(Icon::Folder.image(ui.visuals().weak_text_color(), 18.0));
                    match catalog.root.as_ref().filter(|_| !self.editing_root) {
                        None => {
                            if let Some(c) = controls.as_deref_mut()
                                && ui.add_enabled(!c.picker_open(), egui::Button::new("Choose folder")).clicked()
                            {
                                c.choose_game_folder(ui.ctx(), None);
                            }
                            let response = ui.add_sized(
                                [(ui.available_width() - 80.0).max(100.0), 36.0],
                                egui::TextEdit::singleline(&mut self.root_input)
                                    .hint_text("Game folder containing Mods")
                                    .margin(egui::vec2(10.0, 9.0)),
                            );
                            let enter = response.lost_focus()
                                && ui.input(|input| input.key_pressed(egui::Key::Enter));
                            if ui.button("Open").clicked() || enter {
                                let value = self.root_input.trim();
                                if !value.is_empty() {
                                    scan_root = Some(PathBuf::from(value));
                                    self.editing_root = false;
                                }
                            }
                        }
                        Some(root) => {
                            let path = root.to_string_lossy();
                            let tail: String = path
                                .chars()
                                .rev()
                                .take(64)
                                .collect::<String>()
                                .chars()
                                .rev()
                                .collect();
                            let display = if tail.len() < path.len() {
                                format!("…{tail}")
                            } else {
                                tail
                            };
                            ui.allocate_ui_with_layout(
                                egui::vec2((ui.available_width() - 130.0).max(100.0), 34.0),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(display)
                                                .color(ui.visuals().weak_text_color()),
                                        )
                                        .truncate(),
                                    )
                                    .on_hover_text(path.as_ref());
                                },
                            );
                            if ui.button("Change folder").clicked() {
                                if let Some(c) = controls.as_deref_mut() {
                                    c.choose_game_folder(ui.ctx(), Some(root.clone()));
                                } else {
                                    self.root_input = path.into_owned();
                                    self.editing_root = true;
                                }
                            }
                        }
                    }
                });
                if (self.editing_root || catalog.root.is_none()) && !self.instances.is_empty() {
                    egui::ComboBox::from_id_salt("instances")
                        .selected_text("Detected game folders")
                        .show_ui(ui, |ui| {
                            for instance in &self.instances {
                                let label = instance.path.to_string_lossy();
                                if ui
                                    .selectable_label(
                                        catalog.root.as_ref() == Some(&instance.path),
                                        label.as_ref(),
                                    )
                                    .clicked()
                                {
                                    self.root_input = label.into_owned();
                                    scan_root = Some(instance.path.clone());
                                    self.editing_root = false;
                                }
                            }
                        });
                }
            });

        egui::Panel::bottom("catalog-status")
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(ui.visuals().panel_fill)
                    .inner_margin(egui::Margin::symmetric(24, 6)),
            )
            .show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        RichText::new("Local library")
                            .size(12.0)
                            .color(ui.visuals().weak_text_color()),
                    );
                    ui.separator();
                    if catalog.loading {
                        ui.spinner();
                        ui.label(RichText::new("Scanning your Mods folder...").size(12.0));
                    } else if let Some(ms) = self.scan_ms {
                        ui.label(
                            RichText::new(format!(
                                "{} mods · scanned in {ms:.0} ms",
                                catalog.mods.len()
                            ))
                            .size(12.0)
                            .color(ui.visuals().weak_text_color()),
                        );
                    } else {
                        ui.label(
                            RichText::new("Choose a game folder to browse your mods.").size(12.0),
                        );
                    }
                });
                if let Some(c) = controls.as_deref_mut() {
                    c.activity(ui);
                }
            });

        let settings_page = controls.as_ref().is_some_and(|c| c.settings_page);
        if !narrow {
            egui::Panel::left("library-navigation")
                .exact_size(176.0)
                .resizable(false)
                .frame(
                    egui::Frame::new()
                        .fill(ui.visuals().extreme_bg_color)
                        .inner_margin(16),
                )
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add(Icon::Library.image(ui.visuals().selection.stroke.color, 20.0));
                        ui.label(RichText::new("My library").size(16.0).strong());
                    });
                    ui.add_space(24.0);
                    for filter in [ModFilter::All, ModFilter::Installed, ModFilter::Stored] {
                        let count = catalog.mods.iter().filter(|m| filter.matches(m)).count();
                        let icon = match filter {
                            ModFilter::All => Icon::Grid,
                            ModFilter::Installed => Icon::Check,
                            ModFilter::Stored => Icon::Archive,
                        };
                        let active = !settings_page && catalog.installation_filter == filter;
                        let button = egui::Button::image_and_text(
                            icon.image(
                                if active {
                                    ui.visuals().selection.stroke.color
                                } else {
                                    ui.visuals().weak_text_color()
                                },
                                16.0,
                            ),
                            format!("{}  {count}", filter.label()),
                        )
                        .selected(active);
                        if ui.add_sized([ui.available_width(), 40.0], button).clicked() {
                            catalog.set_filter(filter);
                            if let Some(c) = controls.as_deref_mut() {
                                c.settings_page = false;
                            }
                        }
                        ui.add_space(4.0);
                    }
                    ui.add_space(24.0);
                    ui.separator();
                    ui.add_space(16.0);
                    ui.label(RichText::new("The Sims 4").size(13.0).strong());
                    ui.label(
                        RichText::new("Your mods and custom content, together.")
                            .size(12.0)
                            .color(ui.visuals().weak_text_color()),
                    );
                });
        }

        let settings_page = controls.as_ref().is_some_and(|c| c.settings_page);
        if !narrow && !settings_page {
            egui::Panel::right("mod-details")
                .default_size(300.0)
                .min_size(280.0)
                .max_size(400.0)
                .frame(
                    egui::Frame::new()
                        .fill(ui.visuals().extreme_bg_color)
                        .inner_margin(24),
                )
                .show(ui, |ui| {
                    self.details(ui, catalog, controls.as_deref_mut());
                });
        }

        egui::CentralPanel::default().frame(egui::Frame::new().fill(ui.visuals().panel_fill).inner_margin(24))
            .show(ui, |ui| {
                if settings_page {
                    if let Some(c) = controls.as_deref_mut() { c.settings_content(ui, catalog); }
                    return;
                }
                if let Some(error) = &catalog.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                    ui.label("Check that the game folder exists and contains a readable Mods folder, then open it again.");
                    ui.add_space(16.0);
                }
                if narrow && self.detail_only && catalog.selected_mod().is_some() {
                    let back = ui.add(egui::Button::image_and_text(Icon::Back.image(foreground, 16.0), "Back to mods"));
                    self.stats.back = Some(back.rect);
                    if back.clicked() {
                        self.detail_only = false;
                    }
                    ui.add_space(16.0);
                    self.details(ui, catalog, controls.as_deref_mut());
                    return;
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Mod library").size(27.0).strong());
                    ui.label(RichText::new(format!("{}", catalog.mods.len())).size(14.0).color(ui.visuals().weak_text_color()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.add_enabled(catalog.root.is_some() && !catalog.loading,
                            egui::Button::image_and_text(Icon::Refresh.image(foreground, 16.0), "Rescan")).clicked() {
                            scan_root = catalog.root.clone();
                        }
                    });
                });
                ui.label(RichText::new("Manage the mods and custom content in your local collection.").size(13.0).color(ui.visuals().weak_text_color()));
                ui.add_space(12.0);
                if let Some(c) = controls.as_deref_mut() { c.toolbar(ui, catalog); ui.add_space(12.0); }
                egui::Frame::new()
                    .fill(ui.visuals().extreme_bg_color)
                    .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                    .corner_radius(8)
                    .inner_margin(egui::Margin::symmetric(12, 0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.add(Icon::Search.image(ui.visuals().weak_text_color(), 18.0));
                            let mut query = catalog.query.clone();
                            let response = ui.add_sized(
                                [ui.available_width(), 40.0],
                                egui::TextEdit::singleline(&mut query)
                                    .hint_text("Search mods or filenames")
                                    .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(0, 11))),
                            );
                            self.stats.search = Some(response.rect);
                            if response.changed() { catalog.search(query); }
                        });
                    });
                ui.add_space(16.0);
                ui.horizontal_wrapped(|ui| {
                    if narrow {
                        for filter in [ModFilter::All, ModFilter::Installed, ModFilter::Stored] {
                            if ui.selectable_label(catalog.installation_filter == filter, filter.label()).clicked() {
                                catalog.set_filter(filter);
                            }
                        }
                    } else {
                        ui.label(RichText::new(format!("{} · {} mods", catalog.installation_filter.label(), catalog.visible.len())).size(13.0).color(ui.visuals().weak_text_color()));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let list = ui.add(egui::Button::image(Icon::List.image(foreground, 16.0)).selected(self.layout == LibraryLayout::List)).on_hover_text("List view");
                        self.stats.list_switch = Some(list.rect);
                        if list.clicked() { self.layout = LibraryLayout::List; }
                        if ui.add(egui::Button::image(Icon::Grid.image(foreground, 16.0)).selected(self.layout == LibraryLayout::Cards)).on_hover_text("Card view").clicked() {
                            self.layout = LibraryLayout::Cards;
                        }
                    });
                });
                ui.add_space(12.0);
                if !catalog.query.is_empty() {
                    ui.label(RichText::new(format!("{} matching mods", catalog.visible.len())).size(12.0).color(ui.visuals().weak_text_color()));
                }
                if catalog.visible.is_empty() {
                    ui.add_space(32.0);
                    let (title, message) = if catalog.loading {
                        ("Reading your catalog", "You can keep using the window while the scan runs.")
                    } else if catalog.root.is_none() {
                        ("Start with your game folder", "Open the Sims 4 folder containing Mods, or choose a detected game folder above.")
                    } else if !catalog.query.is_empty() {
                        ("No matching mods", "Try another name or filename, or clear your search.")
                    } else if catalog.installation_filter != ModFilter::All {
                        ("No mods in this view", "Switch to All mods to see the rest of your collection.")
                    } else {
                        ("No mods found", "This folder has no package or script mods. Managed mods also appear here when stored locally.")
                    };
                    ui.heading(title); ui.label(message);
                    return;
                }
                match self.layout {
                    LibraryLayout::Cards => self.card_grid(ui, catalog, narrow, controls.as_deref_mut()),
                    LibraryLayout::List => self.library_list(ui, catalog, controls),
                }

            });
        scan_root
    }

    fn card_grid(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &mut Catalog,
        narrow: bool,
        mut controls: Option<&mut crate::controls::Controls>,
    ) {
        let gap = 16.0;
        let columns = ((ui.available_width() + gap) / 236.0).floor().max(1.0) as usize;
        let width = (ui.available_width() - gap * (columns - 1) as f32) / columns as f32;
        let height = 262.0;
        let rows = catalog.visible.len().div_ceil(columns);
        ui.spacing_mut().item_spacing.y = gap;
        egui::ScrollArea::vertical()
            .id_salt((
                "photo-library",
                catalog.root.clone(),
                catalog.installation_filter as u8,
            ))
            .auto_shrink([false, false])
            .show_rows(ui, height, rows, |ui, range| {
                for row in range {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for column in 0..columns {
                            let Some(&index) = catalog.visible.get(row * columns + column) else {
                                break;
                            };
                            let entry = &catalog.mods[index];
                            let identity = EntryId::of(entry);
                            let selected = catalog.selected.as_ref() == Some(&identity);
                            let response = ui
                                .push_id(&identity, |ui| {
                                    let (rect, response) = ui.allocate_exact_size(
                                        egui::vec2(width, height),
                                        egui::Sense::click(),
                                    );
                                    let color = if response.hovered() {
                                        ui.visuals().widgets.hovered.weak_bg_fill
                                    } else {
                                        ui.visuals().extreme_bg_color
                                    };
                                    let stroke = if selected || response.has_focus() {
                                        egui::Stroke::new(1.5, ui.visuals().selection.stroke.color)
                                    } else {
                                        ui.visuals().widgets.noninteractive.bg_stroke
                                    };
                                    ui.painter().rect(
                                        rect,
                                        12,
                                        color,
                                        stroke,
                                        egui::StrokeKind::Inside,
                                    );
                                    let image_rect = egui::Rect::from_min_max(
                                        rect.min + egui::vec2(8.0, 8.0),
                                        egui::pos2(rect.right() - 8.0, rect.top() + 158.0),
                                    );
                                    self.paint_preview(
                                        ui,
                                        entry.preview.as_deref(),
                                        catalog.root.as_deref(),
                                        image_rect,
                                        true,
                                    );
                                    let title_rect = egui::Rect::from_min_max(
                                        rect.min + egui::vec2(14.0, 172.0),
                                        rect.max - egui::vec2(14.0, 46.0),
                                    );
                                    let mut text = ui.new_child(
                                        egui::UiBuilder::new()
                                            .max_rect(title_rect)
                                            .layout(egui::Layout::top_down(egui::Align::Min)),
                                    );
                                    text.add(
                                        egui::Label::new(
                                            RichText::new(&entry.name).size(15.0).strong(),
                                        )
                                        .truncate(),
                                    );
                                    let author = entry
                                        .source_attachment
                                        .as_ref()
                                        .and_then(|source| source.author.as_deref());
                                    text.add(
                                        egui::Label::new(
                                            RichText::new(
                                                author
                                                    .map(|author| format!("By {author}"))
                                                    .unwrap_or_else(|| {
                                                        if entry.id.is_some() {
                                                            "Managed collection".into()
                                                        } else {
                                                            "External collection".into()
                                                        }
                                                    }),
                                            )
                                            .size(12.0)
                                            .color(ui.visuals().weak_text_color()),
                                        )
                                        .truncate(),
                                    );
                                    let footer_rect = egui::Rect::from_min_max(
                                        egui::pos2(rect.left() + 14.0, rect.bottom() - 35.0),
                                        rect.max - egui::vec2(14.0, 9.0),
                                    );
                                    let mut footer = ui.new_child(
                                        egui::UiBuilder::new().max_rect(footer_rect).layout(
                                            egui::Layout::left_to_right(egui::Align::Center),
                                        ),
                                    );
                                    footer.add(
                                        Icon::File.image(ui.visuals().weak_text_color(), 13.0),
                                    );
                                    footer.label(
                                        RichText::new(format!(
                                            "{} {}",
                                            entry.mod_files.len(),
                                            if entry.mod_files.len() == 1 {
                                                "file"
                                            } else {
                                                "files"
                                            }
                                        ))
                                        .size(12.0)
                                        .color(ui.visuals().weak_text_color()),
                                    );
                                    footer.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let accent = if entry.enabled {
                                                ui.visuals().selection.stroke.color
                                            } else {
                                                ui.visuals().weak_text_color()
                                            };
                                            ui.label(
                                                RichText::new(if entry.enabled {
                                                    "Installed"
                                                } else {
                                                    "Stored"
                                                })
                                                .size(12.0)
                                                .color(accent),
                                            );
                                            if entry.enabled {
                                                ui.add(Icon::Check.image(accent, 13.0));
                                            }
                                        },
                                    );
                                    response.widget_info(|| {
                                        egui::WidgetInfo::selected(
                                            egui::WidgetType::SelectableLabel,
                                            ui.is_enabled(),
                                            selected,
                                            &entry.name,
                                        )
                                    });
                                    response.on_hover_text(&entry.name)
                                })
                                .inner;
                            if response.secondary_clicked() {
                                catalog.select(index);
                            }
                            if let Some(c) = controls.as_deref_mut() {
                                response.context_menu(|ui| c.mod_menu(ui, catalog));
                            }
                            if self.stats.first_row.is_none() {
                                self.stats.first_row = Some(response.rect);
                            }
                            self.stats.rows += 1;
                            if response.clicked() {
                                catalog.select(index);
                                self.detail_only = narrow;
                            }
                        }
                    });
                }
            });
    }

    fn library_list(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &mut Catalog,
        mut controls: Option<&mut crate::controls::Controls>,
    ) {
        let foreground = ui.visuals().text_color();
        let root_id = catalog.root.clone();
        egui::ScrollArea::vertical()
            .id_salt(("catalog", root_id))
            .auto_shrink([false, false])
            .show_rows(ui, 64.0, catalog.visible.len(), |ui, range| {
                for visible_index in range {
                    let index = catalog.visible[visible_index];
                    let entry = &catalog.mods[index];
                    let identity = EntryId::of(entry);
                    let origin = match entry.source {
                        ts4_mod_manager_core::mod_scan::ModSource::Managed => "Managed",
                        ts4_mod_manager_core::mod_scan::ModSource::External => "External",
                    };
                    let state = if entry.enabled { "Installed" } else { "Stored" };
                    let count = entry.mod_files.len();
                    let subtitle = format!(
                        "{origin} · {count} {}",
                        if count == 1 { "mod file" } else { "mod files" }
                    );
                    let selected = catalog.selected.as_ref() == Some(&identity);
                    let response = ui
                        .push_id(identity, |ui| {
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), 64.0),
                                egui::Sense::click(),
                            );
                            let visuals = ui.style().interact_selectable(&response, selected);
                            if selected || response.hovered() || response.has_focus() {
                                ui.painter().rect_filled(rect, 6.0, visuals.bg_fill);
                            }
                            let inner = rect.shrink2(egui::vec2(12.0, 10.0));
                            let thumbnail =
                                egui::Rect::from_min_size(inner.min, egui::Vec2::splat(44.0));
                            self.paint_preview(
                                ui,
                                entry.preview.as_deref(),
                                catalog.root.as_deref(),
                                thumbnail,
                                true,
                            );
                            let mut row = ui.new_child(
                                egui::UiBuilder::new()
                                    .max_rect(egui::Rect::from_min_max(
                                        egui::pos2(inner.left() + 56.0, inner.top()),
                                        egui::pos2(inner.right() - 92.0, inner.bottom()),
                                    ))
                                    .layout(egui::Layout::top_down(egui::Align::Min)),
                            );
                            row.spacing_mut().item_spacing.y = 5.0;
                            row.add(
                                egui::Label::new(
                                    RichText::new(&entry.name).size(15.0).color(foreground),
                                )
                                .truncate(),
                            );
                            let secondary = if selected {
                                ui.visuals().selection.stroke.color
                            } else {
                                ui.visuals().weak_text_color()
                            };
                            row.add(
                                egui::Label::new(
                                    RichText::new(subtitle).size(12.0).color(secondary),
                                )
                                .truncate(),
                            );
                            let mut status = ui.new_child(
                                egui::UiBuilder::new()
                                    .max_rect(egui::Rect::from_min_max(
                                        egui::pos2(inner.right() - 84.0, inner.top()),
                                        inner.max,
                                    ))
                                    .layout(egui::Layout::right_to_left(egui::Align::Center)),
                            );
                            status.label(RichText::new(state).size(12.0).color(secondary));
                            response.widget_info(|| {
                                egui::WidgetInfo::selected(
                                    egui::WidgetType::SelectableLabel,
                                    ui.is_enabled(),
                                    selected,
                                    &entry.name,
                                )
                            });
                            response.on_hover_text(&entry.name)
                        })
                        .inner;
                    if response.secondary_clicked() {
                        catalog.select(index);
                    }
                    if let Some(c) = controls.as_deref_mut() {
                        response.context_menu(|ui| c.mod_menu(ui, catalog));
                    }
                    if self.stats.first_row.is_none() {
                        self.stats.first_row = Some(response.rect);
                    }
                    self.stats.rows += 1;
                    if response.clicked() {
                        catalog.select(index);
                        self.detail_only = ui.ctx().content_rect().width() < 1040.0;
                    }
                }
            });
    }

    fn details(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &Catalog,
        controls: Option<&mut crate::controls::Controls>,
    ) {
        let Some(entry) = catalog.selected_mod() else {
            ui.add_space(24.0);
            ui.heading("Select a mod");
            ui.label("Its files and saved source details will appear here.");
            return;
        };
        if let Some(c) = controls {
            c.selected_actions(ui, catalog);
            ui.add_space(16.0);
        }
        egui::ScrollArea::vertical()
            .id_salt(("detail-summary", EntryId::of(entry)))
            .max_height((ui.available_height() - 150.0).max(80.0))
            .auto_shrink([false, true])
            .show(ui, |ui| {
                ui.add(egui::Label::new(RichText::new(&entry.name).size(23.0).strong()).wrap());
                ui.add_space(12.0);
                let state = if entry.enabled {
                    "Installed in this game folder"
                } else {
                    "Stored in managed storage"
                };
                egui::Frame::new()
                    .fill(ui.visuals().widgets.inactive.weak_bg_fill)
                    .corner_radius(6)
                    .inner_margin(egui::Margin::symmetric(10, 7))
                    .show(ui, |ui| {
                        ui.label(
                            RichText::new(state)
                                .size(12.0)
                                .color(ui.visuals().weak_text_color()),
                        );
                    });
                ui.add_space(24.0);
                let (photo, response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 180.0),
                    egui::Sense::hover(),
                );
                self.paint_preview(
                    ui,
                    entry.preview.as_deref(),
                    catalog.root.as_deref(),
                    photo,
                    false,
                );
                response.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::WidgetType::Image,
                        ui.is_enabled(),
                        format!("Preview of {}", entry.name),
                    )
                });
                ui.add_space(24.0);
                if let Some(source) = &entry.source_url {
                    ui.label(RichText::new("Saved source").size(14.0).strong());
                    if let Some(attachment) = &entry.source_attachment {
                        if let Some(author) = &attachment.author {
                            ui.label(
                                RichText::new(author)
                                    .size(13.0)
                                    .color(ui.visuals().weak_text_color()),
                            );
                        }
                        if attachment
                            .confidence
                            .is_some_and(|confidence| confidence < 70)
                        {
                            ui.label("Low confidence. Verify this source before relying on it.");
                        }
                    }
                    ui.add(
                        egui::Label::new(
                            RichText::new(source)
                                .size(12.0)
                                .color(ui.visuals().weak_text_color()),
                        )
                        .selectable(true)
                        .wrap(),
                    );
                    ui.add_space(24.0);
                }
            });
        ui.add_space(12.0);
        ui.separator();
        ui.add_space(12.0);
        let identity = EntryId::of(entry);
        let now = ui.input(|input| input.time);
        let copied = self
            .copied
            .as_ref()
            .is_some_and(|(id, until)| *id == identity && now < *until);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Files").size(16.0).strong());
            ui.label(
                RichText::new(format!("{}", entry.files.len()))
                    .size(13.0)
                    .color(ui.visuals().weak_text_color()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let icon = if copied { Icon::Check } else { Icon::Copy };
                let copy = ui.add(egui::Button::image_and_text(
                    icon.image(ui.visuals().text_color(), 14.0),
                    if copied { "Copied" } else { "Copy" },
                ));
                self.stats.copy = Some(copy.rect);
                if copy.clicked() {
                    ui.ctx().copy_text(entry.files.join("\n"));
                    self.copied = Some((identity.clone(), now + 2.0));
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_secs(2));
                }
            });
        });
        ui.add_space(12.0);
        egui::ScrollArea::vertical()
            .id_salt(("files", catalog.root.clone(), identity))
            .auto_shrink([false, false])
            .show_rows(ui, 48.0, entry.files.len(), |ui, range| {
                for index in range {
                    let file = &entry.files[index];
                    let (parent, basename) = file.rsplit_once(['/', '\\']).unwrap_or(("", file));
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 48.0),
                        egui::Sense::hover(),
                    );
                    let mut row = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(rect)
                            .layout(egui::Layout::left_to_right(egui::Align::Min)),
                    );
                    row.add(Icon::File.image(ui.visuals().weak_text_color(), 16.0));
                    row.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        ui.add(
                            egui::Label::new(RichText::new(basename).size(13.0))
                                .truncate()
                                .selectable(true),
                        )
                        .on_hover_text(file);
                        if !parent.is_empty() {
                            ui.add(
                                egui::Label::new(
                                    RichText::new(parent)
                                        .size(11.0)
                                        .color(ui.visuals().weak_text_color()),
                                )
                                .truncate(),
                            )
                            .on_hover_text(file);
                        }
                    });
                    self.stats.files += 1;
                }
            });
    }

    fn paint_preview(
        &mut self,
        ui: &egui::Ui,
        preview: Option<&str>,
        root: Option<&std::path::Path>,
        rect: egui::Rect,
        thumbnail: bool,
    ) {
        ui.painter()
            .rect_filled(rect, 6.0, ui.visuals().widgets.inactive.weak_bg_fill);
        match self.previews.load(ui.ctx(), preview, root) {
            crate::preview::State::Ready(texture) => {
                let image = egui::Image::new(texture).corner_radius(6);
                if thumbnail {
                    let uv = cover_uv(texture.size, rect.size());
                    image.uv(uv).paint_at(ui, rect);
                } else {
                    let scale = (rect.width() / texture.size.x).min(rect.height() / texture.size.y);
                    image.paint_at(
                        ui,
                        egui::Rect::from_center_size(rect.center(), texture.size * scale),
                    );
                }
                self.stats.photos += 1;
            }
            crate::preview::State::Pending => {
                egui::Spinner::new().size(18.0).paint_at(
                    ui,
                    egui::Rect::from_center_size(rect.center(), egui::Vec2::splat(18.0)),
                );
            }
            crate::preview::State::Missing => {
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    if thumbnail && rect.width() < 80.0 {
                        "No\nPreview"
                    } else {
                        "No Preview"
                    },
                    egui::FontId::proportional(if thumbnail && rect.width() < 80.0 {
                        9.0
                    } else {
                        13.0
                    }),
                    ui.visuals().weak_text_color(),
                );
                self.stats.missing_photos += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn secondary_text_meets_contrast_on_all_library_surfaces() {
        fn luminance(color: [f32; 3]) -> f32 {
            let linear = color.map(|channel| {
                if channel <= 0.04045 {
                    channel / 12.92
                } else {
                    ((channel + 0.055) / 1.055).powf(2.4)
                }
            });
            linear[0] * 0.2126 + linear[1] * 0.7152 + linear[2] * 0.0722
        }
        let ctx = egui::Context::default();
        setup(&ctx);
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let visuals = &ctx.style_of(theme).visuals;
            let foreground = visuals.weak_text_color().to_array();
            for background in [
                visuals.panel_fill,
                visuals.extreme_bg_color,
                visuals.widgets.inactive.weak_bg_fill,
            ] {
                let background = background.to_array();
                let base = std::array::from_fn(|i| background[i] as f32 / 255.0);
                let painted = std::array::from_fn(|i| {
                    foreground[i] as f32 / 255.0 + base[i] * (1.0 - foreground[3] as f32 / 255.0)
                });
                let a = luminance(base);
                let b = luminance(painted);
                let contrast = (a.max(b) + 0.05) / (a.min(b) + 0.05);
                assert!(
                    contrast >= 4.5,
                    "secondary contrast {contrast:.2} in {theme:?}"
                );
            }
        }
    }
    #[test]
    fn photo_cover_crop_preserves_proportions_in_wide_portrait_and_square_cards() {
        for (source, destination) in [
            (egui::vec2(320.0, 180.0), egui::vec2(300.0, 150.0)),
            (egui::vec2(180.0, 320.0), egui::vec2(300.0, 150.0)),
            (egui::vec2(320.0, 180.0), egui::Vec2::splat(44.0)),
        ] {
            let uv = cover_uv(source, destination);
            let sampled_aspect = source.x * uv.width() / (source.y * uv.height());
            assert!((sampled_aspect - destination.x / destination.y).abs() < 0.001);
            assert!(uv.min.x >= 0.0 && uv.min.y >= 0.0 && uv.max.x <= 1.0 && uv.max.y <= 1.0);
        }
    }

    #[test]
    fn library_defaults_to_photo_cards_and_switches_to_list_without_losing_selection() {
        let mut catalog = populated_catalog();
        let selected = catalog.selected.clone();
        let mut view = View::default();
        let ctx = egui::Context::default();
        setup(&ctx);
        render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        let card = view.stats.first_row.expect("card");
        assert!(card.height() >= 220.0, "default should be photo cards");
        assert!(view.stats.rows < 30);
        let pos = view.stats.list_switch.expect("list switch").center();
        render(
            &ctx,
            &mut view,
            &mut catalog,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                },
            ],
            1200.0,
        );
        render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        assert!(view.stats.first_row.unwrap().height() <= 80.0);
        assert_eq!(catalog.selected, selected);
        assert!(view.stats.search.unwrap().right() <= 1200.0);
    }
    fn populated_catalog() -> Catalog {
        let entries = (0..10_000).map(|index| serde_json::from_value(serde_json::json!({
            "key": format!("Mod{index:05}"), "name": format!("Mod{index:05}"),
            "files": (0..if index == 0 { 10_000 } else { 1 }).map(|n| format!("file{n}.package")).collect::<Vec<_>>(),
            "mod_files": ["file0.package"], "source": "external", "enabled": true, "group_path": ["Pack"]
        })).expect("entry")).collect();
        let mut catalog = Catalog::default();
        let generation = catalog.begin_scan("/fixture/The Sims 4".into());
        catalog.accept_scan(generation, Ok(entries));
        catalog.select(0);
        catalog
    }

    fn render(
        ctx: &egui::Context,
        view: &mut View,
        catalog: &mut Catalog,
        events: Vec<egui::Event>,
        width: f32,
    ) -> egui::FullOutput {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width, 800.0),
            )),
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            view.show(ui, catalog);
        });
        output.textures_delta.clear();
        output
    }

    #[test]
    fn header_logo_preserves_proportions_and_theme_control_at_supported_widths() {
        for width in [640.0, 680.0, 1200.0, 1885.0] {
            for dark in [false, true] {
                let ctx = egui::Context::default();
                setup(&ctx);
                ctx.set_theme(if dark {
                    egui::Theme::Dark
                } else {
                    egui::Theme::Light
                });
                let mut view = View::default();
                let mut catalog = populated_catalog();
                render(&ctx, &mut view, &mut catalog, vec![], width);
                render(&ctx, &mut view, &mut catalog, vec![], width);
                let logo = view
                    .stats
                    .logo
                    .expect("Header must display the supplied logo");
                let button = view
                    .stats
                    .theme_button
                    .expect("Theme control must stay available");
                assert!((logo.width() / logo.height() - 2148.0 / 482.0).abs() < 0.01);
                assert!(logo.left() >= 0.0 && button.right() <= width);
                assert!(!logo.intersects(button));
                render(
                    &ctx,
                    &mut view,
                    &mut catalog,
                    vec![
                        egui::Event::PointerMoved(button.center()),
                        egui::Event::PointerButton {
                            pos: button.center(),
                            button: egui::PointerButton::Primary,
                            pressed: true,
                            modifiers: Default::default(),
                        },
                    ],
                    width,
                );
                render(
                    &ctx,
                    &mut view,
                    &mut catalog,
                    vec![egui::Event::PointerButton {
                        pos: button.center(),
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: Default::default(),
                    }],
                    width,
                );
                assert_eq!(ctx.theme() == egui::Theme::Dark, !dark);
            }
        }
    }
    #[test]
    fn large_catalog_and_file_details_only_render_visible_rows() {
        let mut catalog = populated_catalog();
        let mut view = View::default();
        let ctx = egui::Context::default();
        render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        assert!(view.stats.rows > 0 && view.stats.rows < 30);
        assert!(view.stats.files > 0 && view.stats.files < 40);
        render(&ctx, &mut view, &mut catalog, vec![], 680.0);
        assert!(view.stats.rows < 30);
    }

    #[test]
    fn photo_is_painted_in_catalog_and_details_with_fallback_when_missing() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("preview.png");
        image::RgbaImage::from_pixel(80, 40, image::Rgba([10, 100, 80, 255]))
            .save(&path)
            .unwrap();
        let mut catalog = populated_catalog();
        catalog.mods[0].preview = Some(path.to_string_lossy().into_owned());
        let ctx = egui::Context::default();
        setup(&ctx);
        let mut cache = crate::preview::Previews::default();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let texture = loop {
            if let crate::preview::State::Ready(texture) = cache.load(&ctx, path.to_str(), None) {
                break texture;
            }
            assert!(std::time::Instant::now() < deadline, "photo did not load");
            std::thread::sleep(std::time::Duration::from_millis(5));
        };
        let mut view = View::default();
        render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        let output = render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        let photos = output
            .shapes
            .iter()
            .filter(|shape| shape.shape.texture_id() == texture.id)
            .count();
        assert!(
            photos >= 2,
            "expected thumbnail and detail photo, got {photos}"
        );
        catalog.mods[0].preview = None;
        let output = render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::epaint::Shape::Text(text) if text.galley.text() == "No Preview")));
    }

    #[test]
    fn search_has_a_comfortable_input_target_in_both_layouts() {
        for width in [680.0, 940.0, 1200.0] {
            let mut catalog = populated_catalog();
            let mut view = View::default();
            let ctx = egui::Context::default();
            setup(&ctx);
            render(&ctx, &mut view, &mut catalog, vec![], width);
            render(&ctx, &mut view, &mut catalog, vec![], width);
            let search = view.stats.search.expect("search");
            assert!(search.height() >= 36.0, "search too short: {search:?}");
            assert!(search.right() <= width, "search outside window");
        }
    }

    #[test]
    fn narrow_details_return_to_catalog_and_copy_full_paths() {
        let mut catalog = populated_catalog();
        let mut view = View::default();
        let ctx = egui::Context::default();
        setup(&ctx);
        let click = |pos| {
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                },
            ]
        };
        render(&ctx, &mut view, &mut catalog, vec![], 680.0);
        render(&ctx, &mut view, &mut catalog, vec![], 680.0);
        let row = view.stats.first_row.expect("row").center();
        render(&ctx, &mut view, &mut catalog, click(row), 680.0);
        render(&ctx, &mut view, &mut catalog, vec![], 680.0);
        assert!(view.stats.rows == 0 && view.stats.files > 0);
        let copy = view.stats.copy.expect("copy").center();
        let output = render(&ctx, &mut view, &mut catalog, click(copy), 680.0);
        assert!(output.platform_output.commands.iter().any(|command| matches!(
            command, egui::OutputCommand::CopyText(text) if *text == catalog.selected_mod().unwrap().files.join("\n")
        )));
        assert!(view.copied.is_some());
        let back = view.stats.back.expect("back").center();
        render(&ctx, &mut view, &mut catalog, click(back), 680.0);
        render(&ctx, &mut view, &mut catalog, vec![], 680.0);
        assert!(view.stats.rows > 0 && view.stats.files == 0);
    }

    #[test]
    fn real_search_widget_filters_catalog_after_keyboard_input() {
        let mut catalog = populated_catalog();
        let mut view = View::default();
        let ctx = egui::Context::default();
        render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        let pos = view.stats.search.expect("search").center();
        render(
            &ctx,
            &mut view,
            &mut catalog,
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
                egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: Default::default(),
                },
            ],
            1200.0,
        );
        render(
            &ctx,
            &mut view,
            &mut catalog,
            vec![egui::Event::Text("Mod00042".into())],
            1200.0,
        );
        assert_eq!(catalog.visible.len(), 1);
        assert_eq!(catalog.mods[catalog.visible[0]].name, "Mod00042");
    }
}
