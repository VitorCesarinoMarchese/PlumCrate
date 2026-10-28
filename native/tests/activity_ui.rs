use plumcrate::{
    catalog::Catalog,
    controls::Controls,
    settings::Settings,
    ui::{self, View},
};

struct Fixture {
    _directory: tempfile::TempDir,
    controls: Controls,
    view: View,
    catalog: Catalog,
    ctx: egui::Context,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let mut controls = Controls::new(
            directory.path().join("managed"),
            directory.path().into(),
            Settings::default(),
            || {},
        )
        .unwrap();
        let mut catalog = Catalog::default();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
        while controls.busy() {
            controls.poll(&mut catalog);
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let ctx = egui::Context::default();
        ui::setup(&ctx);
        Self {
            _directory: directory,
            controls,
            view: View::default(),
            catalog,
            ctx,
        }
    }
    fn frame(&mut self, size: [f32; 2], time: f64, events: Vec<egui::Event>) -> egui::FullOutput {
        let mut output = self.ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size.into())),
                time: Some(time),
                events,
                ..Default::default()
            },
            |ui| {
                self.view
                    .show_with_controls(ui, &mut self.catalog, &mut self.controls);
            },
        );
        output.textures_delta.clear();
        output
    }
}
fn text(output: &egui::FullOutput, value: &str) -> bool {
    output
        .shapes
        .iter()
        .any(|s| matches!(&s.shape, egui::Shape::Text(t) if t.galley.text().contains(value)))
}
fn click(rect: egui::Rect) -> Vec<egui::Event> {
    let pos = rect.center();
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
fn idle_library_has_header_activity_without_a_permanent_footer() {
    let mut f = Fixture::new();
    let out = f.frame([1200.0, 800.0], 1.0, vec![]);
    assert!(
        !text(&out, "Local library"),
        "redundant footer must not occupy library space"
    );
    let button = f
        .controls
        .activity_button_rect
        .expect("Activity belongs in the header");
    assert!(button.bottom() < 80.0);
    f.frame([1200.0, 800.0], 2.0, click(button));
    let out = f.frame([1200.0, 800.0], 3.0, vec![]);
    assert!(text(&out, "No activity yet"));
    assert!(f.controls.activity_popup_rect.is_some());
    assert!(
        !f.controls.review_open(),
        "Activity must not be a confirmation dialog"
    );
}
#[test]
fn browser_waiting_can_be_cancelled_from_activity_at_compact_and_wide_sizes() {
    for size in [[640.0, 480.0], [1876.0, 996.0]] {
        let mut f = Fixture::new();
        f.controls.browser_import.phase = plumcrate::browser_import::Phase::Waiting;
        f.frame(size, 1.0, vec![]);
        let button = f
            .controls
            .activity_button_rect
            .expect("running task has a header entry");
        assert!(
            f.ctx.content_rect().contains_rect(button),
            "compact header action must fit"
        );
        f.frame(size, 2.0, click(button));
        let out = f.frame(size, 3.0, vec![]);
        assert!(text(&out, "Waiting for"));
        assert!(
            f.ctx
                .content_rect()
                .contains_rect(f.controls.activity_popup_rect.unwrap())
        );
        let cancel = f
            .controls
            .browser_cancel_rect
            .expect("Cancel remains accessible in Activity");
        f.frame(size, 4.0, click(cancel));
        assert_eq!(
            f.controls.browser_import.phase,
            plumcrate::browser_import::Phase::Cancelled
        );
        assert!(!f._directory.path().join("managed/mods").exists());
    }
}
#[test]
fn errors_remain_visible_until_dismissed_and_remain_in_activity_history() {
    let mut f = Fixture::new();
    f.controls.error = Some("Path collision: original file was kept".into());
    f.frame([1200.0, 800.0], 0.9, vec![]);
    let out = f.frame([1200.0, 800.0], 1.0, vec![]);
    assert!(text(&out, "Path collision"));
    let out = f.frame([1200.0, 800.0], 30.0, vec![]);
    assert!(
        text(&out, "Path collision"),
        "errors must not expire like success messages"
    );
    let dismiss = out
        .shapes
        .iter()
        .find_map(|s| match &s.shape {
            egui::Shape::Text(t) if t.galley.text() == "Dismiss" => {
                Some(egui::Rect::from_min_size(t.pos, t.galley.size()))
            }
            _ => None,
        })
        .expect("error has an explicit dismiss action");
    f.frame([1200.0, 800.0], 31.0, click(dismiss));
    let out = f.frame([1200.0, 800.0], 32.0, vec![]);
    assert!(!text(&out, "Path collision"));
    f.frame(
        [1200.0, 800.0],
        33.0,
        click(f.controls.activity_button_rect.unwrap()),
    );
    let out = f.frame([1200.0, 800.0], 34.0, vec![]);
    assert!(
        text(&out, "Path collision"),
        "dismissal must retain session history"
    );
}

#[test]
fn dismissing_the_activity_panel_does_not_cancel_its_running_task() {
    let mut f = Fixture::new();
    f.controls.browser_import.phase = plumcrate::browser_import::Phase::Waiting;
    f.frame([640.0, 480.0], 1.0, vec![]);
    f.frame(
        [640.0, 480.0],
        2.0,
        click(f.controls.activity_button_rect.unwrap()),
    );
    f.frame([640.0, 480.0], 3.0, vec![]);
    f.frame(
        [640.0, 480.0],
        4.0,
        click(egui::Rect::from_center_size(
            egui::pos2(10.0, 460.0),
            egui::vec2(4.0, 4.0),
        )),
    );
    f.frame([640.0, 480.0], 5.0, vec![]);
    assert!(f.controls.activity_popup_rect.is_none());
    assert_eq!(
        f.controls.browser_import.phase,
        plumcrate::browser_import::Phase::Waiting
    );
}
