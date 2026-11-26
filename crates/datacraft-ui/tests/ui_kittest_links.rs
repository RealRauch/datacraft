//! Linked-table surface in the nav pane (phase 100, step 100.3; item
//! M131; roadmap §9 stage 0).
//!
//! The pane lists every link record read-only: the icon-and-label row
//! language of the native objects, the target on the same row, and a
//! group hint that names the boundary. No row opens a tab, no context
//! menu appears, and the search matches the name and the target.

use datacraft_core::command::Command;
use datacraft_core::port::{LinkKind, LinkedTable, MockPort};
use datacraft_core::schema::{FieldDef, FieldKind, TableDef};
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![FieldDef::new("Name", FieldKind::Text { max_len: 50 })],
    )
}

fn file_link() -> LinkedTable {
    LinkedTable {
        name: "Orders_Remote".into(),
        kind: LinkKind::AccessFile,
        source: Some("Z:\\data\\linkee.accdb".into()),
        connect: None,
        foreign_name: Some("Orders".into()),
        flags: 2_097_152,
    }
}

fn odbc_link() -> LinkedTable {
    LinkedTable {
        name: "Billing_Remote".into(),
        kind: LinkKind::Odbc,
        source: None,
        connect: Some("DSN=billing;UID=reader;PWD=***;DATABASE=sales".into()),
        foreign_name: Some("dbo.Billing".into()),
        flags: 537_919_488,
    }
}

/// App with one real table and both link kinds; the nav pane renders.
fn links_app() -> App {
    let mut port = MockPort::with_tables(vec![customers()]);
    port.with_linked_tables(vec![file_link(), odbc_link()]);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app
}

fn nav_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::nav_pane::nav_pane_ui(app, ui);
        },
        app,
    )
}

#[test]
fn the_nav_pane_lists_both_links_with_their_targets() {
    let harness = nav_harness(links_app());
    // The group header counts the links.
    assert!(
        harness
            .query_by_label_contains("Linked tables (2)")
            .is_some(),
        "the group names its count"
    );
    // Both names are rows, each with its one-line target.
    assert!(harness.query_by_label("Orders_Remote").is_some());
    assert!(
        harness
            .query_by_label_contains("Z:\\data\\linkee.accdb")
            .is_some(),
        "the file link names its external database"
    );
    assert!(harness.query_by_label("Billing_Remote").is_some());
    assert!(
        harness.query_by_label_contains("DSN=billing").is_some(),
        "the ODBC link names its connect fragment, credentials redacted"
    );
    // The boundary is stated, and no credential fragment renders.
    assert!(
        harness
            .query_by_label_contains("the targets are not resolved")
            .is_some(),
        "the group hint names the read-only boundary"
    );
    assert!(
        harness.query_by_label_contains("PWD=").is_none(),
        "no password fragment renders"
    );
}

#[test]
fn a_link_row_opens_no_tab() {
    let mut harness = nav_harness(links_app());
    harness.run();
    harness.get_by_label("Orders_Remote").click();
    harness.run();
    let snapshot = harness.state().snapshot().clone();
    assert!(
        snapshot.tabs.is_empty(),
        "a link row is read-only: {:?}",
        snapshot.tabs
    );
    assert_eq!(snapshot.active_tab, None);
}

#[test]
fn the_search_matches_a_link_target() {
    let mut harness = nav_harness(links_app());
    harness.run();
    harness.state_mut().nav_search = "linkee".to_owned();
    harness.run();
    assert!(harness.query_by_label("Orders_Remote").is_some());
    assert!(
        harness.query_by_label("Billing_Remote").is_none(),
        "the ODBC row does not match a file path"
    );
}
