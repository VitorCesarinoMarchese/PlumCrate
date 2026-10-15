#[path = "support/project_details.rs"]
mod support;
use ts4_mod_manager_native::{
    catalog::Catalog,
    project_details::ReviewRequest,
    ui::{self, View},
};
fn frame(
    ctx: &egui::Context,
    view: &mut View,
    events: Vec<egui::Event>,
    size: [f32; 2],
) -> egui::FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size.into())),
            events,
            ..Default::default()
        },
        |ui| {
            view.show(ui, &mut Catalog::default());
        },
    );
    output.textures_delta.clear();
    output
}
fn text(output: &egui::FullOutput, value: &str) -> bool {
    output
        .shapes
        .iter()
        .any(|s| matches!(&s.shape,egui::epaint::Shape::Text(t) if t.galley.text().contains(value)))
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
fn project_page_shows_full_description_and_separate_real_dependency_reviews() {
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    let mut view = View::default();
    view.browse_page = true;
    support::seed(&ctx, &mut view.discovery);
    let mut output = frame(&ctx, &mut view, vec![], [1440.0, 1000.0]);
    for _ in 0..3 {
        output = frame(&ctx, &mut view, vec![], [1440.0, 1000.0]);
    }
    assert!(text(&output, "A livelier garden"));
    assert!(text(&output, "Synthetic Core Library"));
    assert!(text(&output, "Required"));
    assert!(text(&output, "Incompatible"));
    assert!(text(&output, "Author-linked mods"));
    assert_eq!(view.discovery.stats.dependency_reviews.len(), 1);
    assert!(view.discovery.take_import_request().is_none());
    let rect = view.discovery.stats.dependency_reviews[0].1;
    let output = frame(&ctx, &mut view, click(rect), [1440.0, 1000.0]);
    assert!(matches!(
        view.discovery.take_import_request(),
        Some(ReviewRequest::ProjectId(9))
    ));
    assert!(output.platform_output.commands.is_empty());
}
#[test]
fn choosing_an_older_release_removes_newer_files_dependency_claims() {
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    let mut view = View::default();
    view.browse_page = true;
    support::seed(&ctx, &mut view.discovery);
    for _ in 0..3 {
        frame(&ctx, &mut view, vec![], [640.0, 900.0]);
    }
    view.discovery.details.chosen = Some(101);
    for _ in 0..3 {
        frame(&ctx, &mut view, vec![], [640.0, 900.0]);
    }
    let rect = view.discovery.stats.browser_import.unwrap();
    frame(&ctx, &mut view, click(rect), [640.0, 900.0]);
    assert!(matches!(
        view.discovery.take_import_request(),
        Some(ReviewRequest::Loaded { file_id: 101, .. })
    ));
    let rect = view.discovery.stats.dependencies_jump.unwrap();
    frame(&ctx, &mut view, click(rect), [640.0, 900.0]);
    let mut output = frame(&ctx, &mut view, vec![], [640.0, 900.0]);
    for _ in 0..2 {
        output = frame(&ctx, &mut view, vec![], [640.0, 900.0]);
    }
    assert!(text(&output, "No dependencies declared for this release"));
    assert!(view.discovery.stats.dependency_reviews.is_empty());
    assert!(!text(&output, "Synthetic Core Library"));
}

#[test]
fn short_windows_can_jump_to_description_and_dependencies_without_horizontal_overflow() {
    let ctx = egui::Context::default();
    ui::setup(&ctx);
    let mut view = View::default();
    view.browse_page = true;
    support::seed(&ctx, &mut view.discovery);
    for _ in 0..3 {
        frame(&ctx, &mut view, vec![], [640.0, 480.0]);
    }
    let jump = view.discovery.stats.dependencies_jump.unwrap();
    assert!(jump.right() <= 640.0);
    frame(&ctx, &mut view, click(jump), [640.0, 480.0]);
    let mut output = frame(&ctx, &mut view, vec![], [640.0, 480.0]);
    for _ in 0..3 {
        output = frame(&ctx, &mut view, vec![], [640.0, 480.0]);
    }
    assert!(text(&output, "Synthetic Core Library"));
    for (_, rect) in &view.discovery.stats.dependency_reviews {
        assert!(rect.left() >= 0.0 && rect.right() <= 640.0);
    }
    let jump = view.discovery.stats.description_jump.unwrap();
    frame(&ctx, &mut view, click(jump), [640.0, 480.0]);
    for _ in 0..4 {
        output = frame(&ctx, &mut view, vec![], [640.0, 480.0]);
    }
    assert!(text(&output, "A livelier garden"));
}
