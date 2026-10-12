use egui::{Ui, Vec2, vec2};

pub fn block(ui: &mut Ui, size: Vec2) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 6, ui.visuals().widgets.inactive.bg_fill);
}

pub fn paragraphs(ui: &mut Ui) {
    let width = ui.available_width().min(720.0);
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing.y = 4.0;
        for fraction in [1.0, 0.94, 0.86, 0.62] {
            block(ui, vec2(width * fraction, 13.0));
        }
    });
}

pub fn cards(ui: &mut Ui) -> usize {
    let columns = ((ui.available_width() + 16.0) / 280.0).floor().max(1.0) as usize;
    let width = (ui.available_width() - (columns - 1) as f32 * 16.0) / columns as f32;
    let draw = |ui: &mut Ui| {
        for _ in 0..2 {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                for _ in 0..columns {
                    let frame=egui::Frame::new()
                        .fill(ui.visuals().extreme_bg_color)
                        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
                        .corner_radius(8)
                        .inner_margin(14);
                    let inner_width=width-frame.total_margin().sum().x;
                    frame.show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.set_width(inner_width);
                                ui.set_min_height(238.0);
                                ui.allocate_ui(vec2(ui.available_width(), 64.0), |ui| {
                                    ui.set_min_height(64.0);
                                    ui.horizontal_top(|ui| {
                                        block(ui, vec2(60.0, 60.0));
                                        ui.vertical(|ui| {
                                            ui.spacing_mut().item_spacing.y = 5.0;
                                            let width = ui.available_width();
                                            block(ui, vec2(width * 0.9, 16.0));
                                            block(ui, vec2(width * 0.7, 16.0));
                                            block(ui, vec2(width * 0.55, 11.0));
                                        });
                                    });
                                });
                                ui.add_space(10.0);
                                ui.allocate_ui(vec2(ui.available_width(), 54.0), |ui| {
                                    ui.set_min_height(54.0);
                                    ui.spacing_mut().item_spacing.y = 4.0;
                                    for fraction in [1.0, 0.94, 0.7] {
                                        block(ui, vec2(ui.available_width() * fraction, 13.0));
                                    }
                                });
                                ui.add_space(8.0);
                                block(ui, vec2(ui.available_width() * 0.45, 11.0));
                                block(ui, vec2(ui.available_width() * 0.8, 11.0));
                                block(ui, vec2(ui.available_width(), 34.0));
                            });
                        });
                }
            });
        }
    };
    egui::ScrollArea::vertical()
        .id_salt("pending-cards")
        .auto_shrink([false, false])
        .show(ui, draw);
    columns * 2
}
