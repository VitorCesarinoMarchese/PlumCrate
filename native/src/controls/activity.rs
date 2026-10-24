use super::*;
use egui::RichText;

const NOTICE_SECONDS: f64 = 5.0;

#[derive(Clone)]
struct Message {
    text: String,
    error: bool,
    created: f64,
    read: bool,
    dismissed: bool,
}

#[derive(Default)]
pub(super) struct Activity {
    messages: Vec<Message>,
    last_errors: [Option<String>; 2],
    popup_id: Option<egui::Id>,
}
impl Activity {
    fn push(&mut self, text: String, error: bool, now: f64) {
        self.messages.push(Message {
            text,
            error,
            created: now,
            read: false,
            dismissed: false,
        });
        if self.messages.len() > 30
            && let Some(index) = self
                .messages
                .iter()
                .position(|message| !message.error || message.dismissed)
        {
            self.messages.remove(index);
        }
    }
    fn observe_error(&mut self, source: usize, error: Option<String>, now: f64) {
        if self.last_errors[source] != error {
            if let Some(text) = &error {
                self.push(text.clone(), true, now);
            }
            self.last_errors[source] = error;
        }
    }
    fn issues(&self) -> usize {
        self.messages
            .iter()
            .filter(|message| message.error && !message.dismissed)
            .count()
    }
    fn unread(&self) -> usize {
        self.messages
            .iter()
            .filter(|message| !message.read && !message.dismissed)
            .count()
    }
    fn toast(&self, now: f64) -> Option<(usize, &Message)> {
        self.messages
            .iter()
            .enumerate()
            .rev()
            .find(|(_, message)| message.error && !message.dismissed)
            .or_else(|| {
                self.messages.iter().enumerate().rev().find(|(_, message)| {
                    !message.dismissed && !message.read && now - message.created < NOTICE_SECONDS
                })
            })
    }
}

impl Controls {
    fn sync_activity(&mut self, now: f64) {
        if let Some(notice) = self.notice.take() {
            self.activity.push(notice, false, now);
        }
        self.activity.observe_error(0, self.error.clone(), now);
        let browser_error = (self.browser_import.phase == crate::browser_import::Phase::Failed)
            .then(|| {
                self.browser_import
                    .error
                    .clone()
                    .unwrap_or_else(|| "Browser download import failed.".into())
            });
        self.activity.observe_error(1, browser_error, now);
    }
    fn running_tasks(&self) -> usize {
        let pending =
            |operation: &&Operation| matches!(operation.status.as_str(), "Queued" | "Running");
        let operations = self.operations.iter().filter(pending).count();
        let browser_owned = self
            .operations
            .iter()
            .filter(pending)
            .any(|operation| matches!(operation.job.action, Action::ImportBrowser(_)));
        operations
            + usize::from(self.recovering)
            + usize::from((self.browser_import.running() || self.browser_import.preparing()) && !browser_owned)
    }
    pub fn activity_header(&mut self, ui: &mut egui::Ui, scanning: bool, compact: bool) {
        self.sync_activity(ui.input(|input| input.time));
        self.activity_popup_rect = None;
        self.browser_cancel_rect = None;
        let running = self.running_tasks() + usize::from(scanning);
        let issues = self.activity.issues();
        let count = if issues > 0 {
            issues
        } else if running > 0 {
            running
        } else {
            self.activity.unread()
        };
        let label = if compact {
            if count == 0 {
                String::new()
            } else {
                count.to_string()
            }
        } else if count == 0 {
            "Activity".into()
        } else {
            format!("Activity · {count}")
        };
        let ink = if issues > 0 {
            crate::ui::readable_text(
                ui.visuals().error_fg_color,
                &[
                    ui.visuals().widgets.inactive.weak_bg_fill,
                    ui.visuals().widgets.hovered.weak_bg_fill,
                ],
            )
        } else {
            ui.visuals().text_color()
        };
        let button = ui
            .add_enabled(
                !self.review_open(),
                egui::Button::image_and_text(
                    crate::ui::Icon::Activity.image(ink, 18.0),
                    RichText::new(label).color(ink),
                ),
            )
            .on_hover_text(format!(
                "Activity · {running} running · {issues} need attention · {} unread",
                self.activity.unread()
            ));
        button.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), "Activity")
        });
        self.activity_button_rect = Some(button.rect);
        let popup = egui::Popup::menu(&button)
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .width((ui.ctx().content_rect().width() - 48.0).min(420.0));
        self.activity.popup_id = Some(popup.get_id());
        if self.review_open() {
            egui::Popup::close_id(ui.ctx(), popup.get_id());
        }
        if let Some(response) = popup.show(|ui| {
            ui.set_max_width((ui.ctx().content_rect().width() - 80.0).min(396.0));
            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
            crate::ui::text_on_fill(ui, ui.visuals().extreme_bg_color);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Activity")
                        .size(20.0)
                        .color(ui.visuals().text_color()),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close = ui
                        .add(
                            egui::Button::image(
                                crate::ui::Icon::Clear.image(ui.visuals().text_color(), 16.0),
                            )
                            .frame(false),
                        )
                        .on_hover_text("Close activity");
                    close.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::WidgetType::Button,
                            ui.is_enabled(),
                            "Close activity",
                        )
                    });
                    if close.clicked() {
                        ui.close();
                    }
                });
            });
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt("activity-history")
                .max_height((ui.ctx().content_rect().height() - 180.0).clamp(120.0, 420.0))
                .show(ui, |ui| self.activity_contents(ui, scanning));
        }) {
            self.activity_popup_rect = Some(response.response.rect);
            for message in &mut self.activity.messages {
                message.read = true;
            }
        }
    }
    fn activity_contents(&mut self, ui: &mut egui::Ui, scanning: bool) {
        if scanning {
            ui.label("Scanning your Mods folder…");
        }
        if self.recovering {
            ui.label("Checking interrupted operations…");
        }
        if self.busy() && !self.recovering {
            ui.label("Approved operations will finish before the window closes.");
        }
        self.browser_import_status(ui);
        for operation in self
            .operations
            .iter()
            .filter(|operation| matches!(operation.status.as_str(), "Queued" | "Running"))
        {
            if matches!(operation.job.action, Action::ImportBrowser(_)) {
                continue;
            }
            ui.horizontal_wrapped(|ui| {
                if operation.status == "Running" {
                    ui.spinner();
                }
                ui.label(format!(
                    "{} · {}",
                    operation.job.action.label(),
                    operation.status
                ));
            });
        }
        if let Some(theme) = self
            .saved_settings
            .custom_themes
            .iter()
            .find(|theme| Some(&theme.name) == self.saved_settings.active_custom_theme.as_ref())
            && !theme
                .palette(ui.visuals().dark_mode)
                .contrast_warnings()
                .is_empty()
        {
            ui.label("Theme text is adjusted for readability. Your chosen colors are kept.");
        }
        if self.activity.messages.is_empty()
            && self.operations.is_empty()
            && self.running_tasks() == 0
            && !scanning
            && !matches!(
                self.browser_import.phase,
                crate::browser_import::Phase::Cancelled
            )
        {
            ui.label("No activity yet");
            ui.label(
                RichText::new("Tasks and recent notifications will appear here.")
                    .color(ui.visuals().weak_text_color()),
            );
        }
        let mut dismiss = None;
        if !self.activity.messages.is_empty() {
            ui.add_space(8.0);
            ui.label(RichText::new("Recent notifications").color(ui.visuals().weak_text_color()));
            for (index, message) in self.activity.messages.iter().enumerate().rev() {
                ui.push_id(index, |ui| {
                    ui.add_space(6.0);
                    if message.error && !message.dismissed {
                        ui.colored_label(
                            crate::ui::readable_text(
                                ui.visuals().error_fg_color,
                                &[ui.visuals().extreme_bg_color],
                            ),
                            &message.text,
                        );
                        if ui.button("Dismiss").clicked() {
                            dismiss = Some(index);
                        }
                    } else {
                        ui.label(&message.text);
                    }
                    ui.separator();
                });
            }
        }
        if let Some(index) = dismiss {
            self.activity.messages[index].dismissed = true;
        }
        let mut retry = None;
        if !self.operations.is_empty() {
            egui::CollapsingHeader::new("Operation history")
                .id_salt("management-activity")
                .default_open(self.operations.iter().any(|operation| operation.error))
                .show(ui, |ui| {
                    for operation in self.operations.iter().rev() {
                        ui.push_id(operation.job.identity.operation, |ui| {
                            ui.label(format!(
                                "{} · {}",
                                operation.job.action.label(),
                                operation.status
                            ));
                            ui.label(
                                RichText::new(operation.job.identity.root.display().to_string())
                                    .small()
                                    .color(ui.visuals().weak_text_color()),
                            );
                            if operation.error
                                && !matches!(operation.job.action, Action::ImportBrowser(_))
                                && ui.button("Retry").clicked()
                            {
                                retry = Some(operation.job.clone());
                            }
                            ui.separator();
                        });
                    }
                });
        }
        if let Some(mut job) = retry {
            job.identity.operation = self.next;
            self.next += 1;
            self.submit(job.identity, job.action);
        }
    }
    pub fn notifications(&mut self, ctx: &egui::Context) {
        let now = ctx.input(|input| input.time);
        self.sync_activity(now);
        if self.review_open() || self.activity_popup_rect.is_some() {
            return;
        }
        let Some((index, message)) = self
            .activity
            .toast(now)
            .map(|(index, message)| (index, message.clone()))
        else {
            return;
        };
        if !message.error {
            ctx.request_repaint_after(std::time::Duration::from_secs_f64(
                (NOTICE_SECONDS - (now - message.created)).max(0.01),
            ));
        }
        egui::Area::new(egui::Id::new("activity-notification"))
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-24.0, -24.0))
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .inner_margin(16)
                    .corner_radius(12)
                    .show(ui, |ui| {
                        ui.set_width((ctx.content_rect().width() - 80.0).clamp(240.0, 360.0));
                        crate::ui::text_on_fill(ui, ui.visuals().extreme_bg_color);
                        let title = if message.error {
                            "Needs attention"
                        } else {
                            "Notification"
                        };
                        let color = if message.error {
                            crate::ui::readable_text(
                                ui.visuals().error_fg_color,
                                &[ui.visuals().extreme_bg_color],
                            )
                        } else {
                            ui.visuals().text_color()
                        };
                        ui.label(RichText::new(title).color(color).strong());
                        ui.add(
                            egui::Label::new(message.text.lines().next().unwrap_or(&message.text))
                                .wrap(),
                        );
                        ui.horizontal(|ui| {
                            if ui.button("View activity").clicked()
                                && let Some(id) = self.activity.popup_id
                            {
                                egui::Popup::open_id(ctx, id);
                            }
                            if ui.button("Dismiss").clicked() {
                                self.activity.messages[index].dismissed = true;
                            }
                        });
                    });
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn an_unresolved_error_takes_priority_over_a_newer_success() {
        let mut activity = Activity::default();
        activity.observe_error(0, Some("Original file kept".into()), 1.0);
        activity.push("Settings saved".into(), false, 2.0);
        assert!(activity.toast(3.0).unwrap().1.error);
    }
    #[test]
    fn notices_expire_without_losing_history_and_errors_require_dismissal() {
        let mut activity = Activity::default();
        activity.push("Settings saved".into(), false, 1.0);
        assert!(activity.toast(2.0).is_some());
        assert!(activity.toast(7.0).is_none());
        assert_eq!(activity.messages.len(), 1);
        activity.observe_error(0, Some("Original file kept".into()), 8.0);
        assert!(activity.toast(1000.0).unwrap().1.error);
        activity.messages[1].dismissed = true;
        assert!(activity.toast(1001.0).is_none());
        assert_eq!(activity.messages.len(), 2);
    }
    #[test]
    fn repeated_notices_are_distinct_but_unchanged_errors_are_not_repeated() {
        let mut activity = Activity::default();
        activity.push("Settings saved".into(), false, 1.0);
        activity.push("Settings saved".into(), false, 2.0);
        activity.observe_error(0, Some("Error".into()), 3.0);
        activity.observe_error(0, Some("Error".into()), 4.0);
        assert_eq!(activity.messages.len(), 3);
        activity.observe_error(0, None, 5.0);
        activity.observe_error(0, Some("Error".into()), 6.0);
        assert_eq!(activity.messages.len(), 4);
    }
    #[test]
    fn recent_history_does_not_evict_undismissed_errors() {
        let mut activity = Activity::default();
        activity.observe_error(0, Some("Original file kept".into()), 0.0);
        for index in 0..40 {
            activity.push(format!("Notice {index}"), false, index as f64);
        }
        assert_eq!(activity.messages.len(), 30);
        assert_eq!(activity.issues(), 1);
        assert!(activity.toast(100.0).unwrap().1.error);
    }
}
