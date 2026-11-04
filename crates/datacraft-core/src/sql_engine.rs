//! Shadow SQL engine (step 13.3, item H37).
//!
//! SQL runs over a throwaway in-memory SQLite database: the loaded tables
//! are materialized from the port and the statement executes there, so the
//! source files are never written and every call starts from a fresh
//! snapshot (deterministic, no stale caches).
//!
//! Agent safety gate: exactly one statement, and only `SELECT`/`WITH`.
//! Everything else is rejected before SQLite sees it.
//!
//! The engine sits behind the `sql-engine` feature (`rusqlite` is optional
//! in `datacraft-core`, step 31.2 / M19): the result types live in
//! [`crate::sql_types`] so the state and the wire vocabulary stay C-free.

use rusqlite::Connection;
use rusqlite::types::ValueRef;

use crate::schema::{AutoKind, FieldKind, FieldValue, NumericKind, TableDef};
pub use crate::sql_types::{Source, SqlResult};

/// Executes one `SELECT`/`WITH` statement over the materialized sources.
pub fn run_sql(sql: &str, sources: &[Source]) -> Result<SqlResult, String> {
    let statement = gate(sql)?;
    let conn = Connection::open_in_memory().map_err(|e| format!("shadow database: {e}"))?;
    for (definition, rows) in sources {
        materialize(&conn, definition, rows)?;
    }
    execute(&conn, statement)
}

/// Rejects empty input, non-`SELECT`/`WITH` statements and multiple
/// statements; strips the trailing terminator.
fn gate(sql: &str) -> Result<&str, String> {
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        return Err("empty SQL statement".to_owned());
    }
    let keyword = trimmed
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .next()
        .unwrap_or_default();
    if !keyword.eq_ignore_ascii_case("SELECT") && !keyword.eq_ignore_ascii_case("WITH") {
        return Err(format!(
            "only SELECT and WITH statements are allowed (found {})",
            if keyword.is_empty() {
                "no keyword".to_owned()
            } else {
                format!("{keyword:?}")
            }
        ));
    }
    let mut quote: Option<char> = None;
    for (index, c) in trimmed.char_indices() {
        match quote {
            Some(end) => {
                if c == end {
                    quote = None;
                }
            }
            None => match c {
                '\'' | '"' | '`' => quote = Some(c),
                '[' => quote = Some(']'),
                ';' if !trimmed[index + 1..].trim().is_empty() => {
                    return Err("only one statement is allowed per RunSql call".to_owned());
                }
                _ => {}
            },
        }
    }
    Ok(trimmed.trim_end_matches(|c: char| c == ';' || c.is_whitespace()))
}

/// Creates the table and fills it with the loaded rows.
fn materialize(
    conn: &Connection,
    definition: &TableDef,
    rows: &[Vec<FieldValue>],
) -> Result<(), String> {
    let columns: Vec<String> = definition
        .fields
        .iter()
        .map(|field| format!("{} {}", quote_ident(&field.name), affinity(&field.kind)))
        .collect();
    let create = format!(
        "CREATE TABLE {} ({})",
        quote_ident(&definition.name),
        columns.join(", ")
    );
    conn.execute_batch(&create)
        .map_err(|e| format!("materializing {:?} failed: {e}", definition.name))?;
    if rows.is_empty() {
        return Ok(());
    }
    let placeholders: Vec<String> = (1..=definition.fields.len())
        .map(|n| format!("?{n}"))
        .collect();
    let insert = format!(
        "INSERT INTO {} VALUES ({})",
        quote_ident(&definition.name),
        placeholders.join(", ")
    );
    let mut statement = conn
        .prepare(&insert)
        .map_err(|e| format!("materializing {:?} failed: {e}", definition.name))?;
    for (index, row) in rows.iter().enumerate() {
        if row.len() != definition.fields.len() {
            return Err(format!(
                "materializing {:?} failed: row {index} has {} values, expected {}",
                definition.name,
                row.len(),
                definition.fields.len()
            ));
        }
        let params: Vec<rusqlite::types::Value> = definition
            .fields
            .iter()
            .zip(row)
            .map(|(field, value)| sql_value(&field.kind, value))
            .collect();
        statement
            .execute(rusqlite::params_from_iter(params))
            .map_err(|e| format!("materializing {:?} failed: {e}", definition.name))?;
    }
    Ok(())
}

/// SQLite storage class for a domain kind (matching the SQLite backend's
/// declared-type round-trip).
fn affinity(kind: &FieldKind) -> &'static str {
    match kind {
        FieldKind::Text { .. } | FieldKind::Memo => "TEXT",
        FieldKind::Number(NumericKind::Byte | NumericKind::Integer | NumericKind::LongInteger)
        | FieldKind::AutoNumber(AutoKind::LongInteger | AutoKind::ReplicationId)
        | FieldKind::Boolean { .. }
        | FieldKind::Currency => "INTEGER",
        FieldKind::Number(NumericKind::Single | NumericKind::Double)
        | FieldKind::DateTime
        | FieldKind::Number(NumericKind::Decimal { .. }) => "REAL",
        // Complex cells are materialized as their joined display text
        // (the shadow SQL engine has no complex type; documented lossiness).
        FieldKind::MultiValue(_) | FieldKind::Complex => "TEXT",
        FieldKind::Binary(_) | FieldKind::Guid => "BLOB",
    }
}

/// Domain value -> SQLite parameter.
fn sql_value(kind: &FieldKind, value: &FieldValue) -> rusqlite::types::Value {
    use rusqlite::types::Value as Sql;
    match value {
        FieldValue::Null => Sql::Null,
        FieldValue::Boolean(b) => Sql::Integer(i64::from(*b)),
        FieldValue::Integer(i) => Sql::Integer(*i),
        FieldValue::Float(f) => Sql::Real(*f),
        FieldValue::Currency(c) => Sql::Integer(*c),
        FieldValue::DateTime(days) => Sql::Real(*days),
        FieldValue::Decimal(scaled) => {
            let scale = match kind {
                FieldKind::Number(NumericKind::Decimal { scale, .. }) => u32::from(*scale),
                _ => 0,
            };
            if scale == 0 {
                Sql::Integer(i64::try_from(*scaled).unwrap_or(i64::MAX))
            } else {
                Sql::Real(*scaled as f64 / 10f64.powi(scale as i32))
            }
        }
        FieldValue::Text(text) => Sql::Text(text.clone()),
        FieldValue::Binary(bytes) => Sql::Blob(bytes.clone()),
        FieldValue::Guid(bytes) => Sql::Blob(bytes.to_vec()),
        FieldValue::Complex(_) => Sql::Text(crate::present::display_value(value)),
    }
}

/// Runs the gated statement and maps its result rows.
fn execute(conn: &Connection, sql: &str) -> Result<SqlResult, String> {
    let mut statement = conn.prepare(sql).map_err(|e| format!("SQL error: {e}"))?;
    let columns: Vec<String> = statement.column_names().map(str::to_owned).collect();
    let mut rows = statement.query([]).map_err(|e| format!("SQL error: {e}"))?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().map_err(|e| format!("SQL error: {e}"))? {
        let mut values = Vec::with_capacity(columns.len());
        for index in 0..columns.len() {
            let value = row.get_ref(index).map_err(|e| format!("SQL error: {e}"))?;
            values.push(to_field_value(value));
        }
        out.push(values);
    }
    Ok(SqlResult { columns, rows: out })
}

/// Dynamic result mapping (expressions have no schema kind).
fn to_field_value(value: ValueRef<'_>) -> FieldValue {
    match value {
        ValueRef::Null => FieldValue::Null,
        ValueRef::Integer(i) => FieldValue::Integer(i),
        ValueRef::Real(f) => FieldValue::Float(f),
        ValueRef::Text(bytes) => FieldValue::Text(String::from_utf8_lossy(bytes).into_owned()),
        ValueRef::Blob(bytes) => FieldValue::Binary(bytes.to_vec()),
    }
}

/// Quotes a catalog identifier for the shadow DDL (LESSONS #5).
fn quote_ident(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{FieldDef, TableDef};

    #[test]
    fn quoting_escapes_embedded_double_quotes() {
        assert_eq!(quote_ident("plain"), "\"plain\"");
        assert_eq!(quote_ident("we\"ird"), "\"we\"\"ird\"");
    }

    /// Step 39.2 (item M36): the dialect fixture — a Currency column, a
    /// DateTime column and a text column.
    fn source() -> Source {
        let definition = TableDef::new(
            "Orders",
            vec![
                FieldDef::new("amount", FieldKind::Currency),
                FieldDef::new("stamp", FieldKind::DateTime),
                FieldDef::new("city", FieldKind::Text { max_len: 50 }),
            ],
        );
        let rows = vec![
            vec![
                FieldValue::Currency(7_230_000),
                FieldValue::DateTime(45_292.0),
                FieldValue::Text("Berlin".to_owned()),
            ],
            vec![
                FieldValue::Currency(1_000_000),
                FieldValue::DateTime(45_662.0),
                FieldValue::Text("Hamburg".to_owned()),
            ],
        ];
        (definition, rows)
    }

    fn one_value(sql: &str) -> FieldValue {
        let result = run_sql(sql, &[source()]).expect("the statement runs");
        assert_eq!(result.rows.len(), 1, "{sql}");
        result.rows[0][0].clone()
    }

    /// Step 39.2 (item M36): Currency materializes unscaled — the scaled
    /// INTEGER storage leaked into every aggregate (`SUM(amount)` showed
    /// 8230000 instead of 823.0).
    #[test]
    fn currency_materializes_unscaled() {
        assert_eq!(one_value("SELECT SUM(amount) FROM Orders"), FieldValue::Float(723.0));
        assert_eq!(one_value("SELECT amount FROM Orders"), FieldValue::Float(723.0));
    }

    /// Step 39.2 (item M36): the SQLite ordering fence is pinned — a text
    /// literal never matches the numeric DateTime column (REAL sorts before
    /// TEXT), numeric literals do.
    #[test]
    fn text_literals_compare_as_text_per_sqlite_ordering() {
        assert_eq!(
            one_value("SELECT COUNT(*) FROM Orders WHERE stamp >= '2024-01-01'"),
            FieldValue::Integer(0),
            "text vs REAL is the documented SQLite fence, not silent Access luck"
        );
        assert_eq!(
            one_value("SELECT COUNT(*) FROM Orders WHERE stamp >= 45292"),
            FieldValue::Integer(2),
            "numeric comparisons work"
        );
    }

    /// Step 39.2 (item M36): LIKE speaks SQLite wildcards — `%` matches,
    /// the Access `*` does not (documented fence, never silent Access luck).
    #[test]
    fn like_uses_sqlite_wildcards() {
        assert_eq!(
            one_value("SELECT COUNT(*) FROM Orders WHERE city LIKE 'B*'"),
            FieldValue::Integer(0),
            "Access star is a literal under SQLite semantics"
        );
        assert_eq!(
            one_value("SELECT COUNT(*) FROM Orders WHERE city LIKE 'B%'"),
            FieldValue::Integer(1)
        );
    }

    /// Step 39.2 (item M36): integer overflow promotes to REAL exactly like
    /// SQLite — pinned so the promotion stays documented behavior.
    #[test]
    fn integer_overflow_promotes_to_real() {
        assert_eq!(
            one_value("SELECT 9223372036854775807 + 1"),
            FieldValue::Float(9.223_372_036_854_776e18),
            "SQLite promotes the overflowing sum to REAL"
        );
    }

    #[test]
    fn affinities_match_the_sqlite_backend_mapping() {
        assert_eq!(affinity(&FieldKind::Memo), "TEXT");
        assert_eq!(
            affinity(&FieldKind::Number(NumericKind::LongInteger)),
            "INTEGER"
        );
        assert_eq!(
            affinity(&FieldKind::Number(NumericKind::Decimal {
                precision: 10,
                scale: 2
            })),
            "REAL"
        );
        assert_eq!(affinity(&FieldKind::Guid), "BLOB");
        assert_eq!(
            affinity(&FieldKind::MultiValue(
                crate::schema::MultiValueKind::Text { max_len: 5 }
            )),
            "TEXT"
        );
        assert_eq!(affinity(&FieldKind::Complex), "TEXT");
    }

    /// Complex cells materialize as their joined display text (the shadow
    /// engine has no complex type).
    #[test]
    fn complex_values_materialize_as_display_text() {
        let value = FieldValue::Complex(vec![crate::schema::ComplexRecord {
            fields: vec![crate::schema::ComplexField {
                name: "Value".to_owned(),
                value: FieldValue::Text("a".to_owned()),
            }],
        }]);
        assert_eq!(
            sql_value(&FieldKind::Complex, &value),
            rusqlite::types::Value::Text("a".to_owned())
        );
    }

    #[test]
    fn semicolons_inside_string_literals_do_not_split_statements() {
        let result = run_sql("SELECT 'a;b' AS text", &[]).expect("literal semicolon");
        assert_eq!(result.rows[0][0], FieldValue::Text("a;b".into()));
    }

    #[test]
    fn materialization_rejects_ragged_rows() {
        let def = TableDef::new(
            "t",
            vec![crate::schema::FieldDef::new(
                "x",
                FieldKind::Number(NumericKind::LongInteger),
            )],
        );
        let err = run_sql("SELECT 1", &[(def, vec![vec![]])]).expect_err("ragged row");
        assert!(err.contains("expected 1"), "{err}");
    }
}
