//! In-memory table renames for `MockPort` (phase 72, step 72.1; item M81).
//!
//! Mirrors the physical rename: the catalog entry changes, row storage
//! moves to the new key and in-memory relationships follow both sides.

use super::MockPort;
use crate::port::PortError;

/// Renames one mock table in place, returning the canonical new name.
pub(super) fn rename(
    port: &mut MockPort,
    table: &str,
    new_name: &str,
) -> Result<String, PortError> {
    if let Some(detail) = &port.write_fail {
        return Err(PortError::WriteFailed {
            detail: detail.clone(),
        });
    }
    let position = port
        .tables
        .iter()
        .position(|candidate| candidate.name.eq_ignore_ascii_case(table))
        .ok_or_else(|| PortError::TableNotFound(table.to_owned()))?;
    let old_name = port.tables[position].name.clone();
    if port.tables.iter().enumerate().any(|(index, candidate)| {
        index != position && candidate.name.eq_ignore_ascii_case(new_name)
    }) {
        return Err(PortError::WriteFailed {
            detail: format!("duplicate table name {new_name:?}"),
        });
    }
    port.tables[position].name = new_name.to_owned();
    if let Some(rows) = port.rows.remove(&old_name) {
        port.rows.insert(new_name.to_owned(), rows);
    }
    for relationship in &mut port.relationships {
        if relationship.table.eq_ignore_ascii_case(&old_name) {
            relationship.table = new_name.to_owned();
        }
        if relationship.related_table.eq_ignore_ascii_case(&old_name) {
            relationship.related_table = new_name.to_owned();
        }
    }
    Ok(new_name.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::port::Relationship;
    use crate::schema::{FieldDef, FieldKind, NumericKind, TableDef};

    fn mock() -> MockPort {
        let mut port = MockPort::with_tables(vec![TableDef::new(
            "T",
            vec![FieldDef::new(
                "id",
                FieldKind::Number(NumericKind::LongInteger),
            )],
        )]);
        port.rows.insert(
            "T".to_owned(),
            vec![vec![crate::schema::FieldValue::Integer(1)]],
        );
        port
    }

    #[test]
    fn rename_moves_rows_and_relationships() {
        let mut port = mock();
        port.with_relationships(vec![Relationship {
            table: "T".into(),
            column: "id".into(),
            related_table: "T".into(),
            related_column: "id".into(),
        }]);
        let canonical = rename(&mut port, "t", "Renamed").expect("rename");
        assert_eq!(canonical, "Renamed");
        assert_eq!(port.tables[0].name, "Renamed");
        assert!(port.rows.contains_key("Renamed"));
        assert!(!port.rows.contains_key("T"));
        assert_eq!(port.relationships[0].table, "Renamed");
        assert_eq!(port.relationships[0].related_table, "Renamed");
    }

    #[test]
    fn unknown_and_duplicate_names_are_precise() {
        let mut port = mock();
        let unknown = rename(&mut port, "ghost", "X").expect_err("unknown");
        assert!(
            matches!(unknown, PortError::TableNotFound(_)),
            "{unknown:?}"
        );

        port.tables.push(TableDef::new(
            "Other",
            vec![FieldDef::new(
                "id",
                FieldKind::Number(NumericKind::LongInteger),
            )],
        ));
        let duplicate = rename(&mut port, "T", "other").expect_err("duplicate");
        assert!(duplicate.to_string().contains("duplicate"), "{duplicate}");
    }

    #[test]
    fn write_failures_are_reported() {
        let mut port = mock();
        port.write_fail = Some("disk full".to_owned());
        let error = rename(&mut port, "T", "U").expect_err("write failure");
        assert!(error.to_string().contains("disk full"), "{error}");
    }
}
