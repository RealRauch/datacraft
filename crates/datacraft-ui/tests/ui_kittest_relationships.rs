//! Relationship mapper affordances (step 41.1, item M45).
//!
//! The mapper pane dispatches exactly one write command per flow over the
//! real bus: a per-relationship delete affordance and the create form's
//! four pickers (one `CreateRelationship`). The capability-restricted
//! rendering pins the same availability map the CSV driver feeds —
//! `MockPort` simulates the restricted driver (phase-30 precedent).
//! Assertions stay on the snapshot and the semantic tree — never pixels.

use datacraft_core::command::Command;
use datacraft_core::port::{DriverCapabilities, MockPort, Relationship, Support};
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_ui::app::App;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
        ],
    )
}

fn orders() -> TableDef {
    TableDef::new(
        "Orders",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("customer_id", FieldKind::Number(NumericKind::LongInteger)),
        ],
    )
}

fn relationship() -> Relationship {
    Relationship {
        table: "Customers".into(),
        column: "id".into(),
        related_table: "Orders".into(),
        related_column: "customer_id".into(),
    }
}

/// App with both tables and the mapper open over `relationships`. When
/// `restricted`, the mock reports the CSV driver's relationship surface
/// (`Support::No`).
fn mapper_app(relationships: Vec<Relationship>, restricted: bool) -> App {
    let mut port = MockPort::with_tables(vec![customers(), orders()]);
    port.with_relationships(relationships);
    let mut caps = DriverCapabilities::full();
    caps.relationships = if restricted {
        Support::No
    } else {
        Support::Yes
    };
    port.with_capabilities(caps);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::ToggleRelationshipMapper);
    app
}

fn mapper_harness(app: App) -> Harness<'static, App> {
    Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            datacraft_ui::relationships::mapper_window(app, ui);
        },
        app,
    )
}

/// Semantic tree dump: role + label + value + disabled state of every
/// labelled node, in traversal order. Deterministic across DPI and fonts.
fn tree_dump<State>(harness: &Harness<'_, State>) -> String {
    let mut lines = Vec::new();
    for node in harness.root().children_recursive() {
        let node = node.accesskit_node();
        let label = node.label().unwrap_or_default();
        let value = node.value().unwrap_or_default();
        if label.is_empty() && value.is_empty() {
            continue;
        }
        let disabled = if node.is_disabled() { " disabled" } else { "" };
        lines.push(format!("{:?}|{label}|{value}{disabled}", node.role()));
    }
    lines.join("\n")
}

/// One delete affordance per listed relationship; clicking one dispatches
/// exactly one `DeleteRelationship` for that row — the untouched
/// relationship survives, no failure reaches the status bar.
#[test]
fn the_delete_action_removes_exactly_the_clicked_relationship() {
    let other = Relationship {
        table: "Customers".into(),
        column: "id".into(),
        related_table: "Orders".into(),
        related_column: "id".into(),
    };
    let mut harness = mapper_harness(mapper_app(vec![relationship(), other], false));
    harness.run();
    let snapshot = harness
        .state()
        .snapshot()
        .database
        .clone()
        .expect("database open");
    assert_eq!(snapshot.relationships.len(), 2);

    let delete_buttons: Vec<_> = harness.get_all_by_label("Delete").collect();
    assert_eq!(
        delete_buttons.len(),
        2,
        "one delete affordance per listed relationship"
    );
    delete_buttons[1].click();
    harness.run();

    let snapshot = harness
        .state()
        .snapshot()
        .database
        .clone()
        .expect("database open");
    assert_eq!(
        snapshot.relationships,
        vec![relationship()],
        "exactly the clicked relationship is gone"
    );
    assert!(harness.state().last_error().is_none());
}

/// The create form drives the four pickers over the snapshot's schema and
/// dispatches exactly one `CreateRelationship`; the snapshot then carries
/// the new relationship.
#[test]
fn the_create_form_dispatches_one_create_command() {
    let mut harness = mapper_harness(mapper_app(vec![], false));
    harness.run();

    harness.get_by_label("New Relationship").click();
    harness.run();
    harness.get_by_label("Referencing table").click();
    harness.run();
    harness.get_by_label("Customers").click();
    harness.run();
    harness.get_by_label("Referencing field").click();
    harness.run();
    harness.get_by_label("id").click();
    harness.run();
    harness.get_by_label("Referenced table").click();
    harness.run();
    harness.get_by_label("Orders").click();
    harness.run();
    harness.get_by_label("Referenced field").click();
    harness.run();
    harness.get_by_label("customer_id").click();
    harness.run();

    harness.get_by_label("Create Relationship").click();
    harness.run();

    let snapshot = harness
        .state()
        .snapshot()
        .database
        .clone()
        .expect("database open");
    assert_eq!(
        snapshot.relationships,
        vec![relationship()],
        "the picked relationship reached the write path"
    );
    assert!(harness.state().last_error().is_none());
}

/// A `Support::No` surface (the CSV driver's declaration) disables the
/// create and delete affordances with the localized reason — the same
/// availability map the real driver feeds.
#[test]
fn a_no_relationships_source_disables_the_mapper_affordances() {
    let app = mapper_app(vec![relationship()], true);
    let availability = &app
        .snapshot()
        .database
        .as_ref()
        .expect("database open")
        .availability;
    for id in [
        "relationships",
        "relationships.create",
        "relationships.delete",
    ] {
        assert!(
            availability.contains_key(id),
            "affordance {id:?} must be restricted"
        );
    }

    let mut harness = mapper_harness(app);
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("Button|New Relationship| disabled"),
        "the create affordance renders disabled:\n{dump}"
    );
    assert!(
        dump.contains("Button|Delete| disabled"),
        "the delete affordance renders disabled:\n{dump}"
    );
}
