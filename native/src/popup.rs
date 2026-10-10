use egui::{Context, Id, ModalResponse, Response, Ui};

pub fn show<T>(
    ctx: &Context,
    id: &str,
    title: &str,
    content: impl FnOnce(&mut Ui) -> T,
) -> ModalResponse<T> {
    let style = ctx.style_of(ctx.theme());
    let frame = egui::Frame::popup(&style)
        .fill(style.visuals.extreme_bg_color)
        .stroke(egui::Stroke::NONE)
        .corner_radius(12)
        .inner_margin(24);
    egui::Modal::new(Id::new((id, title)))
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_width((ctx.content_rect().width() - 96.0).clamp(240.0, 460.0));
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            ui.label(egui::RichText::new(title).size(23.0).strong());
            ui.add_space(12.0);
            content(ui)
        })
}

pub fn body_height(ctx: &Context) -> f32 {
    (ctx.content_rect().height() - 220.0).clamp(120.0, 420.0)
}

pub fn actions(ui: &mut Ui, label: &str, enabled: bool) -> (Response, Response) {
    ui.add_space(16.0);
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 36.0),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            let selection = ui.visuals().selection;
            let approve = ui.add_enabled(
                enabled,
                egui::Button::new(label)
                    .min_size(egui::vec2(96.0, 36.0))
                    .fill(selection.bg_fill)
                    .stroke(selection.stroke),
            );
            let cancel = ui.add_sized([80.0, 36.0], egui::Button::new("Cancel"));
            (approve, cancel)
        },
    )
    .inner
}

pub fn close(ui: &mut Ui) -> Response {
    ui.add_space(16.0);
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 36.0),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| ui.add_sized([80.0, 36.0], egui::Button::new("Close")),
    )
    .inner
}

pub fn text_field(ui: &mut Ui, value: &mut String) -> Response {
    let fill = ui.visuals().widgets.inactive.weak_bg_fill;
    ui.add_sized(
        [ui.available_width(), 36.0],
        egui::TextEdit::singleline(value)
            .background_color(fill)
            .margin(egui::vec2(10.0, 8.0)),
    )
}
