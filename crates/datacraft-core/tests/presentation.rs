//! Contract suite for presentation helpers (step 5.1, item H17).
//!
//! ARCHITECTURE §8: no layout or business logic in paint code — the UI
//! renders strings and models computed here, so they are unit-testable
//! without a window.

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::port::MockPort;
use datacraft_core::present::status_line;
use datacraft_core::schema::{FieldDef, FieldKind, TableDef};

fn open_core(path: &str) -> Core {
    let mut core = Core::new(Box::new(MockPort::new()));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from(path),
    });
    core
}

fn add_table(core: &mut Core, name: &str) {
    core.dispatch(Command::CreateTable {
        definition: TableDef::new(name, vec![FieldDef::new("id", FieldKind::Currency)]),
    });
}

#[test]
fn closed_core_reports_no_database() {
    let snapshot = Core::new(Box::new(MockPort::new())).inspect();
    assert_eq!(status_line(&snapshot, None), "No database open");
}

#[test]
fn status_context_prefers_the_active_object_over_the_table_count() {
    let mut core = open_core("shop.mdb");
    let snapshot = core.inspect();
    assert_eq!(status_line(&snapshot, None), "shop (Jet4) — 0 tables");

    add_table(&mut core, "Customers");
    let snapshot = core.inspect();
    assert_eq!(status_line(&snapshot, None), "shop (Jet4) — 1 table");

    // An active document replaces the generic count with its context.
    core.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Table,
        "Customers",
    )));
    let snapshot = core.inspect();
    let active = snapshot
        .active_tab
        .and_then(|id| snapshot.tabs.iter().find(|t| t.id == id))
        .map(|t| (t.object_kind.clone(), t.object_name.clone()));
    assert_eq!(
        status_line(
            &snapshot,
            active.as_ref().map(|(k, n)| (k.as_str(), n.as_str()))
        ),
        "shop (Jet4) — Customers"
    );
    assert_eq!(
        status_line(&snapshot, Some(("report", "Weekly"))),
        "shop (Jet4) — Weekly"
    );

    add_table(&mut core, "Orders");
    assert_eq!(status_line(&core.inspect(), None), "shop (Jet4) — 2 tables");
}

#[test]
fn format_names_are_human_readable() {
    assert_eq!(
        datacraft_core::port::DatabaseFormat::Jet3.to_string(),
        "Jet3"
    );
    assert_eq!(
        datacraft_core::port::DatabaseFormat::Jet4.to_string(),
        "Jet4"
    );
    assert_eq!(datacraft_core::port::DatabaseFormat::Ace.to_string(), "Ace");
}

#[test]
fn tab_placeholders_name_the_object_and_the_arriving_phase() {
    use datacraft_core::ObjectKind;
    use datacraft_core::present::tab_placeholder;

    assert_eq!(
        tab_placeholder(ObjectKind::Table, "Customers"),
        "Datasheet for \"Customers\" arrives in Phase 6."
    );
    assert_eq!(
        tab_placeholder(ObjectKind::Query, "Active Orders"),
        "Query results for \"Active Orders\" arrive in Phase 7."
    );
    assert_eq!(
        tab_placeholder(ObjectKind::Form, "Customer Card"),
        "Form view for \"Customer Card\" arrives in Phase 8."
    );
    assert_eq!(
        tab_placeholder(ObjectKind::Report, "Invoices"),
        "Report preview for \"Invoices\" arrives in Phase 8."
    );
}

#[test]
fn display_values_are_user_renderable() {
    use datacraft_core::present::display_value;
    use datacraft_core::schema::FieldValue;

    assert_eq!(display_value(&FieldValue::Null), "");
    assert_eq!(display_value(&FieldValue::Boolean(true)), "Yes");
    assert_eq!(display_value(&FieldValue::Boolean(false)), "No");
    assert_eq!(display_value(&FieldValue::Integer(-42)), "-42");
    assert_eq!(display_value(&FieldValue::Float(1.5)), "1.5");
    assert_eq!(display_value(&FieldValue::Currency(123_456)), "12.3456");
    assert_eq!(display_value(&FieldValue::Currency(-10_000)), "-1");
    assert_eq!(
        display_value(&FieldValue::DateTime(45_657.5)),
        "2024-12-31 12:00"
    );
    assert_eq!(display_value(&FieldValue::Decimal(-9)), "-9");
    assert_eq!(display_value(&FieldValue::Text("hi ✓".into())), "hi ✓");
    assert_eq!(
        display_value(&FieldValue::Binary(vec![1, 2, 3])),
        "<binary: 3 bytes>"
    );
    assert_eq!(
        display_value(&FieldValue::Guid([0u8; 16])),
        "00000000-0000-0000-0000-000000000000"
    );
}

#[test]
fn field_kinds_display_access_ui_names() {
    use datacraft_core::present::field_kind_display;
    use datacraft_core::schema::{AutoKind, BinaryKind, FieldKind, NumericKind};

    assert_eq!(
        field_kind_display(&FieldKind::Text { max_len: 255 }),
        "Short Text"
    );
    assert_eq!(field_kind_display(&FieldKind::Memo), "Long Text");
    assert_eq!(
        field_kind_display(&FieldKind::Number(NumericKind::Byte)),
        "Byte"
    );
    assert_eq!(
        field_kind_display(&FieldKind::Number(NumericKind::Integer)),
        "Integer"
    );
    assert_eq!(
        field_kind_display(&FieldKind::Number(NumericKind::LongInteger)),
        "Long Integer"
    );
    assert_eq!(
        field_kind_display(&FieldKind::Number(NumericKind::Single)),
        "Single"
    );
    assert_eq!(
        field_kind_display(&FieldKind::Number(NumericKind::Double)),
        "Double"
    );
    assert_eq!(
        field_kind_display(&FieldKind::Number(NumericKind::Decimal {
            precision: 18,
            scale: 2
        })),
        "Decimal (18, 2)"
    );
    assert_eq!(field_kind_display(&FieldKind::DateTime), "Date/Time");
    assert_eq!(field_kind_display(&FieldKind::Currency), "Currency");
    assert_eq!(
        field_kind_display(&FieldKind::AutoNumber(AutoKind::LongInteger)),
        "AutoNumber"
    );
    assert_eq!(
        field_kind_display(&FieldKind::AutoNumber(AutoKind::ReplicationId)),
        "AutoNumber (Replication ID)"
    );
    assert_eq!(
        field_kind_display(&FieldKind::Boolean { tri_state: true }),
        "Yes/No"
    );
    assert_eq!(
        field_kind_display(&FieldKind::Binary(BinaryKind::OleObject)),
        "OLE Object"
    );
    assert_eq!(
        field_kind_display(&FieldKind::Binary(BinaryKind::Attachment)),
        "Attachment"
    );
    assert_eq!(field_kind_display(&FieldKind::Guid), "GUID");
}
