//! Report designer grouping picker (step 41.4, item M48).
//!
//! The designer's grouping picker lists the record source's fields plus
//! a "no grouping" entry and dispatches exactly one `SetReportGrouping`
//! per change. Both directions are pinned: the snapshot carries the new
//! grouping and the cached preview re-renders the group bands from the
//! refreshed snapshot.

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::report_preview::Band;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_ui::app::App;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn orders() -> TableDef {
    TableDef::new(
        "Orders",
        vec![
            FieldDef::new("customer_id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("total", FieldKind::Currency),
        ],
    )
}

/// App with an open report-design tab over a sourced, ungrouped report.
fn grouping_app() -> (App, datacraft_core::snapshot::TabSnapshot) {
    let mut rows: Vec<Vec<datacraft_core::schema::FieldValue>> = (0..4)
        .map(|index| {
            vec![
                datacraft_core::schema::FieldValue::Integer(index % 2),
                datacraft_core::schema::FieldValue::Currency(100 + index),
            ]
        })
        .collect();
    rows[0][0] = datacraft_core::schema::FieldValue::Integer(1);
    let mut port = MockPort::with_tables(vec![orders()]);
    port.with_row_data("Orders", rows);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::CreateReport {
        name: "Roster".into(),
    });
    app.set_record_source(datacraft_core::ObjectKind::Report, "Roster", "Orders");
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Report,
        "Roster",
    )));
    let tab = app.snapshot().tabs[0].clone();
    app.dispatch(Command::SetTabMode {
        tab_id: tab.id,
        mode: datacraft_core::state::TabMode::Design,
    });
    (app, tab)
}

fn design_harness(
    app: App,
    tab: datacraft_core::snapshot::TabSnapshot,
) -> Harness<'static, (App, datacraft_core::snapshot::TabSnapshot)> {
    Harness::new_ui_state(
        move |ui, (app, tab)| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::report_design::report_design_ui(app, tab, ui);
        },
        (app, tab),
    )
}

fn group_bands(harness: &Harness<'_, (App, datacraft_core::snapshot::TabSnapshot)>) -> usize {
    harness
        .state()
        .0
        .preview_pages(harness.state().1.id)
        .iter()
        .filter(|page| matches!(page.band, Band::GroupHeader))
        .count()
}

/// Picking a field dispatches one SetReportGrouping; the snapshot
/// carries it and the preview gains group-header bands.
#[test]
fn picking_a_field_sets_the_grouping_and_renders_the_bands() {
    let (app, tab) = grouping_app();
    assert!(
        app.snapshot().database.as_ref().expect("open").reports[0]
            .group_by
            .is_none()
    );
    let mut harness = design_harness(app, tab);
    // A wide virtual screen: the design view lays the canvas (595 pt)
    // and the palette column side by side, so the picker sits past a
    // 1024-px viewport and the popup would be culled.
    harness.set_size(egui::vec2(1400.0, 1000.0));
    harness.run();

    harness.get_by_label("Grouping").click();
    harness.run();
    harness.get_by_label("customer_id").click();
    harness.run();

    let snapshot = harness.state().0.snapshot();
    assert_eq!(
        snapshot.database.as_ref().expect("open").reports[0].group_by,
        Some("customer_id".to_owned()),
        "the picked field reached the bus"
    );
    assert!(
        group_bands(&harness) > 0,
        "the preview re-renders group-header bands"
    );
    assert!(harness.state().0.last_error().is_none());
}

/// The "no grouping" entry clears the grouping; the preview drops the
/// group bands again.
#[test]
fn the_no_grouping_entry_clears_the_grouping() {
    let (app, tab) = grouping_app();
    let mut harness = design_harness(app, tab);
    harness.set_size(egui::vec2(1400.0, 1000.0));
    harness.run();
    harness.get_by_label("Grouping").click();
    harness.run();
    harness.get_by_label("customer_id").click();
    harness.run();
    assert!(group_bands(&harness) > 0, "precondition: grouped");

    harness.get_by_label("Grouping").click();
    harness.run();
    harness.get_by_label("no grouping").click();
    harness.run();

    let snapshot = harness.state().0.snapshot();
    assert_eq!(
        snapshot.database.as_ref().expect("open").reports[0].group_by,
        None,
        "the clear entry reached the bus"
    );
    assert_eq!(
        group_bands(&harness),
        0,
        "the preview drops the group bands"
    );
    assert!(harness.state().0.last_error().is_none());
}
