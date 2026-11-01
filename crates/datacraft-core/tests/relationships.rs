//! Contract suite for relationship reads (step 7.1, item H25).
//!
//! The port exposes relationships as first-class domain data; the state
//! caches them on open (dropping a table drops its relationships) and the
//! snapshot carries them so the mapper pane and agents render from the
//! same seam.

use std::path::PathBuf;

use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::port::{MockPort, Relationship, StoragePort as _};

fn relationship(
    table: &str,
    column: &str,
    related_table: &str,
    related_column: &str,
) -> Relationship {
    Relationship {
        table: table.to_owned(),
        column: column.to_owned(),
        related_table: related_table.to_owned(),
        related_column: related_column.to_owned(),
    }
}

#[test]
fn mock_relationships_seed_and_read() {
    let mut port = MockPort::new();
    port.with_relationships(vec![relationship(
        "Orders",
        "customer_id",
        "Customers",
        "id",
    )]);
    let rels = port.relationships().expect("read");
    assert_eq!(rels.len(), 1);
    assert_eq!(rels[0].table, "Orders");
    assert_eq!(rels[0].related_column, "id");
}

#[test]
fn fresh_ports_have_no_relationships() {
    let port = MockPort::new();
    assert!(port.relationships().expect("read").is_empty());
}

#[test]
fn the_snapshot_carries_relationships() {
    let mut port = MockPort::new();
    port.with_relationships(vec![
        relationship("Orders", "customer_id", "Customers", "id"),
        relationship("Orders", "product_id", "Products", "id"),
    ]);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
    });
    let snap = core.inspect();
    let db = snap.database.expect("open");
    assert_eq!(db.relationships.len(), 2);
    assert_eq!(db.relationships[0].column, "customer_id");
}

#[test]
fn dropping_a_table_drops_its_relationships() {
    use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
    let mut port = MockPort::new();
    port.with_relationships(vec![
        relationship("Orders", "customer_id", "Customers", "id"),
        relationship("Orders", "product_id", "Products", "id"),
    ]);
    let mut core = Core::new(Box::new(port));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
    });
    for name in ["Orders", "Customers", "Products"] {
        core.dispatch(Command::CreateTable {
            definition: TableDef::new(
                name,
                vec![FieldDef::new(
                    "id",
                    FieldKind::Number(NumericKind::LongInteger),
                )],
            ),
        });
    }
    core.dispatch(Command::DropTable {
        table: "Orders".into(),
    });
    let db = core.inspect().database.expect("open");
    assert!(
        db.relationships.is_empty(),
        "Orders relationships die with the table"
    );
}

#[test]
fn relationships_survive_serialization() {
    use datacraft_core::port::Relationship;
    let rel = relationship("Orders", "customer_id", "Customers", "id");
    let json = serde_json::to_string(&rel).unwrap();
    let back: Relationship = serde_json::from_str(&json).unwrap();
    assert_eq!(back, rel);
}
