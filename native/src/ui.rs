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
        Clear => lucide "x",
        Activity => lucide "clock",
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
        Compass => "compass",
        Settings => lucide "settings",
        Trash => lucide "trash-2",
        Plus => lucide "plus",
        Download => "download",

    }
}

const CUSTOM_CHROME: &str = "custom-theme-chrome";

#[derive(Clone, Copy)]
pub(crate) struct ChromeColors {
    pub rail: Color32,
    pub rail_text: Color32,
    pub rail_muted: Color32,
    pub rail_selected: Color32,
    pub rail_selected_text: Color32,
    pub primary: Color32,
    pub primary_text: Color32,
    pub switch_on: Color32,
    pub switch_off: Color32,
    pub switch_knob: Color32,
}

pub(crate) fn chrome_colors(ui: &egui::Ui) -> ChromeColors {
    let dark = ui.visuals().dark_mode;
    let custom = ui.ctx().data(|data| data.get_temp::<[Option<ChromeColors>; 2]>(egui::Id::new(CUSTOM_CHROME))).unwrap_or([None; 2]);
    if let Some(colors) = custom[usize::from(dark)] { colors } else {
        ChromeColors {
            rail: if dark { Color32::from_rgb(28, 31, 38) } else { Color32::from_rgb(8, 99, 185) },
            rail_text: Color32::WHITE,
            rail_muted: if dark { Color32::from_rgb(172, 180, 192) } else { Color32::from_rgb(218, 235, 252) },
            rail_selected: if dark { Color32::from_rgb(44, 60, 85) } else { Color32::WHITE },
            rail_selected_text: if dark { Color32::from_rgb(184, 210, 255) } else { Color32::from_rgb(9, 87, 162) },
            primary: if dark { Color32::from_rgb(55, 116, 214) } else { Color32::from_rgb(18, 99, 202) },
            primary_text: Color32::WHITE,
            switch_on: if dark { Color32::from_rgb(78, 132, 81) } else { Color32::from_rgb(66, 126, 22) },
            switch_off: if dark { Color32::from_rgb(87, 96, 110) } else { Color32::from_rgb(166, 185, 199) },
            switch_knob: Color32::WHITE,
        }
    }
}

pub(crate) struct SearchField {
    pub input: egui::Response,
    pub submit: Option<egui::Response>,
    pub clear: Option<egui::Rect>,
}

pub(crate) fn search_field(
    ui: &mut egui::Ui,
    query: &mut String,
    hint: &str,
    id: &str,
    focus: bool,
    with_submit: bool,
) -> SearchField {
    let fill = ui.visuals().extreme_bg_color;
    let border = ui.visuals().widgets.noninteractive.bg_stroke;
    let accent = ui.visuals().hyperlink_color;
    let frame = egui::Frame::new()
        .fill(fill)
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(12, 6))
        .show(ui, |ui| {
            text_on_fill(ui, fill);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                ui.add(Icon::Search.image(ui.visuals().weak_text_color(), 18.0));
                let clear_visible = !query.is_empty();
                let shortcut = !clear_visible && ui.available_width() > 500.0;
                let trailing = if with_submit { 98.0 } else { 0.0 }
                    + if clear_visible { 42.0 } else if shortcut { 52.0 } else { 0.0 };
                let mut input = ui.add_sized(
                    [(ui.available_width() - trailing).max(60.0), 36.0],
                    egui::TextEdit::singleline(query)
                        .id_salt(id)
                        .text_color(readable_text(ui.visuals().text_color(), &[fill]))
                        .hint_text(RichText::new(hint).color(readable_text(ui.visuals().weak_text_color(), &[fill])))
                        .frame(egui::Frame::NONE.inner_margin(egui::Margin::symmetric(0, 9))),
                );
                if focus { input.request_focus(); }
                let mut clear = None;
                if clear_visible {
                    let response = ui.add_sized([32.0, 32.0],
                        egui::Button::image(Icon::Clear.image(ui.visuals().weak_text_color(), 16.0)).frame(false))
                        .on_hover_text("Clear search");
                    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), "Clear search"));
                    clear = Some(response.rect);
                    if response.clicked() {
                        query.clear();
                        input.mark_changed();
                        input.request_focus();
                    }
                } else if shortcut {
                    ui.add_sized([42.0, 32.0], egui::Label::new(RichText::new("Ctrl+F").size(11.0).color(ui.visuals().weak_text_color())));
                }
                let submit = with_submit.then(|| {
                    let colors = chrome_colors(ui);
                    let button = ui.add_sized([88.0, 36.0], egui::Button::new(RichText::new("Search").color(colors.primary_text))
                        .fill(colors.primary).stroke(egui::Stroke::NONE));
                    if button.hovered() || button.has_focus() {
                        ui.painter().rect_stroke(button.rect, 9, egui::Stroke::new(1.5, accent), egui::StrokeKind::Inside);
                    }
                    button
                });
                SearchField { input, submit, clear }
            }).inner
        });
    let focused = frame.inner.input.has_focus() || focus;
    let stroke = if focused { egui::Stroke::new(1.5, accent) }
        else if frame.response.hovered() { egui::Stroke::new(1.0, blend(border.color, accent, 0.35)) }
        else { border };
    ui.painter().rect_stroke(frame.response.rect, 10, stroke, egui::StrokeKind::Inside);
    frame.inner
}

fn rail_button(ui: &mut egui::Ui, icon: Icon, label: &str, active: bool) -> egui::Response {
    let colors = chrome_colors(ui);
    let ink = if active { colors.rail_selected_text } else { colors.rail_text };
    ui.scope(|ui| {
        let v = ui.visuals_mut();
        for widget in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active] {
            widget.fg_stroke.color = ink;
        }
        ui.add_sized([ui.available_width(), 46.0], egui::Button::image_and_text(icon.image(ink, 19.0), RichText::new(label).color(ink).strong())
            .fill(if active { colors.rail_selected } else { Color32::TRANSPARENT }).stroke(egui::Stroke::NONE))
    }).inner
}

fn mod_switch(ui: &mut egui::Ui, enabled: bool, name: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(42.0, 28.0), egui::Sense::click());
    let track = egui::Rect::from_center_size(rect.center(), egui::vec2(40.0, 24.0));
    let colors = chrome_colors(ui);
    let fill = if !ui.is_enabled() { ui.visuals().widgets.noninteractive.bg_stroke.color }
        else if enabled { colors.switch_on } else { colors.switch_off };
    ui.painter().rect_filled(track, 12, fill);
    let x = if enabled { track.right() - 12.0 } else { track.left() + 12.0 };
    ui.painter().circle_filled(egui::pos2(x, track.center().y), 9.0, colors.switch_knob);
    if response.has_focus() { ui.painter().rect_stroke(rect.expand(2.0), 12, ui.visuals().selection.stroke, egui::StrokeKind::Outside); }
    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, ui.is_enabled(), enabled, format!("{name}: {}", if enabled { "On" } else { "Off" })));
    response.on_hover_text(if enabled { "On in game. Click to turn off." } else { "Off in game. Click to turn on." })
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
    ctx.data_mut(|data| data.insert_temp(egui::Id::new(CUSTOM_CHROME), [None::<ChromeColors>; 2]));
    let rendering = fastframe_text::detect();
    ctx.all_styles_mut(|style| {
        let colors = crate::settings::ThemeColors::base(style.visuals.dark_mode);
        let v = &mut style.visuals;
        v.panel_fill = hex_color(&colors.background).unwrap();
        v.extreme_bg_color = hex_color(&colors.surface).unwrap();
        v.window_fill = v.extreme_bg_color;
        v.faint_bg_color = v.extreme_bg_color;
        v.override_text_color = Some(hex_color(&colors.text).unwrap());
        v.weak_text_color = Some(hex_color(&colors.muted_text).unwrap());
        v.selection.bg_fill = if v.dark_mode { Color32::from_rgb(41, 62, 50) } else { Color32::from_rgb(223, 236, 203) };
        v.selection.stroke.color = if v.dark_mode { Color32::from_rgb(163, 214, 172) } else { Color32::from_rgb(46, 87, 16) };
        v.hyperlink_color = hex_color(&colors.accent).unwrap();
        v.window_stroke.color = hex_color(&colors.border).unwrap();
        v.widgets.noninteractive.bg_stroke.color = v.window_stroke.color;
        v.widgets.noninteractive.fg_stroke.color = v.weak_text_color();
        for (widget, fill) in [
            (&mut v.widgets.inactive, if v.dark_mode { hex_color(&colors.surface).unwrap() } else { Color32::WHITE }),
            (&mut v.widgets.hovered, if v.dark_mode { Color32::from_rgb(45, 50, 60) } else { Color32::from_rgb(225, 237, 245) }),
            (&mut v.widgets.active, if v.dark_mode { Color32::from_rgb(52, 59, 71) } else { v.selection.bg_fill }),
            (&mut v.widgets.open, if v.dark_mode { Color32::from_rgb(45, 50, 60) } else { Color32::from_rgb(225, 237, 245) }),
        ] {
            widget.corner_radius = egui::CornerRadius::same(9);
            widget.bg_fill = fill;
            widget.weak_bg_fill = fill;
            widget.bg_stroke.color = v.window_stroke.color;
            widget.fg_stroke.color = v.override_text_color.unwrap();
        }
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(14.0, 10.0);
        style.spacing.interact_size.y = 36.0;
        rendering.apply_to_visuals(v);
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
        let mut custom_chrome = [None; 2];
        ctx.all_styles_mut(|style| {
            let colors = theme.palette(style.visuals.dark_mode);
            if let Some(visuals) = custom_palette_visuals(&style.visuals, colors) {
                custom_chrome[usize::from(style.visuals.dark_mode)] = Some(custom_palette_chrome(colors, &visuals));
                style.visuals = visuals;
            }
        });
        ctx.data_mut(|data| data.insert_temp(egui::Id::new(CUSTOM_CHROME), custom_chrome));
    }
}

fn blend(a: Color32, b: Color32, amount: f32) -> Color32 {
    Color32::from_rgb(
        (f32::from(a.r()) * (1.0 - amount) + f32::from(b.r()) * amount) as u8,
        (f32::from(a.g()) * (1.0 - amount) + f32::from(b.g()) * amount) as u8,
        (f32::from(a.b()) * (1.0 - amount) + f32::from(b.b()) * amount) as u8,
    )
}

pub(crate) fn readable_text(requested: Color32, backgrounds: &[Color32]) -> Color32 {
    let luminance = |color: Color32| {
        let channels = [color.r(), color.g(), color.b()].map(|value| {
            let value = f32::from(value) / 255.0;
            if value <= 0.04045 { value / 12.92 } else { ((value + 0.055) / 1.055).powf(2.4) }
        });
        channels[0] * 0.2126 + channels[1] * 0.7152 + channels[2] * 0.0722
    };
    let contrast = |ink: Color32| backgrounds.iter().map(|background| {
        let a = luminance(ink); let b = luminance(*background);
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }).fold(f32::INFINITY, f32::min);
    if contrast(requested) >= 4.5 { return requested; }
    let destination = if contrast(Color32::BLACK) >= contrast(Color32::WHITE) { Color32::BLACK } else { Color32::WHITE };
    for step in 1..=100 {
        let ink = blend(requested, destination, step as f32 / 100.0);
        if contrast(ink) >= 4.5 { return ink; }
    }
    destination
}

pub(crate) fn custom_palette_visuals(base: &egui::Visuals, colors: &crate::settings::ThemeColors) -> Option<egui::Visuals> {
    let [accent, background, surface, text, muted, border] = [
        &colors.accent, &colors.background, &colors.surface, &colors.text, &colors.muted_text, &colors.border,
    ].map(|value| hex_color(value));
    let (accent, background, surface, text, muted, border) = (accent?, background?, surface?, text?, muted?, border?);
    let fills = [0.06, 0.13, 0.20].map(|amount| blend(surface, accent, amount));
    let mut v = base.clone();
    v.panel_fill = background; v.extreme_bg_color = surface; v.window_fill = surface;
    v.faint_bg_color = surface; v.code_bg_color = surface;
    v.override_text_color = None;
    v.text_edit_bg_color = Some(surface);
    v.weak_text_color = Some(readable_text(muted, &[background]));
    v.selection.bg_fill = fills[2]; v.selection.stroke.color = readable_text(text, &[fills[2]]);
    v.hyperlink_color = readable_text(accent, &[background]);
    v.window_stroke.color = border; v.widgets.noninteractive.bg_stroke.color = border;
    v.widgets.noninteractive.fg_stroke.color = readable_text(text, &[background]);
    for (widget, fill) in [(&mut v.widgets.inactive, fills[0]), (&mut v.widgets.hovered, fills[1]), (&mut v.widgets.active, fills[2]), (&mut v.widgets.open, fills[1])] {
        widget.bg_fill = fill; widget.weak_bg_fill = fill; widget.bg_stroke.color = border;
        widget.fg_stroke.color = readable_text(text, &[fill]);
    }
    Some(v)
}

pub(crate) fn custom_palette_chrome(colors: &crate::settings::ThemeColors, visuals: &egui::Visuals) -> ChromeColors {
    let accent = hex_color(&colors.accent).expect("validated palette");
    let surface = visuals.extreme_bg_color;
    let text = visuals.override_text_color.unwrap_or(visuals.text_color());
    let muted = visuals.weak_text_color();
    ChromeColors {
        rail: surface, rail_text: readable_text(text, &[surface]), rail_muted: readable_text(muted, &[surface]),
        rail_selected: visuals.selection.bg_fill, rail_selected_text: readable_text(text, &[visuals.selection.bg_fill]),
        primary: accent, primary_text: readable_text(surface, &[accent]),
        switch_on: accent, switch_off: muted, switch_knob: readable_text(surface, &[accent, muted]),
    }
}

pub(crate) fn text_on_fill(ui: &mut egui::Ui, fill: Color32) {
    if ui.visuals().text_edit_bg_color.is_none() { return; }
    let text = readable_text(ui.visuals().text_color(), &[fill]);
    let muted = readable_text(ui.visuals().weak_text_color(), &[fill]);
    let link = readable_text(ui.visuals().hyperlink_color, &[fill]);
    let v = ui.visuals_mut();
    v.override_text_color = None;
    v.widgets.noninteractive.fg_stroke.color = text;
    v.weak_text_color = Some(muted);
    v.hyperlink_color = link;
}

pub(crate) fn choice_label(ui: &mut egui::Ui, selected: bool, text: impl Into<egui::WidgetText>) -> egui::Response {
    ui.scope(|ui| {
        if ui.visuals().text_edit_bg_color.is_some() {
            ui.visuals_mut().widgets.inactive.fg_stroke.color = ui.visuals().text_color();
        }
        ui.selectable_label(selected, text.into())
    }).inner
}

pub(crate) fn choice_value<Value: PartialEq>(ui: &mut egui::Ui, current: &mut Value, value: Value, text: impl Into<egui::WidgetText>) -> egui::Response {
    let mut response = choice_label(ui, *current == value, text);
    if response.clicked() && *current != value {
        *current = value;
        response.mark_changed();
    }
    response
}

pub(crate) fn cover_uv(source: egui::Vec2, destination: egui::Vec2) -> egui::Rect {
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
    pub browse_page: bool,
    updates_page: bool,
    pub discovery: crate::discovery::DiscoverView,
    detail_open: Option<EntryId>,
    editing_root: bool,
    help_open: bool,
    copied: Option<(EntryId, f64)>,
    source_copied: Option<(EntryId, f64)>,
    previews: crate::preview::Previews,
    layout: LibraryLayout,
}

#[derive(Default)]
pub struct RenderStats {
    pub logo: Option<egui::Rect>,
    pub browse_tab: Option<egui::Rect>,
    pub library_tab: Option<egui::Rect>,
    pub theme_button: Option<egui::Rect>,
    pub updates_tab: Option<egui::Rect>,
    pub first_toggle: Option<egui::Rect>,
    pub rows: usize,
    pub skeleton_cards: usize,
    pub files: usize,
    pub search: Option<egui::Rect>,
    pub search_clear: Option<egui::Rect>,
    pub first_row: Option<egui::Rect>,
    pub detail_close: Option<egui::Rect>,
    pub copy: Option<egui::Rect>,
    pub photos: usize,
    pub pending_photos: usize,
    pub missing_photos: usize,
    pub list_switch: Option<egui::Rect>,
}

impl View {
    pub fn with_cache(directory: PathBuf) -> Self {
        Self {
            previews: crate::preview::Previews::with_cache(directory.clone()),
            discovery: crate::discovery::DiscoverView::with_cache(directory),
            ..Self::default()
        }
    }

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
        let narrow = ui.available_width() < 900.0;
        let foreground = ui.visuals().text_color();
        let review_open = controls.as_ref().is_some_and(|c| c.review_open() || c.busy());
        if !self.help_open
            && !egui::Popup::is_any_open(ui.ctx())
            && let Some(c) = controls.as_deref_mut().filter(|c| c.settings_page && !c.review_open())
            && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
        {
            c.settings_page = false;
            ui.memory_mut(|memory| {
                if let Some(id) = memory.focused() {
                    memory.surrender_focus(id);
                }
            });
        }
        let search_requested = !review_open && !controls.as_ref().is_some_and(|c| c.settings_page)
            && ui.input_mut(|input| input.consume_key(egui::Modifiers::CTRL, egui::Key::F));
        if search_requested { self.detail_open = None; self.help_open = false; self.updates_page = false; }
        if !review_open && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::F1)) { self.help_open = true; }
        if !narrow {
            let colors = chrome_colors(ui);
            egui::Panel::left("library-navigation").exact_size(240.0).resizable(false)
                .frame(egui::Frame::new().fill(colors.rail).inner_margin(24))
                .show(ui, |ui| {
                    ui.visuals_mut().override_text_color = Some(colors.rail_text);
                    ui.visuals_mut().weak_text_color = Some(colors.rail_muted);
                    ui.horizontal(|ui| { self.brand(ui); });
                    ui.add_space(28.0);
                    self.destinations(ui, &mut controls, true);
                    ui.add_space(20.0);
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
                        if let Some(c) = controls.as_deref_mut() {
                            if rail_button(ui, Icon::Settings, "Settings", c.settings_page).clicked() { c.settings_page = !c.settings_page; }
                            if rail_button(ui, Icon::Trash, "Trash", false).clicked() { c.open_trash(catalog); }
                        }
                    });
                });
        }
        egui::Panel::top("catalog-header")
            .resizable(false)
            .frame(
                egui::Frame::new()
                    .fill(ui.visuals().panel_fill)
                    .inner_margin(egui::Margin::symmetric(24, 10)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    if narrow { self.brand(ui); }
                    else if let Some(root) = catalog.root.as_ref()
                        && ui.add(egui::Button::image_and_text(Icon::Folder.image(foreground, 18.0), "The Sims 4")).on_hover_text(root.display().to_string()).clicked() {
                            if let Some(c) = controls.as_deref_mut() { c.choose_game_folder(ui.ctx(), Some(root.clone())); }
                            else { self.editing_root = true; self.root_input = root.display().to_string(); }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if narrow && let Some(c) = controls.as_deref_mut()
                            && ui.selectable_label(c.settings_page, "Settings").clicked()
                        {
                            c.settings_page = !c.settings_page;
                        }
                        if narrow && let Some(root) = catalog.root.as_ref()
                            && ui.add(egui::Button::image(Icon::Folder.image(foreground, 18.0))).on_hover_text(format!("Change game folder: {}", root.display())).clicked() {
                                if let Some(c) = controls.as_deref_mut() { c.choose_game_folder(ui.ctx(), Some(root.clone())); }
                                else { self.editing_root = true; self.root_input = root.display().to_string(); }
                        }
                        if narrow && let Some(c) = controls.as_deref_mut()
                            && ui.add(egui::Button::image(Icon::Trash.image(foreground, 18.0))).on_hover_text("Restore from trash").clicked() { c.open_trash(catalog); }
                        if let Some(c) = controls.as_deref_mut() {
                            c.app_updates.banner(ui);
                        }
                        if let Some(c) = controls.as_deref_mut() { c.activity_header(ui, catalog.loading, narrow); }
                        if ui.button("Help").on_hover_text("Tasks and keyboard shortcuts (F1)").clicked() { self.help_open = true; }
                        if !controls.as_ref().is_some_and(|c| c.settings_page) {
                        let dark = ui.visuals().dark_mode;
                        let icon = if dark { Icon::Sun } else { Icon::Moon };
                        let label = if dark { "Light" } else { "Dark" };
                        let theme_button = ui.add(egui::Button::image_and_text(
                            icon.image(foreground, 16.0),
                            if narrow { "" } else { label },
                        )).on_hover_text(format!("Switch to {label} appearance"));
                        theme_button.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label));
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
                        }
                    });
                });
                if narrow { ui.horizontal_wrapped(|ui| self.destinations(ui, &mut controls, false)); }
                if self.browse_page || self.updates_page || (!self.editing_root && catalog.root.is_some()) { return; }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add(Icon::Folder.image(ui.visuals().weak_text_color(), 18.0));
                    match catalog.root.as_ref().filter(|_| !self.editing_root) {
                        None => {
                            if let Some(c) = controls.as_deref_mut()
                                && ui
                                    .add_enabled(
                                        !c.picker_open(),
                                        egui::Button::new("Choose folder"),
                                    )
                                    .clicked()
                            {
                                c.choose_game_folder(ui.ctx(), None);
                            }
                            let response = ui.add_sized(
                                [(ui.available_width() - 80.0).max(100.0), 36.0],
                                egui::TextEdit::singleline(&mut self.root_input).text_color(crate::ui::readable_text(ui.visuals().text_color(), &[ui.visuals().text_edit_bg_color()]))
                                    .hint_text(egui::RichText::new("Game folder containing Mods").color(crate::ui::readable_text(ui.visuals().weak_text_color(), &[ui.visuals().text_edit_bg_color()])))
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
                                .take(if narrow { 36 } else { 64 })
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

        let settings_page = controls.as_ref().is_some_and(|c| c.settings_page);
        egui::CentralPanel::default().frame(egui::Frame::new().fill(ui.visuals().panel_fill).inner_margin(24))
            .show(ui, |ui| {
                if settings_page {
                    if let Some(c) = controls.as_deref_mut() { c.settings_content(ui, catalog, if self.browse_page { "Back to browsing" } else { "Back to library" }); }
                    return;
                }
                if self.updates_page {
                    if let Some(c) = controls.as_deref_mut()
                        && let Some(index) = c.updates_content(ui, catalog) { catalog.select(index); self.detail_open = catalog.selected.clone(); }
                    return;
                }
                if self.browse_page {
                    let key = controls.as_ref().and_then(|c| c.source_api_key());
                    self.discovery.download_labels = controls.as_ref().map(|c| c.download_labels()).unwrap_or_default();
                    let settings_requested = self.discovery.show(ui, key, search_requested, catalog);
                    if let Some(c) = controls.as_deref_mut() && let Some(request) = self.discovery.take_import_request() { c.prepare_browser_import(ui.ctx(), catalog, request); }
                    if let Some(identity) = self.discovery.take_library_request()
                        && let Some(index) = catalog.mods.iter().position(|entry| EntryId::of(entry) == identity) {
                        catalog.search(String::new());
                        catalog.set_filter(ModFilter::All);
                        catalog.select(index);
                        self.detail_open = catalog.selected.clone();
                        self.browse_page = false;
                    }
                    if settings_requested && let Some(c) = controls.as_deref_mut() { c.settings_page = true; }
                    return;
                }
                if let Some(error) = &catalog.error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                    ui.label("Check that the game folder exists and contains a readable Mods folder, then open it again.");
                    ui.add_space(16.0);
                }
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Your mods").size(32.0).strong().color(ui.visuals().text_color()));
                    ui.label(RichText::new(format!("{} mods", catalog.visible.len())).size(14.0).color(ui.visuals().weak_text_color()));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if let Some(c) = controls.as_deref_mut() { c.add_mods_button(ui, catalog); }
                        if ui.add_enabled(catalog.root.is_some() && !catalog.loading,
                            egui::Button::image(Icon::Refresh.image(foreground, 18.0))).on_hover_text("Rescan mods").clicked() {
                            scan_root = catalog.root.clone();
                        }
                    });
                });
                ui.add_space(12.0);
                if let Some(c) = controls.as_deref_mut() { c.library_toolbar(ui, catalog); if c.update_summary(ui, true) { self.updates_page = true; } ui.add_space(12.0); }
                let mut query = catalog.query.clone();
                let field = search_field(ui, &mut query, "Search mods", "library-search", search_requested, false);
                self.stats.search = Some(field.input.rect);
                self.stats.search_clear = field.clear;
                if field.input.changed() { catalog.search(query); }
                ui.add_space(16.0);
                ui.horizontal_wrapped(|ui| {
                    for (filter, label) in [(ModFilter::All, "All"), (ModFilter::Installed, "On"), (ModFilter::Stored, "Off")] {
                        if ui.add(egui::Button::new(label).selected(catalog.installation_filter == filter)).on_hover_text(filter.label()).clicked() { catalog.set_filter(filter); }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let list = ui.add(egui::Button::image_and_text(Icon::List.image(foreground, 18.0), "").selected(self.layout == LibraryLayout::List)).on_hover_text("List view");
                        list.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), "List"));
                        self.stats.list_switch = Some(list.rect);
                        if list.clicked() { self.layout = LibraryLayout::List; }
                        let cards = ui.add(egui::Button::image(Icon::Grid.image(foreground, 18.0)).selected(self.layout == LibraryLayout::Cards)).on_hover_text("Card view");
                        cards.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), "Cards"));
                        if cards.clicked() { self.layout = LibraryLayout::Cards; }
                    });
                });
                ui.add_space(12.0);
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
                    if catalog.loading {
                        ui.add_space(16.0);
                        self.stats.skeleton_cards = crate::skeleton::cards(ui);
                    }
                    return;
                }
                match self.layout {
                    LibraryLayout::Cards if ui.available_height() >= 220.0 => self.card_grid(ui, catalog, controls.as_deref_mut()),
                    LibraryLayout::Cards => self.library_list(ui, catalog, controls.as_deref_mut()),
                    LibraryLayout::List => self.library_list(ui, catalog, controls.as_deref_mut()),
                }

            });
        if self.detail_open != catalog.selected || catalog.selected_mod().is_none() {
            self.detail_open = None;
        }
        if !settings_page
            && !self.browse_page
            && !self.help_open
            && self.detail_open.is_some()
            && !controls
                .as_ref()
                .is_some_and(|c| c.review_open() || c.busy())
        {
            self.details_popup(ui.ctx(), catalog, controls.as_deref_mut());
        }
        if self.help_open && !review_open {
            let popup = crate::popup::show(ui.ctx(), "task-help", "Using your mod library", |ui| {
                egui::ScrollArea::vertical().max_height(crate::popup::body_height(ui.ctx())).show(ui, |ui| {
                    for (title, body) in [
                        ("Add mods", "Choose your Sims 4 game folder, then Import folder or Import ZIP. New mods start enabled. Use the library switch to turn a mod on or off. Originals stay in place."),
                        ("Inspect and organize", "Click a card or list row for details. Use Previous/Next to browse. Right-click a mod for rename, source and trash actions. Trash remains recoverable."),
                        ("Browse CurseForge", "Browse mods searches The Sims 4 mods and custom content on CurseForge. Save your API key in Settings, then search, filter by category or sort. Click a card for details or choose Download in browser to open the download page. This saves the selected source and installs the verified file in your selected game. Use Activity to inspect or cancel the task."),
                        ("Save a source", "Find source uses your locally saved CurseForge key. No match? Attach a known URL manually. Review every attachment before saving."),
                        ("Check updates", "Attach a CurseForge source, then choose Check updates. Record the release you installed to compare versions. Update notices link to the mod page; download there, replace your local files, and record the new release."),
                        ("Change appearance", "Settings shows a preview. Save changes applies your choices; Discard changes restores saved values."),
                        ("Keyboard shortcuts", "Ctrl+F searches · Alt+Left/Right browses open mod details · Tab moves focus · Enter/Space activates buttons · Escape goes back one level · F1 opens Help."),
                    ] {
                        ui.label(RichText::new(title).strong().color(ui.visuals().text_color())); ui.label(body); ui.add_space(12.0);
                    }
                });
                crate::popup::close(ui).clicked()
            });
            if popup.inner || popup.should_close() { self.help_open = false; }
        }
        if let Some(c) = controls { c.notifications(ui.ctx()); }
        scan_root
    }

    fn brand(&mut self, ui: &mut egui::Ui) {
        let logo = ui.add(egui::Image::new(egui::include_image!("../assets/plumbob.svg")).fit_to_exact_size(egui::vec2(22.0, 34.0)).alt_text("Sims 4 Mod Manager"));
        self.stats.logo = Some(logo.rect);
        ui.label(RichText::new("Mod Manager").size(18.0).strong().color(ui.visuals().text_color()));
    }

    fn destinations(&mut self, ui: &mut egui::Ui, controls: &mut Option<&mut crate::controls::Controls>, rail: bool) {
        let settings = controls.as_ref().is_some_and(|c| c.settings_page);
        for (icon, label, active) in [
            (Icon::Grid, "Your mods", !self.browse_page && !self.updates_page && !settings),
            (Icon::Compass, "Discover", self.browse_page && !settings),
            (Icon::Refresh, "Updates", self.updates_page && !settings),
        ] {
            let response = if rail { rail_button(ui, icon, label, active) } else {
                ui.add(egui::Button::image_and_text(icon.image(ui.visuals().text_color(), 18.0), label).selected(active))
            };
            if label == "Your mods" { self.stats.library_tab = Some(response.rect); }
            if label == "Discover" { self.stats.browse_tab = Some(response.rect); }
            if label == "Updates" { self.stats.updates_tab = Some(response.rect); }
            if response.clicked() {
                self.browse_page = label == "Discover";
                self.updates_page = label == "Updates";
                self.detail_open = None;
                if let Some(c) = controls.as_deref_mut() { c.settings_page = false; }
            }
            if rail { ui.add_space(4.0); }
        }
    }

    fn card_grid(
        &mut self,
        ui: &mut egui::Ui,
        catalog: &mut Catalog,
        mut controls: Option<&mut crate::controls::Controls>,
    ) {
        let gap = 16.0;
        let columns = ((ui.available_width() + gap) / 236.0).floor().max(1.0) as usize;
        let width = (ui.available_width() - gap * (columns - 1) as f32) / columns as f32;
        let photo_height = if catalog.visible.iter().any(|index| catalog.mods[*index].preview.is_some()) { 140.0 } else { 88.0 };
        let height = photo_height + 98.0;
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
                            let mut toggle_requested = false;
                            let response = ui
                                .push_id(&identity, |ui| {
                                    ui.style_mut().interaction.selectable_labels = false;
                                    let (rect, response) = ui.allocate_exact_size(
                                        egui::vec2(width, height),
                                        egui::Sense::click(),
                                    );
                                    let color = if response.hovered() {
                                        ui.visuals().widgets.hovered.weak_bg_fill
                                    } else {
                                        ui.visuals().extreme_bg_color
                                    };
                                    text_on_fill(ui, color);
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
                                        rect.min + egui::vec2(0.0, 0.0),
                                        egui::pos2(rect.right(), rect.top() + photo_height),
                                    );
                                    self.paint_preview(
                                        ui,
                                        entry.preview.as_deref(),
                                        catalog.root.as_deref(),
                                        image_rect,
                                        true,
                                    );
                                    if entry.id.as_ref().is_some_and(|id| controls.as_ref().is_some_and(|c| c.updates.available(id))) {
                                        let badge = egui::Rect::from_min_size(image_rect.min + egui::vec2(6.0, 6.0), egui::vec2(132.0, 24.0));
                                        ui.painter().rect_filled(badge, 4, ui.visuals().selection.bg_fill);
                                        ui.painter().text(badge.center(), egui::Align2::CENTER_CENTER, "Update available", egui::FontId::proportional(12.0), ui.visuals().selection.stroke.color);
                                    }
                                    let title_rect = egui::Rect::from_min_max(
                                        rect.min + egui::vec2(14.0, photo_height + 22.0),
                                        rect.max - egui::vec2(14.0, 46.0),
                                    );
                                    let mut text = ui.new_child(
                                        egui::UiBuilder::new()
                                            .max_rect(title_rect)
                                            .layout(egui::Layout::top_down(egui::Align::Min)),
                                    );
                                    text.add(
                                        egui::Label::new(
                                            RichText::new(&entry.name).size(15.0).strong().color(ui.visuals().text_color()),
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
                                            let enabled = entry.enabled;
                                            let toggle = ui.add_enabled_ui(controls.as_ref().is_some_and(|c| !c.busy()) && entry.id.is_some() && !catalog.loading,
                                                |ui| mod_switch(ui, enabled, &entry.name)).inner;
                                            if self.stats.first_toggle.is_none() { self.stats.first_toggle = Some(toggle.rect); }
                                            if toggle.clicked() { toggle_requested = true; }

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
                            if toggle_requested {
                                catalog.select(index);
                                if let Some(c) = controls.as_deref_mut() { c.toggle_selected(catalog); }
                            }
                            if response.secondary_clicked() {
                                catalog.select(index);
                            }
                            if let Some(c) = controls.as_deref_mut() {
                                response.context_menu(|ui| { text_on_fill(ui, ui.visuals().window_fill); c.mod_menu(ui, catalog) });
                            }
                            if self.stats.first_row.is_none() {
                                self.stats.first_row = Some(response.rect.intersect(ui.clip_rect()));
                            }
                            self.stats.rows += 1;
                            if response.clicked() && !toggle_requested {
                                catalog.select(index);
                                self.detail_open = catalog.selected.clone();
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
        let compact = ui.available_height() < 64.0;
        let row_height = if compact { 40.0 } else { 64.0 };
        let root_id = catalog.root.clone();
        egui::ScrollArea::vertical()
            .id_salt(("catalog", root_id))
            .auto_shrink([false, false])
            .show_rows(ui, row_height, catalog.visible.len(), |ui, range| {
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
                    let mut subtitle = format!(
                        "{origin} · {count} {}",
                        if count == 1 { "mod file" } else { "mod files" }
                    );
                    if entry.id.as_ref().is_some_and(|id| controls.as_ref().is_some_and(|c| c.updates.available(id))) { subtitle.push_str(" · Update available"); }
                    let selected = catalog.selected.as_ref() == Some(&identity);
                    let mut toggle_requested = false;
                    let response = ui
                        .push_id(identity, |ui| {
                            let (rect, response) = ui.allocate_exact_size(
                                egui::vec2(ui.available_width(), row_height),
                                egui::Sense::click(),
                            );
                            let visuals = ui.style().interact_selectable(&response, selected);
                            if selected || response.hovered() || response.has_focus() {
                                ui.painter().rect_filled(rect, 6.0, visuals.bg_fill);
                            }
                            let fill = if selected || response.hovered() || response.has_focus() { visuals.bg_fill } else { ui.visuals().panel_fill };
                            text_on_fill(ui, fill);
                            let inner = rect.shrink2(egui::vec2(12.0, if compact {4.0} else {10.0}));
                            let thumbnail =
                                egui::Rect::from_min_size(inner.min, egui::Vec2::splat(inner.height().min(44.0)));
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
                            row.spacing_mut().item_spacing.y = if compact {0.0} else {5.0};
                            row.add(
                                egui::Label::new(
                                    RichText::new(&entry.name).size(15.0).color(ui.visuals().text_color()),
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
                            if entry.id.is_some() {
                                let toggle = status.add_enabled_ui(controls.as_ref().is_some_and(|c| !c.busy()) && !catalog.loading,
                                    |ui| mod_switch(ui, entry.enabled, &entry.name)).inner;
                                if self.stats.first_toggle.is_none() { self.stats.first_toggle = Some(toggle.rect); }
                                toggle_requested = toggle.clicked();
                            } else {
                                status.label(RichText::new(state).size(12.0).color(secondary));
                            }
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
                    if toggle_requested {
                        catalog.select(index);
                        if let Some(c) = controls.as_deref_mut() { c.toggle_selected(catalog); }
                    }
                    if response.secondary_clicked() {
                        catalog.select(index);
                    }
                    if let Some(c) = controls.as_deref_mut() {
                        response.context_menu(|ui| { text_on_fill(ui, ui.visuals().window_fill); c.mod_menu(ui, catalog) });
                    }
                    if self.stats.first_row.is_none() {
                        self.stats.first_row = Some(response.rect.intersect(ui.clip_rect()));
                    }
                    self.stats.rows += 1;
                    if response.clicked() && !toggle_requested {
                        catalog.select(index);
                        self.detail_open = catalog.selected.clone();
                    }
                }
            });
    }

    fn details_popup(
        &mut self,
        ctx: &egui::Context,
        catalog: &mut Catalog,
        mut controls: Option<&mut crate::controls::Controls>,
    ) {
        let Some(entry) = catalog.selected_mod() else {
            return;
        };
        let title = entry.name.clone();
        let position = catalog
            .visible
            .iter()
            .position(|index| Some(EntryId::of(&catalog.mods[*index])) == catalog.selected);
        let mut direction = 0;
        if !egui::Popup::is_any_open(ctx) {
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::ALT, egui::Key::ArrowLeft))
            {
                direction = -1;
            }
            if ctx.input_mut(|input| input.consume_key(egui::Modifiers::ALT, egui::Key::ArrowRight))
            {
                direction = 1;
            }
        }
        let popup = crate::popup::show_width(ctx, "mod-details", &title, 600.0, |ui| {
            if let Some(c) = controls.as_deref_mut() {
                c.selected_actions(ui, catalog);
                ui.add_space(8.0);
            }
            let height =
                (ctx.content_rect().height() - ui.min_rect().height() - 164.0).clamp(120.0, 560.0);
            self.details(ui, catalog, height, controls);
            ui.add_space(12.0);
            ui.separator();
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(
                        position.is_some_and(|i| i > 0),
                        egui::Button::new("Previous").min_size(egui::vec2(80.0, 36.0)),
                    )
                    .on_hover_text("Previous mod (Alt+Left)")
                    .clicked()
                {
                    direction = -1;
                }
                if ui
                    .add_enabled(
                        position.is_some_and(|i| i + 1 < catalog.visible.len()),
                        egui::Button::new("Next").min_size(egui::vec2(80.0, 36.0)),
                    )
                    .on_hover_text("Next mod (Alt+Right)")
                    .clicked()
                {
                    direction = 1;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close = ui.add_sized([80.0, 36.0], egui::Button::new("Close"));
                    self.stats.detail_close = Some(close.rect);
                    close.clicked()
                })
                .inner
            })
            .inner
        });
        if popup.inner || popup.should_close() {
            self.detail_open = None;
        } else if let Some(index) = position
            .and_then(|i| i.checked_add_signed(direction))
            .filter(|i| *i < catalog.visible.len())
            && direction != 0
        {
            catalog.select(catalog.visible[index]);
            self.detail_open = catalog.selected.clone();
        }
    }

    fn details(&mut self, ui: &mut egui::Ui, catalog: &Catalog, height: f32, controls: Option<&mut crate::controls::Controls>) {
        let Some(entry) = catalog.selected_mod() else {
            return;
        };
        let compact = height < 320.0;
        egui::ScrollArea::vertical()
            .id_salt(("detail-body", catalog.root.clone(), EntryId::of(entry)))
            .max_height(height)
            .auto_shrink([false, true])
            .show(ui, |ui| {
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
                        text_on_fill(ui, ui.visuals().widgets.inactive.weak_bg_fill);
                        ui.label(
                            RichText::new(state)
                                .size(12.0)
                                .color(ui.visuals().weak_text_color()),
                        );
                    });
                ui.add_space(8.0);
                if let Some(c) = controls
                    && entry.id.is_some()
                    && entry.source_attachment.as_ref().is_some_and(|source| source.provider_id == "curseforge" && source.project_id.is_some())
                { c.update_details(ui, catalog); ui.add_space(12.0); }
                let (photo, response) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), if compact { 120.0 } else { 240.0 }),
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
                ui.add_space(16.0);
                if let Some(source) = &entry.source_url {
                    ui.scope(|ui| {
                        ui.spacing_mut().item_spacing.y = 4.0;
                        ui.label(RichText::new("Saved source").size(16.0).strong().color(ui.visuals().text_color()));
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
                                ui.label(
                                    "Low confidence. Verify this source before relying on it.",
                                );
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
                        let identity = EntryId::of(entry);
                        let now = ui.input(|input| input.time);
                        let copied = self
                            .source_copied
                            .as_ref()
                            .is_some_and(|(id, until)| *id == identity && now < *until);
                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .button(
                                    RichText::new("Open source")
                                        .color(ui.visuals().hyperlink_color),
                                )
                                .clicked()
                            {
                                ui.ctx().open_url(egui::OpenUrl::new_tab(source));
                            }
                            let icon = if copied { Icon::Check } else { Icon::Copy };
                            if ui
                                .add(egui::Button::image_and_text(
                                    icon.image(ui.visuals().text_color(), 14.0),
                                    if copied { "URL copied" } else { "Copy URL" },
                                ))
                                .clicked()
                            {
                                ui.ctx().copy_text(source.clone());
                                self.source_copied = Some((identity, now + 2.0));
                                ui.ctx()
                                    .request_repaint_after(std::time::Duration::from_secs(2));
                            }
                        });
                    });
                    ui.add_space(12.0);
                }
                ui.separator();
                ui.add_space(8.0);
                let identity = EntryId::of(entry);
                let now = ui.input(|input| input.time);
                let copied = self
                    .copied
                    .as_ref()
                    .is_some_and(|(id, until)| *id == identity && now < *until);
                ui.horizontal(|ui| {
                    ui.label(RichText::new("Files").size(16.0).strong().color(ui.visuals().text_color()));
                    ui.label(
                        RichText::new(format!("{}", entry.files.len()))
                            .size(13.0)
                            .color(ui.visuals().weak_text_color()),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let icon = if copied { Icon::Check } else { Icon::Copy };
                        let copy = ui.add(egui::Button::image_and_text(
                            icon.image(ui.visuals().text_color(), 14.0),
                            if copied {
                                "Paths copied"
                            } else {
                                "Copy file paths"
                            },
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
                ui.add_space(4.0);
                let stride = 48.0 + ui.spacing().item_spacing.y;
                let origin = ui.next_widget_position().y;
                let start = (((ui.clip_rect().top() - origin) / stride).floor().max(0.0) as usize)
                    .min(entry.files.len());
                let end = ((((ui.clip_rect().bottom() - origin) / stride)
                    .ceil()
                    .max(0.0) as usize)
                    + 1)
                .min(entry.files.len())
                .max(start);
                if start > 0 {
                    ui.add_space(start as f32 * stride);
                }
                for index in start..end {
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
                if end < entry.files.len() {
                    ui.add_space((entry.files.len() - end) as f32 * stride);
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
        let state = self.previews.load(ui.ctx(), preview, root);
        if thumbnail || !matches!(state, crate::preview::State::Ready(_)) {
            ui.painter()
                .rect_filled(rect, 6.0, ui.visuals().widgets.inactive.weak_bg_fill);
        }
        match state {
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
                self.stats.pending_photos += 1;
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
    #[test]
    fn built_in_dark_surfaces_are_neutral_and_elevated() {
        let ctx = egui::Context::default();
        setup(&ctx);
        let v = &ctx.style_of(egui::Theme::Dark).visuals;
        for color in [v.panel_fill, v.extreme_bg_color] {
            let [r, g, b, _] = color.to_array();
            assert!(r.max(g).max(b) - r.min(g).min(b) <= 16,
                "dark surfaces should let the mod photos carry color: {color:?}");
        }
        assert!(v.extreme_bg_color.r() > v.panel_fill.r(), "cards must lift off the canvas");
    }

    #[test]
    fn library_search_can_be_cleared_without_losing_keyboard_focus() {
        let mut catalog = populated_catalog();
        catalog.search("0000".into());
        let mut view = View::default();
        let ctx = egui::Context::default();
        setup(&ctx);
        render(&ctx, &mut view, &mut catalog, vec![], 680.0);
        let clear = view.stats.search_clear.expect("nonempty search needs a clear action");
        render(&ctx, &mut view, &mut catalog, click(clear.center()), 680.0);
        assert!(catalog.query.is_empty());
        let id = ctx.memory(|memory| memory.focused()).expect("clearing returns focus to search");
        render(&ctx, &mut view, &mut catalog, vec![egui::Event::Text("new query".into())], 680.0);
        assert_eq!(catalog.query, "new query");
        assert_eq!(ctx.memory(|memory| memory.focused()), Some(id));
    }

    #[test]
    fn library_reuses_cached_remote_photo_after_rescan_and_restart_offline() {
        use sha2::{Digest, Sha256};
        let temp = tempfile::tempdir().unwrap();
        let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/photo.png", server.local_addr().unwrap());
        drop(server);
        let cache = temp.path().join("photos");
        std::fs::create_dir(&cache).unwrap();
        let path = cache.join(format!("ts4-photo-{:x}", Sha256::digest(url.as_bytes())));
        image::RgbaImage::from_pixel(20, 30, image::Rgba([10, 100, 80, 255]))
            .save_with_format(&path, image::ImageFormat::Png).unwrap();

        for _ in 0..2 {
            let ctx = egui::Context::default();
            setup(&ctx);
            let mut view = View::with_cache(cache.clone());
            for generation in [1, 2] {
                view.previews.begin_frame(&ctx, generation);
                let started = std::time::Instant::now();
                loop {
                    match view.previews.load(&ctx, Some(&url), None) {
                        crate::preview::State::Ready(texture) => {
                            assert_eq!(texture.size, egui::vec2(20.0, 30.0));
                            break;
                        }
                        crate::preview::State::Missing => panic!("library must reuse cached photos offline"),
                        crate::preview::State::Pending => {}
                    }
                    assert!(started.elapsed() < std::time::Duration::from_secs(3), "cached library photo did not load");
                    std::thread::sleep(std::time::Duration::from_millis(5));
                }
            }
        }
    }

    use super::*;
    #[test]
    fn unreadable_custom_text_is_adjusted_without_replacing_the_palette() {
        let ctx = egui::Context::default();
        setup(&ctx);
        let mut theme = crate::settings::CustomTheme::base();
        theme.light.text = theme.light.surface.clone();
        let settings = crate::settings::Settings { theme: crate::settings::Theme::Light, active_custom_theme: Some(theme.name.clone()), custom_themes: vec![theme], ..Default::default() };
        apply_settings_theme(&ctx, &settings);
        let visuals = &ctx.style_of(egui::Theme::Light).visuals;
        assert_ne!(visuals.text_color(), visuals.extreme_bg_color, "a 1:1 theme must not hide its own recovery controls");
    }

    #[test]
    fn opposing_custom_fills_use_their_own_text_inks() {
        let mut colors = crate::settings::ThemeColors::base(false);
        colors.background = "#FFFFFF".into();
        colors.surface = "#000000".into();
        colors.text = "#000000".into();
        colors.muted_text = "#000000".into();
        let visuals = custom_palette_visuals(&egui::Visuals::light(), &colors).unwrap();
        assert_eq!(visuals.text_color(), Color32::BLACK);
        let ctx = egui::Context::default();
        ctx.set_visuals(visuals);
        let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
            text_on_fill(ui, Color32::BLACK);
            assert_ne!(ui.visuals().text_color(), Color32::BLACK);
            assert_ne!(ui.visuals().weak_text_color(), Color32::BLACK);
            assert_eq!(ui.visuals().extreme_bg_color, Color32::BLACK);
            assert_eq!(ui.visuals().panel_fill, Color32::WHITE);
        });
        output.textures_delta.clear();
    }

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
            for foreground in [visuals.weak_text_color(), visuals.hyperlink_color] {
            let foreground = foreground.to_array();
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
                    "text/link contrast {contrast:.2} in {theme:?}"
                );
            }
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
        assert!(card.height() >= 180.0, "default should remain cards even without previews");
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
                assert!((logo.width() / logo.height() - 22.0 / 34.0).abs() < 0.01);
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
        assert_eq!(
            view.stats.files, 0,
            "details stay closed until a card is opened"
        );
        let row = view.stats.first_row.unwrap().center();
        render(&ctx, &mut view, &mut catalog, click(row), 1200.0);
        render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        assert!(view.stats.files > 0 && view.stats.files < 40);
        render(&ctx, &mut view, &mut catalog, vec![], 680.0);
        assert!(view.stats.rows < 30);
    }

    #[test]
    fn detail_photos_keep_their_proportions_without_a_full_width_tray() {
        let temp = tempfile::tempdir().unwrap();
        for (width, height) in [(80, 80), (40, 80), (160, 80)] {
            let path = temp.path().join(format!("{width}x{height}.png"));
            image::RgbaImage::from_pixel(width, height, image::Rgba([10, 100, 80, 255]))
                .save(&path).unwrap();
            let ctx = egui::Context::default();
            setup(&ctx);
            let mut view = View::default();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            let texture = loop {
                if let crate::preview::State::Ready(texture) = view.previews.load(&ctx, path.to_str(), None) {
                    break texture;
                }
                assert!(std::time::Instant::now() < deadline, "photo did not load");
                std::thread::sleep(std::time::Duration::from_millis(5));
            };
            for (preview, thumbnail) in [(path.to_str(), false), (path.to_str(), true), (None, false)] {
                let mut rect = egui::Rect::NOTHING;
                let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                    rect = ui.allocate_exact_size(egui::vec2(600.0, 180.0), egui::Sense::hover()).0;
                    view.paint_preview(ui, preview, None, rect, thumbnail);
                });
                output.textures_delta.clear();
                let has_tray = output.shapes.iter().any(|shape| matches!(&shape.shape,
                    egui::epaint::Shape::Rect(background) if background.rect == rect && background.fill != egui::Color32::TRANSPARENT));
                assert_eq!(has_tray, thumbnail || preview.is_none(), "loaded detail photos must not paint empty side gutters");
                if preview.is_some() && !thumbnail {
                    let photo = output.shapes.iter().find(|shape| shape.shape.texture_id() == texture.id).unwrap();
                    let bounds = photo.shape.visual_bounding_rect();
                    assert!((bounds.width() / bounds.height() - width as f32 / height as f32).abs() < 0.01);
                    assert!(bounds.center().distance(rect.center()) < 0.1);
                    assert!(rect.contains_rect(bounds));
                }
            }
        }
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
        render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        let row = view.stats.first_row.unwrap().center();
        render(&ctx, &mut view, &mut catalog, click(row), 1200.0);
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
    fn library_layout_controls_have_accessible_names() {
        let ctx = egui::Context::default();
        setup(&ctx);
        ctx.enable_accesskit();
        let mut view = View::default();
        let mut catalog = populated_catalog();
        render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        let output = render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        let tree = output.platform_output.accesskit_update.unwrap();
        for label in ["Cards", "List"] {
            assert!(tree.nodes.iter().any(|(_, node)| node.label() == Some(label)), "missing accessible {label} control");
        }
    }

    #[test]
    fn collections_without_previews_use_compact_cards() {
        let ctx = egui::Context::default();
        setup(&ctx);
        let mut view = View::default();
        let mut catalog = populated_catalog();
        render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        assert!(view.stats.first_row.unwrap().height() <= 200.0, "missing previews should not dominate the collection");
    }

    #[test]
    fn detail_shortcuts_follow_filtered_order_and_help_dismisses_safely() {
        let ctx = egui::Context::default();
        setup(&ctx);
        let mut view = View::default();
        let mut catalog = populated_catalog();
        catalog.search("0000".into());
        let first = catalog.visible[0];
        let next = catalog.visible[1];
        catalog.select(first);
        view.detail_open = catalog.selected.clone();
        render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        render(&ctx, &mut view, &mut catalog, vec![egui::Event::Key { key: egui::Key::ArrowRight, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::ALT }], 1200.0);
        assert_eq!(catalog.selected, Some(EntryId::of(&catalog.mods[next])));
        render(&ctx, &mut view, &mut catalog, vec![egui::Event::Key { key: egui::Key::F1, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }], 1200.0);
        let output = render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        assert!(view.help_open);
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::epaint::Shape::Text(text) if text.galley.text() == "Using your mod library")));
        render(&ctx, &mut view, &mut catalog, vec![egui::Event::Key { key: egui::Key::Escape, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }], 1200.0);
        assert!(!view.help_open);
        assert_eq!(view.detail_open, catalog.selected);
    }

    #[test]
    fn control_f_focuses_search_and_leaves_details_without_losing_selection() {
        let ctx = egui::Context::default();
        setup(&ctx);
        let mut view = View::default();
        let mut catalog = populated_catalog();
        catalog.select(0);
        view.detail_open = catalog.selected.clone();
        render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        render(&ctx, &mut view, &mut catalog, vec![egui::Event::Key { key: egui::Key::F, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::CTRL }], 1200.0);
        render(&ctx, &mut view, &mut catalog, vec![egui::Event::Text("mod 00001".into())], 1200.0);
        assert_eq!(catalog.query, "mod 00001");
        assert!(view.detail_open.is_none());
        assert!(catalog.selected.is_some());
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

    fn click(pos: egui::Pos2) -> Vec<egui::Event> {
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
    }

    #[test]
    fn the_shared_details_scroll_reaches_the_last_file_without_rendering_every_file() {
        let ctx = egui::Context::default();
        setup(&ctx);
        let mut view = View::default();
        let mut catalog = populated_catalog();
        catalog.select(0);
        view.detail_open = catalog.selected.clone();
        render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        let position = view.stats.copy.unwrap().center();
        render(&ctx, &mut view, &mut catalog, vec![egui::Event::PointerMoved(position), egui::Event::MouseWheel { unit: egui::MouseWheelUnit::Point, delta: egui::vec2(0.0, -1_000_000.0), phase: egui::TouchPhase::Move, modifiers: Default::default() }], 1200.0);
        for _ in 0..6 { render(&ctx, &mut view, &mut catalog, vec![], 1200.0); }
        let output = render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        assert!(view.stats.files > 0 && view.stats.files < 40);
        let last = catalog.selected_mod().unwrap().files.last().unwrap().rsplit('/').next().unwrap();
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::epaint::Shape::Text(text) if text.galley.text() == last)), "one body scroll must reach the final file");
    }

    #[test]
    fn details_source_actions_have_comfortable_targets_and_preserve_the_saved_url() {
        let ctx = egui::Context::default();
        setup(&ctx);
        let mut view = View::default();
        let mut catalog = populated_catalog();
        let source = "https://example.com/a-saved-source?file=42&name=Original";
        catalog.mods[0].source_url = Some(source.into());
        catalog.mods[0].files = vec!["One.package".into()];
        view.detail_open = catalog.selected.clone();
        for _ in 0..3 {
            render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        }
        let output = render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        let position = |output: &egui::FullOutput, label: &str| {
            output
                .shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) if text.galley.text() == label => {
                        Some(text.pos + text.galley.rect.center().to_vec2())
                    }
                    _ => None,
                })
                .unwrap()
        };
        let output = render(
            &ctx,
            &mut view,
            &mut catalog,
            click(position(&output, "Open source") + egui::vec2(0.0, 9.0)),
            1200.0,
        );
        assert!(output.platform_output.commands.iter().any(
            |command| matches!(command, egui::OutputCommand::OpenUrl(url) if url.url == source)
        ));
        let output = render(
            &ctx,
            &mut view,
            &mut catalog,
            click(position(&output, "Copy URL")),
            1200.0,
        );
        assert!(output.platform_output.commands.iter().any(
            |command| matches!(command, egui::OutputCommand::CopyText(text) if text == source)
        ));
        let output = render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        assert!(output.shapes.iter().any(|shape| matches!(&shape.shape, egui::epaint::Shape::Text(text) if text.galley.text() == "URL copied")));
    }

    #[test]
    fn short_mod_details_shrink_to_content_and_give_the_preview_more_space() {
        let ctx = egui::Context::default();
        setup(&ctx);
        let mut view = View::default();
        let mut catalog = populated_catalog();
        catalog.mods[0].files = vec!["One.package".into()];
        catalog.mods[0].source_url = None;
        catalog.select(0);
        view.detail_open = catalog.selected.clone();
        for _ in 0..3 {
            render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        }
        let output = render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
        let popup = ctx
            .read_response(egui::Id::new((
                "mod-details",
                &catalog.selected_mod().unwrap().name,
            )))
            .unwrap()
            .rect;
        assert!(
            popup.height() < 650.0,
            "one-file details should not reserve space for a long list: {popup:?}"
        );
        let preview = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Rect(rect)
                    if popup.contains_rect(rect.rect)
                        && rect.rect.width() > 400.0
                        && rect.rect.height() >= 120.0
                        && rect.rect.height() < 300.0 =>
                {
                    Some(rect.rect)
                }
                _ => None,
            })
            .expect("preview placeholder");
        assert!(
            preview.height() >= 240.0,
            "tall windows should give the mod preview more presence"
        );
        let previous = output
            .shapes
            .iter()
            .find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) if text.galley.text() == "Previous" => {
                    Some(text.pos)
                }
                _ => None,
            })
            .unwrap();
        assert!(
            previous.x < popup.left() + popup.width() / 2.0,
            "navigation belongs on the left of the footer"
        );
        assert!(
            ctx.content_rect()
                .contains_rect(view.stats.detail_close.unwrap())
        );
    }

    #[test]
    fn short_details_popup_shows_a_readable_preview_and_keeps_close_visible() {
        let ctx = egui::Context::default();
        setup(&ctx);
        let mut view = View::default();
        let mut catalog = populated_catalog();
        catalog.select(0);
        view.detail_open = catalog.selected.clone();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(640.0, 480.0));
        let mut output = None;
        for _ in 0..3 {
            let mut frame = ctx.run_ui(egui::RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| { view.show(ui, &mut catalog); });
            frame.textures_delta.clear();
            output = Some(frame);
        }
        let popup = ctx.read_response(egui::Id::new(("mod-details", &catalog.selected_mod().unwrap().name))).unwrap().rect;
        assert!(screen.contains_rect(popup));
        assert!(screen.contains_rect(view.stats.detail_close.unwrap()));
        assert!(output.unwrap().shapes.iter().any(|shape| matches!(&shape.shape, egui::epaint::Shape::Text(text)
            if text.galley.text() == "No Preview" && popup.contains(text.pos) && shape.clip_rect.contains(text.pos + text.galley.rect.center().to_vec2()) && shape.clip_rect.height() >= 120.0)), "preview must be visible in the popup body, not clipped to a strip");
    }

    #[test]
    fn details_popup_keeps_library_visible_and_copies_full_paths_at_every_width() {
        for width in [640.0, 680.0, 1200.0, 1885.0] {
            let mut catalog = populated_catalog();
            let mut view = View::default();
            let ctx = egui::Context::default();
            setup(&ctx);
            render(&ctx, &mut view, &mut catalog, vec![], width);
            render(&ctx, &mut view, &mut catalog, vec![], width);
            assert_eq!(view.stats.files, 0, "no persistent inspector");
            let row = view.stats.first_row.unwrap().center();
            render(&ctx, &mut view, &mut catalog, click(row), width);
            render(&ctx, &mut view, &mut catalog, vec![], width);
            assert!(view.stats.rows > 0 && view.stats.files > 0);
            let copy = view.stats.copy.unwrap().center();
            let output = render(&ctx, &mut view, &mut catalog, click(copy), width);
            assert!(output.platform_output.commands.iter().any(|command| matches!(
                command, egui::OutputCommand::CopyText(text) if *text == catalog.selected_mod().unwrap().files.join("\n")
            )));
            assert!(view.copied.is_some());
            let close = view.stats.detail_close.expect("popup Close control");
            assert!(ctx.content_rect().contains_rect(close));
            render(&ctx, &mut view, &mut catalog, click(close.center()), width);
            render(&ctx, &mut view, &mut catalog, vec![], width);
            assert!(view.stats.rows > 0 && view.stats.files == 0);
            assert!(catalog.selected.is_some());
        }
    }

    #[test]
    fn details_popup_dismisses_without_changing_selection_or_files() {
        for outside in [false, true] {
            let mut catalog = populated_catalog();
            let selected = catalog.selected.clone();
            let files = catalog.mods[0].files.clone();
            let mut view = View::default();
            let ctx = egui::Context::default();
            setup(&ctx);
            render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
            render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
            let row = view.stats.first_row.unwrap().center();
            render(&ctx, &mut view, &mut catalog, click(row), 1200.0);
            render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
            let events = if outside {
                click(egui::pos2(10.0, 10.0))
            } else {
                vec![egui::Event::Key {
                    key: egui::Key::Escape,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers: Default::default(),
                }]
            };
            render(&ctx, &mut view, &mut catalog, events, 1200.0);
            render(&ctx, &mut view, &mut catalog, vec![], 1200.0);
            assert_eq!(view.stats.files, 0);
            assert!(view.stats.rows > 0);
            assert_eq!(catalog.selected, selected);
            assert_eq!(catalog.mods[0].files, files);
        }
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
