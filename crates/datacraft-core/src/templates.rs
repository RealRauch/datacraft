//! Shipped database templates (phase 29, item H44).
//!
//! Templates are core-owned, deterministic definitions: every table and
//! row is fixed data, so a template materializes identically everywhere —
//! the sample generator pins the bytes with `--check` (step 29.3). The
//! registry is the standard set: `blank`, `contacts`, `inventory` and the
//! `bike_shop` demo.

use serde::{Deserialize, Serialize};

use crate::port::DatabaseFormat;
use crate::schema::{AutoKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef};

/// One table of a template: its definition plus the seed rows.
#[derive(Debug, Clone)]
pub struct TemplateTable {
    /// Table definition (field kinds, properties).
    pub definition: TableDef,
    /// Seed rows in insertion order; `Null` on an AutoNumber field means
    /// engine-assigned.
    pub rows: Vec<Vec<FieldValue>>,
}

/// A shipped database template.
#[derive(Debug, Clone)]
pub struct DatabaseTemplate {
    /// Registry name (the command's `template` argument).
    pub name: &'static str,
    /// Human-facing title.
    pub title: &'static str,
    /// Format family of the materialized file.
    pub format: DatabaseFormat,
    /// Tables in creation order.
    pub tables: Vec<TemplateTable>,
}

/// Serializable template summary for snapshots (the wire-visible registry).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateSummary {
    /// Registry name.
    pub name: String,
    /// Human-facing title.
    pub title: String,
    /// Format family of the materialized file.
    pub format: DatabaseFormat,
    /// Number of tables the template creates.
    pub tables: usize,
}

/// The shipped template registry (order = backstage listing order).
pub fn templates() -> Vec<DatabaseTemplate> {
    vec![blank(), contacts(), inventory(), bike_shop()]
}

/// Resolves a template by its registry name (case-insensitive).
pub fn find_template(name: &str) -> Option<DatabaseTemplate> {
    templates()
        .into_iter()
        .find(|template| template.name.eq_ignore_ascii_case(name))
}

/// Snapshot summaries of every shipped template.
pub fn summaries() -> Vec<TemplateSummary> {
    templates()
        .into_iter()
        .map(|template| TemplateSummary {
            name: template.name.to_owned(),
            title: template.title.to_owned(),
            format: template.format,
            tables: template.tables.len(),
        })
        .collect()
}

/// The empty database: a fresh file with no tables.
fn blank() -> DatabaseTemplate {
    DatabaseTemplate {
        name: "blank",
        title: "Blank database",
        format: DatabaseFormat::Jet4,
        tables: Vec::new(),
    }
}

/// Contact records: one table, a few people.
fn contacts() -> DatabaseTemplate {
    let table = TableDef::new(
        "Contacts",
        vec![
            autonumber("Id"),
            required(text("FullName", 60)),
            text("Email", 80),
            text("Phone", 20),
            FieldDef::new("Birthday", FieldKind::DateTime),
        ],
    );
    let rows = vec![
        vec![
            nul(),
            text_value("Ada Lovelace"),
            text_value("ada@example.com"),
            text_value("+44 20 7946 0958"),
            FieldValue::DateTime(8_762.0),
        ],
        vec![
            nul(),
            text_value("Grace Hopper"),
            text_value("grace@example.com"),
            text_value("+1 212 555 0173"),
            FieldValue::DateTime(13_679.0),
        ],
        vec![
            nul(),
            text_value("Edsger Dijkstra"),
            text_value("edsger@example.com"),
            text_value("+31 15 278 9111"),
            nul(),
        ],
        vec![
            nul(),
            text_value("Margaret Hamilton"),
            text_value("margaret@example.com"),
            text_value("+1 617 555 0142"),
            FieldValue::DateTime(13_140.0),
        ],
    ];
    DatabaseTemplate {
        name: "contacts",
        title: "Contacts",
        format: DatabaseFormat::Jet4,
        tables: vec![TemplateTable {
            definition: table,
            rows,
        }],
    }
}

/// Inventory: categories and stocked items.
fn inventory() -> DatabaseTemplate {
    let categories = TableDef::new(
        "Categories",
        vec![autonumber("Id"), required(text("Name", 40))],
    );
    let category_rows = vec![
        vec![nul(), text_value("Frames")],
        vec![nul(), text_value("Wheels")],
        vec![nul(), text_value("Apparel")],
        vec![nul(), text_value("Tools")],
    ];
    let items = TableDef::new(
        "Items",
        vec![
            autonumber("Id"),
            required(text("Name", 60)),
            text("Category", 40),
            FieldDef::new("Stock", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("Price", FieldKind::Currency),
        ],
    );
    let item_rows = vec![
        vec![
            nul(),
            text_value("Road frame 56"),
            text_value("Frames"),
            int(7),
            money(45_900_000),
        ],
        vec![
            nul(),
            text_value("Road frame 54"),
            text_value("Frames"),
            int(3),
            money(45_900_000),
        ],
        vec![
            nul(),
            text_value("Wheelset DT Swiss"),
            text_value("Wheels"),
            int(11),
            money(89_900_000),
        ],
        vec![
            nul(),
            text_value("Winter jacket"),
            text_value("Apparel"),
            int(24),
            money(14_900_000),
        ],
        vec![
            nul(),
            text_value("Torque wrench"),
            text_value("Tools"),
            int(9),
            money(7_900_000),
        ],
        vec![
            nul(),
            text_value("Cable set"),
            text_value("Tools"),
            int(31),
            money(2_400_000),
        ],
    ];
    DatabaseTemplate {
        name: "inventory",
        title: "Inventory",
        format: DatabaseFormat::Jet4,
        tables: vec![
            TemplateTable {
                definition: categories,
                rows: category_rows,
            },
            TemplateTable {
                definition: items,
                rows: item_rows,
            },
        ],
    }
}

/// The Bike Shop demo: three related tables plus the `.datacraft` demo
/// sidecar (form + query) the generator writes next to the databases.
fn bike_shop() -> DatabaseTemplate {
    let bikes = TableDef::new(
        "Bikes",
        vec![
            autonumber("Id"),
            required(text("Model", 60)),
            text("FrameSize", 4),
            FieldDef::new("Price", FieldKind::Currency),
            FieldDef::new("InStock", FieldKind::Boolean { tri_state: false }),
            FieldDef::new("Added", FieldKind::DateTime),
        ],
    );
    let bike_rows = vec![
        vec![
            nul(),
            text_value("Alpine Trail 29"),
            text_value("L"),
            money(329_900_000),
            yes(),
            date(45_658.0),
        ],
        vec![
            nul(),
            text_value("Canyon Road 7"),
            text_value("M"),
            money(279_900_000),
            yes(),
            date(45_665.0),
        ],
        vec![
            nul(),
            text_value("City Cruiser 3"),
            text_value("S"),
            money(129_900_000),
            no(),
            date(45_672.0),
        ],
        vec![
            nul(),
            text_value("Gravel Explorer"),
            text_value("M"),
            money(219_900_000),
            yes(),
            date(45_679.0),
        ],
        vec![
            nul(),
            text_value("E-Commute 5"),
            text_value("L"),
            money(389_900_000),
            no(),
            date(45_686.0),
        ],
        vec![
            nul(),
            text_value("Kids Runner 20"),
            text_value("Kids"),
            money(89_900_000),
            yes(),
            date(45_693.0),
        ],
    ];
    let customers = TableDef::new(
        "Customers",
        vec![
            autonumber("Id"),
            required(text("Name", 60)),
            text("Email", 80),
        ],
    );
    let customer_rows = vec![
        vec![
            nul(),
            text_value("Lena Fischer"),
            text_value("lena@example.com"),
        ],
        vec![
            nul(),
            text_value("Tobias Weber"),
            text_value("tobias@example.com"),
        ],
        vec![
            nul(),
            text_value("Mara Keller"),
            text_value("mara@example.com"),
        ],
        vec![
            nul(),
            text_value("Jonas Brand"),
            text_value("jonas@example.com"),
        ],
    ];
    let orders = TableDef::new(
        "ServiceOrders",
        vec![
            autonumber("Id"),
            FieldDef::new("BikeId", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("CustomerId", FieldKind::Number(NumericKind::LongInteger)),
            text("Description", 120),
            FieldDef::new("Done", FieldKind::Boolean { tri_state: false }),
        ],
    );
    let order_rows = vec![
        vec![nul(), int(1), int(1), text_value("Brake service"), yes()],
        vec![nul(), int(3), int(2), text_value("Gear indexing"), yes()],
        vec![
            nul(),
            int(5),
            int(3),
            text_value("Bottom bracket replacement"),
            no(),
        ],
    ];
    DatabaseTemplate {
        name: "bike_shop",
        title: "Bike Shop (demo)",
        format: DatabaseFormat::Jet4,
        tables: vec![
            TemplateTable {
                definition: bikes,
                rows: bike_rows,
            },
            TemplateTable {
                definition: customers,
                rows: customer_rows,
            },
            TemplateTable {
                definition: orders,
                rows: order_rows,
            },
        ],
    }
}

fn text(name: &str, max_len: u16) -> FieldDef {
    FieldDef::new(name, FieldKind::Text { max_len })
}

fn required(mut field: FieldDef) -> FieldDef {
    field.properties.required = true;
    field
}

fn autonumber(name: &str) -> FieldDef {
    FieldDef::new(name, FieldKind::AutoNumber(AutoKind::LongInteger))
}

fn nul() -> FieldValue {
    FieldValue::Null
}

fn text_value(value: &str) -> FieldValue {
    FieldValue::Text(value.to_owned())
}

fn int(value: i64) -> FieldValue {
    FieldValue::Integer(value)
}

/// Currency in its storage scale (10⁻⁴): `money(45_900_000)` = 4590.00.
fn money(value: i64) -> FieldValue {
    FieldValue::Currency(value)
}

fn yes() -> FieldValue {
    FieldValue::Boolean(true)
}

fn no() -> FieldValue {
    FieldValue::Boolean(false)
}

fn date(ole_days: f64) -> FieldValue {
    FieldValue::DateTime(ole_days)
}
