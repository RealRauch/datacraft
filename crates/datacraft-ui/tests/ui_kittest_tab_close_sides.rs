//! Tab-strip close-to-side pins (step 88.3, item L49).
//!
//! The context menu closes the tabs to one side of the addressed tab,
//! and re-activating a tab that carries pending edits keeps both the tab
//! and its staged batch (the idle-tab reuse policy never replaces).

use datacraft_core::command::Command;
use datacraft_core::ids::TabId;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::layout_probe;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn app_with_tables(names: &[&str]) -> App {
    let port = MockPort::with_tables(
        names
            .iter()
            .map(|name| {
                TableDef::new(
                    name.to_string(),
                    vec![FieldDef::new(
                        "Id",
                        FieldKind::Number(NumericKind::LongInteger),
                    )],
                )
            })
            .collect(),
    );
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "database.mdb".into(),
        password: None,
        source: None,
    });
    for name in names {
        app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
            datacraft_core::ObjectKind::Table,
            *name,
        )));
    }
    app
}

fn shell_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app: &mut App| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            layout_probe::clear();
            let mut frame = eframe::Frame::_new_kittest();
            eframe::App::ui(app, ui, &mut frame);
        },
        app,
    )
}

/// Right-clicks at the center of a recorded probe rect (the tab strip
/// paints one rect per tab block; a label query would collide with the
/// nav pane's row of the same object).
fn right_click_rect(harness: &mut Harness<'static, App>, probe_id: &str) {
    let pos = layout_probe::drain()
        .iter()
        .find(|(id, _)| id == probe_id)
        .map(|(_, rect)| rect.center())
        .unwrap_or_else(|| panic!("probe rect {probe_id:?} recorded"));
    harness.hover_at(pos);
    let press = |pressed| egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Secondary,
        pressed,
        modifiers: egui::Modifiers::default(),
    };
    harness.event(press(true));
    harness.run();
    harness.event(press(false));
    harness.run();
}

fn open_names(app: &App) -> Vec<String> {
    app.snapshot()
        .tabs
        .iter()
        .map(|tab| tab.object_name.clone())
        .collect()
}

#[test]
fn the_tab_menu_closes_the_tabs_on_each_side() {
    let mut harness = shell_harness(app_with_tables(&["Alpha", "Bravo", "Charlie", "Delta"]));
    harness.run();
    harness.run();

    // Close to the Left of Bravo: Alpha leaves, the active tab (Delta,
    // on the surviving side) stays active.
    right_click_rect(&mut harness, "workspace.tab.table.bravo");
    harness.get_by_label("Close Tabs to the Left").click();
    harness.run();
    let snapshot = harness.state().snapshot();
    assert_eq!(
        open_names(harness.state()),
        vec!["Bravo", "Charlie", "Delta"],
        "close to the left must close exactly the left-side tabs"
    );
    assert_eq!(
        snapshot.active_tab,
        snapshot.tabs.last().map(|tab| tab.id),
        "the active tab survived on the right side"
    );

    // Close to the Right of Bravo: Charlie and Delta leave, and the
    // anchor becomes active because the active tab was closed.
    right_click_rect(&mut harness, "workspace.tab.table.bravo");
    harness.get_by_label("Close Tabs to the Right").click();
    harness.run();
    let snapshot = harness.state().snapshot();
    assert_eq!(
        open_names(harness.state()),
        vec!["Bravo"],
        "close to the right must close exactly the right-side tabs"
    );
    assert_eq!(
        snapshot.active_tab,
        snapshot.tabs.first().map(|tab| tab.id),
        "the anchor tab became active after its right side closed"
    );
}

/// Re-activating an object whose tab carries a staged batch must
/// activate that tab and keep the batch — never open a second tab and
/// never replace the pending edits (step 88.3, item L49).
#[test]
fn reactivating_a_tab_with_pending_edits_keeps_the_staged_batch() {
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "Orders",
        vec![FieldDef::new(
            "Id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    )]);
    port.with_row_data("Orders", vec![vec![FieldValue::Integer(1)]]);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "database.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Orders",
    )));
    let mut harness = shell_harness(app);
    // The datasheet paints and loads its rows, then the batch is staged.
    harness.run();
    harness.run();
    let tab_id: TabId = harness.state().snapshot().tabs[0].id;
    assert!(
        harness
            .state_mut()
            .stage_row_insert(tab_id, vec![FieldValue::Text("new".to_owned())]),
        "the pin needs a staged insert"
    );
    assert!(harness.state().has_staged_edits(tab_id), "batch staged");

    harness
        .state_mut()
        .dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
            datacraft_core::ObjectKind::Table,
            "Orders",
        )));
    harness.run();
    let snapshot = harness.state().snapshot();
    assert_eq!(snapshot.tabs.len(), 1, "no second tab was opened");
    assert_eq!(
        snapshot.active_tab,
        Some(tab_id),
        "the existing tab was re-activated"
    );
    assert!(
        harness.state().has_staged_edits(tab_id),
        "the staged batch survived the re-activation"
    );
}
