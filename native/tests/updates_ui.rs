#[path = "support/updates.rs"]
mod support;
use egui::{Event, FullOutput};

fn frame(ctx: &egui::Context, fixture: &mut support::Fixture, events: Vec<Event>) -> FullOutput {
    let mut output = ctx.run_ui(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1200.0, 800.0),
            )),
            time: Some(0.0),
            events,
            ..Default::default()
        },
        |ui| {
            fixture
                .view
                .show_with_controls(ui, &mut fixture.catalog, &mut fixture.controls);
            fixture.controls.show_dialogs(ui.ctx(), &fixture.catalog);
        },
    );
    output.textures_delta.clear();
    output
}
fn click(rect: egui::Rect) -> Vec<Event> {
    let pos = rect.center();
    vec![
        Event::PointerMoved(pos),
        Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: true,
            modifiers: Default::default(),
        },
        Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        },
    ]
}
fn has_text(output: &FullOutput, label: &str) -> bool {
    output.shapes.iter().any(|shape| matches!(&shape.shape,egui::epaint::Shape::Text(text) if text.galley.text().contains(label)))
}

#[test]
fn library_offers_update_checks_and_detail_page_opens_without_changing_files() {
    let mut fixture = support::fixture();
    let id = fixture.catalog.mods[0].id.clone().unwrap();
    let release = fixture
        .report
        .releases
        .iter()
        .find(|r| r.file_id == 200)
        .unwrap()
        .clone();
    fixture
        .controls
        .updates
        .reports
        .get_mut(&id)
        .unwrap()
        .as_mut()
        .unwrap()
        .status = ts4_mod_manager_core::mod_updates::UpdateStatus::Available(release);
    let metadata = fixture
        .directory
        .path()
        .join("managed/mods")
        .join(&id)
        .join("meta.json");
    let before = std::fs::read(&metadata).unwrap();
    let ctx = egui::Context::default();
    ts4_mod_manager_native::ui::setup(&ctx);
    for _ in 0..3 {
        frame(&ctx, &mut fixture, vec![]);
    }
    let output = frame(&ctx, &mut fixture, vec![]);
    assert!(has_text(&output, "Check updates"));
    let row = fixture.view.stats.first_row.unwrap();
    frame(&ctx, &mut fixture, click(row));
    let output = frame(&ctx, &mut fixture, vec![]);
    assert!(has_text(&output, "Newer public project release available"));
    let page = fixture
        .controls
        .open_update_page_rect
        .expect("mod page action in details");
    let output = frame(&ctx, &mut fixture, click(page));
    assert!(output.platform_output.commands.iter().any(|command| matches!(command,egui::OutputCommand::OpenUrl(url) if url.url==fixture.report.source_url)));
    assert_eq!(std::fs::read(metadata).unwrap(), before);
    assert_eq!(
        std::fs::read(
            fixture
                .directory
                .path()
                .join("managed/mods")
                .join(id)
                .join("files/fixture.package")
        )
        .unwrap(),
        b"synthetic fixture bytes"
    );
}

#[test]
fn missing_saved_key_opens_settings_without_starting_a_check() {
    let mut fixture = support::fixture();
    let ctx = egui::Context::default();
    ts4_mod_manager_native::ui::setup(&ctx);
    for _ in 0..3 {
        frame(&ctx, &mut fixture, vec![]);
    }
    let button = fixture
        .controls
        .check_updates_rect
        .expect("library update button");
    frame(&ctx, &mut fixture, click(button));
    assert!(fixture.controls.settings_page);
    assert!(!fixture.controls.updates.pending);
}

#[test]
fn recording_a_release_writes_only_after_the_confirmation_button() {
    let mut fixture = support::fixture();
    let id = fixture.catalog.mods[0].id.clone().unwrap();
    let metadata = fixture
        .directory
        .path()
        .join("managed/mods")
        .join(&id)
        .join("meta.json");
    let before = std::fs::read(&metadata).unwrap();
    let release = fixture
        .report
        .releases
        .iter()
        .find(|r| r.file_id == 100)
        .unwrap()
        .clone();
    fixture
        .controls
        .review_installed_release(&fixture.catalog, &fixture.report, &release);
    let ctx = egui::Context::default();
    ts4_mod_manager_native::ui::setup(&ctx);
    for _ in 0..3 {
        frame(&ctx, &mut fixture, vec![]);
    }
    assert_eq!(std::fs::read(&metadata).unwrap(), before);
    let confirm = fixture
        .controls
        .confirm_rect
        .expect("explicit confirmation");
    frame(&ctx, &mut fixture, click(confirm));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while fixture.controls.busy() {
        fixture.controls.poll(&fixture.catalog);
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    let saved: serde_json::Value =
        serde_json::from_slice(&std::fs::read(metadata).unwrap()).unwrap();
    assert_eq!(saved["sourceAttachment"]["installedRelease"]["fileId"], 100);
    assert_eq!(
        std::fs::read(
            fixture
                .directory
                .path()
                .join("managed/mods")
                .join(id)
                .join("files/fixture.package")
        )
        .unwrap(),
        b"synthetic fixture bytes"
    );
}

fn text_rect(output: &FullOutput, label: &str) -> egui::Rect {
    let texts: Vec<_> = output.shapes.iter().filter_map(|shape| match &shape.shape {
        egui::epaint::Shape::Text(text) => Some(text), _ => None,
    }).collect();
    let text = texts.iter().find(|text| text.galley.text() == label)
        .or_else(|| texts.iter().find(|text| text.galley.text().contains(label)))
        .unwrap_or_else(|| panic!("visible text missing: {label}"));
    text.galley.rect.translate(text.pos.to_vec2())
}

#[test]
fn unknown_version_picker_requires_a_choice_before_review() {
    let mut fixture = support::fixture();
    let ctx = egui::Context::default();
    ts4_mod_manager_native::ui::setup(&ctx);
    ctx.all_styles_mut(|style| style.animation_time = 0.0);
    for _ in 0..3 {
        frame(&ctx, &mut fixture, vec![]);
    }
    let row = fixture.view.stats.first_row.unwrap();
    frame(&ctx, &mut fixture, click(row));
    let output = frame(&ctx, &mut fixture, vec![]);
    let picker = text_rect(&output, "Choose the release you installed");
    frame(&ctx, &mut fixture, click(picker));
    let output = frame(&ctx, &mut fixture, vec![]);
    let old = text_rect(&output, "Synthetic mod 1.0.zip");
    frame(&ctx, &mut fixture, click(old));
    frame(&ctx, &mut fixture, vec![]);
    let review = fixture.controls.record_release_rect.unwrap();
    frame(&ctx, &mut fixture, click(review));
    let output = frame(&ctx, &mut fixture, vec![]);
    assert!(has_text(&output, "Confirm installed release"));
    assert!(fixture.controls.confirm_rect.is_some());
    let id = fixture.catalog.mods[0].id.as_ref().unwrap();
    let metadata: serde_json::Value = serde_json::from_slice(
        &std::fs::read(
            fixture
                .directory
                .path()
                .join("managed/mods")
                .join(id)
                .join("meta.json"),
        )
        .unwrap(),
    )
    .unwrap();
    assert!(metadata["sourceAttachment"]["installedRelease"].is_null());
}

#[test]
fn library_switches_enable_and_disable_without_opening_details_or_review() {
    for list in [false, true] {
        let mut fixture = support::fixture();
        fixture.catalog.selected = None;
        let original = fixture.directory.path().join("source/fixture.package");
        let ctx = egui::Context::default();
        ts4_mod_manager_native::ui::setup(&ctx);
        frame(&ctx, &mut fixture, vec![]);
        frame(&ctx, &mut fixture, vec![]);
        if list {
            let button = fixture.view.stats.list_switch.unwrap();
            frame(&ctx, &mut fixture, click(button));
            frame(&ctx, &mut fixture, vec![]);
        }
        for enabled in [true, false] {
            let switch = fixture.view.stats.first_toggle.expect("both library layouts need a switch");
            frame(&ctx, &mut fixture, click(switch));
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
            while fixture.controls.busy() {
                fixture.controls.poll(&fixture.catalog);
                assert!(std::time::Instant::now() < deadline);
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            let root = fixture.catalog.root.clone().unwrap();
            let mods = ts4_mod_manager_native::worker::scan(&root, &fixture.directory.path().join("managed")).unwrap().mods;
            assert_eq!(mods[0].enabled, enabled, "one switch click must apply the change");
            let generation = fixture.catalog.begin_scan(root);
            fixture.catalog.accept_scan(generation, Ok(mods));
            frame(&ctx, &mut fixture, vec![]);
            assert!(!fixture.controls.review_open());
            assert!(fixture.view.stats.detail_close.is_none());
            assert_eq!(std::fs::read(&original).unwrap(), b"synthetic fixture bytes");
        }
    }
}

#[test]
fn updates_navigation_preserves_library_search_and_checks_only_on_request() {
    let mut fixture = support::fixture();
    fixture.catalog.search("Synthetic".into());
    let selected = fixture.catalog.selected.clone();
    let ctx = egui::Context::default();
    ts4_mod_manager_native::ui::setup(&ctx);
    frame(&ctx, &mut fixture, vec![]);
    frame(&ctx, &mut fixture, vec![]);
    let updates = fixture.view.stats.updates_tab.unwrap();
    frame(&ctx, &mut fixture, click(updates));
    let output = frame(&ctx, &mut fixture, vec![]);
    assert!(has_text(&output, "Updates"));
    assert!(has_text(&output, "Choose installed release"));
    assert!(!fixture.controls.updates.pending);
    assert_eq!(fixture.catalog.query, "Synthetic");
    assert_eq!(fixture.catalog.selected, selected);
    let library = fixture.view.stats.library_tab.unwrap();
    frame(&ctx, &mut fixture, click(library));
    frame(&ctx, &mut fixture, vec![]);
    assert!(fixture.view.stats.search.is_some());
    assert_eq!(fixture.catalog.query, "Synthetic");
}

#[test]
fn add_mods_menu_exposes_folder_and_zip_without_starting_an_operation() {
    let mut fixture = support::fixture();
    let ctx = egui::Context::default();
    ts4_mod_manager_native::ui::setup(&ctx);
    frame(&ctx, &mut fixture, vec![]);
    let output = frame(&ctx, &mut fixture, vec![]);
    let add = output.shapes.iter().find_map(|shape| match &shape.shape {
        egui::Shape::Text(text) if text.galley.text() == "Add mods" => Some(egui::Rect::from_min_size(text.pos, text.galley.size())),
        _ => None,
    }).expect("primary Add mods action");
    frame(&ctx, &mut fixture, click(add));
    frame(&ctx, &mut fixture, vec![]);
    let output = frame(&ctx, &mut fixture, vec![]);
    assert!(has_text(&output, "Import folder"));
    assert!(has_text(&output, "Import ZIP"));
    assert!(!fixture.controls.busy());
    assert!(!fixture.controls.review_open());
}

#[test]
fn updates_page_has_one_working_review_per_mod() {
    let mut fixture = support::fixture();
    let ctx = egui::Context::default();
    ts4_mod_manager_native::ui::setup(&ctx);
    frame(&ctx, &mut fixture, vec![]);
    frame(&ctx, &mut fixture, vec![]);
    let updates = fixture.view.stats.updates_tab.unwrap();
    frame(&ctx, &mut fixture, click(updates));
    let output = frame(&ctx, &mut fixture, vec![]);
    let reviews: Vec<_> = output.shapes.iter().filter_map(|shape| match &shape.shape {
        egui::Shape::Text(text) if text.galley.text() == "Review" => Some(egui::Rect::from_min_size(text.pos, text.galley.size())),
        _ => None,
    }).collect();
    assert_eq!(reviews.len(), 1, "only the linked mod has a Review action on Updates");
    frame(&ctx, &mut fixture, click(reviews[0]));
    frame(&ctx, &mut fixture, vec![]);
    assert!(fixture.view.stats.detail_close.is_some(), "Review opens that mod's details");
}

#[test]
fn a_blocked_switch_shows_the_problem_without_touching_the_existing_file() {
    let mut fixture = support::fixture();
    let existing = fixture.directory.path().join("game/Mods/fixture.package");
    std::fs::write(&existing, b"unmanaged original").unwrap();
    let ctx = egui::Context::default();
    ts4_mod_manager_native::ui::setup(&ctx);
    frame(&ctx, &mut fixture, vec![]);
    frame(&ctx, &mut fixture, vec![]);
    let switch = fixture.view.stats.first_toggle.unwrap();
    frame(&ctx, &mut fixture, click(switch));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while fixture.controls.busy() {
        fixture.controls.poll(&fixture.catalog);
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    frame(&ctx, &mut fixture, vec![]);
    let out = frame(&ctx, &mut fixture, vec![]);
    assert!(!fixture.controls.review_open());
    assert!(has_text(&out, "Path collision"), "the failure must be visible without opening Activity");
    assert_eq!(std::fs::read(existing).unwrap(), b"unmanaged original");
}

#[test]
fn a_duplicate_content_warning_does_not_report_a_successful_switch_as_failed() {
    let mut fixture = support::fixture();
    std::fs::write(fixture.directory.path().join("game/Mods/other.package"), b"synthetic fixture bytes").unwrap();
    let ctx = egui::Context::default();
    ts4_mod_manager_native::ui::setup(&ctx);
    frame(&ctx, &mut fixture, vec![]);
    frame(&ctx, &mut fixture, vec![]);
    let switch = fixture.view.stats.first_toggle.unwrap();
    frame(&ctx, &mut fixture, click(switch));
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(3);
    while fixture.controls.busy() {
        fixture.controls.poll(&fixture.catalog);
        assert!(std::time::Instant::now() < deadline);
        std::thread::sleep(std::time::Duration::from_millis(2));
    }
    assert!(fixture.controls.error.is_none(), "the change applied successfully");
    frame(&ctx, &mut fixture, vec![]);
    let out = frame(&ctx, &mut fixture, vec![]);
    assert!(has_text(&out, "Mod enabled"));
    assert!(has_text(&out, "Duplicate content"), "keep the nonblocking warning visible");
}
