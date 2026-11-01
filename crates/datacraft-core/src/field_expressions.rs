//! Field expression wiring (step 15.2, item H41).
//!
//! One source of truth for how the expression service connects to the
//! data paths — the row commands ([`crate::command::Command::InsertRow`]
//! / [`crate::command::Command::UpdateRow`]) and the journaled write
//! profile alike go through [`prepare_insert`]/[`prepare_update`]:
//!
//! 1. **Defaults** fill `Null` cells on insert (never on update) with an
//!    empty scope — Access defaults see no row fields.
//! 2. **Calculated** fields are engine-computed from the (growing) row;
//!    user-provided values are refused like AutoNumber values, and the
//!    result validates against the field kind.
//! 3. **Validation rules** run over the final row: only `True` accepts;
//!    `False` and `Null` violate with the validation text (or the rule).
//!
//! AutoNumber `Null` cells stay engine-assigned; rules see them as `Null`
//! at rule time.

use crate::command::ExpressionContext;
use crate::core::failed;
use crate::event::Event;
use crate::expr::{self, Scope};
use crate::schema::{FieldDef, FieldKind, FieldValue, TableDef};

/// Prepares a new row: defaults, calculated values, rules, schema.
pub(crate) fn prepare_insert(
    table: &TableDef,
    values: &[FieldValue],
) -> Result<Vec<FieldValue>, String> {
    if values.len() != table.fields.len() {
        return Err(format!(
            "insert into {:?}: expected {} values, found {}",
            table.name,
            table.fields.len(),
            values.len()
        ));
    }
    let mut out = values.to_vec();
    for (index, field) in table.fields.iter().enumerate() {
        if matches!(field.kind, FieldKind::AutoNumber(_)) && matches!(out[index], FieldValue::Null)
        {
            continue;
        }
        if let Some(default) = &field.properties.default_value
            && matches!(out[index], FieldValue::Null)
        {
            out[index] = eval_standalone(default, field, "default")?;
        }
        refuse_calculated(field, &out[index])?;
        field
            .validate_value(&out[index])
            .map_err(|e| format!("field {:?}: {e}", field.name))?;
    }
    compute_calculated(table, &mut out)?;
    enforce_rules(table, &out)?;
    Ok(out)
}

/// Prepares a replacement row: no defaults (Access applies them only on
/// new records), calculated recompute, rules and schema re-validate.
pub(crate) fn prepare_update(
    table: &TableDef,
    current: &[FieldValue],
    values: &[FieldValue],
) -> Result<Vec<FieldValue>, String> {
    if values.len() != table.fields.len() {
        return Err(format!(
            "update {:?}: expected {} values, found {}",
            table.name,
            table.fields.len(),
            values.len()
        ));
    }
    let mut out = values.to_vec();
    for (index, field) in table.fields.iter().enumerate() {
        if matches!(field.kind, FieldKind::AutoNumber(_)) {
            if matches!(out[index], FieldValue::Null) {
                out[index] = current[index].clone();
            } else if out[index] != current[index] {
                return Err(format!(
                    "field {:?}: AutoNumber values are engine-assigned and cannot be changed",
                    field.name
                ));
            }
            continue;
        }
        refuse_calculated(field, &out[index])?;
        field
            .validate_value(&out[index])
            .map_err(|e| format!("field {:?}: {e}", field.name))?;
    }
    compute_calculated(table, &mut out)?;
    enforce_rules(table, &out)?;
    Ok(out)
}

fn refuse_calculated(field: &FieldDef, value: &FieldValue) -> Result<(), String> {
    if field.properties.calculated.is_some() && !matches!(value, FieldValue::Null) {
        return Err(format!(
            "field {:?}: calculated values are engine-computed — leave the cell empty",
            field.name
        ));
    }
    Ok(())
}

/// Computes every calculated field in field order; already-computed
/// neighbors are visible (single pass, no self-reference).
fn compute_calculated(table: &TableDef, out: &mut [FieldValue]) -> Result<(), String> {
    for index in 0..table.fields.len() {
        let field = &table.fields[index];
        let Some(expression) = &field.properties.calculated else {
            continue;
        };
        let value = row_scope(table, out).evaluate_prepared(expression, field)?;
        out[index] = value;
        field
            .validate_value(&out[index])
            .map_err(|e| format!("field {:?}: {e}", field.name))?;
    }
    Ok(())
}

/// Only `True` accepts the row; `False` and `Null` violate.
fn enforce_rules(table: &TableDef, out: &[FieldValue]) -> Result<(), String> {
    let scope = row_scope(table, out);
    for field in &table.fields {
        let Some(rule) = &field.properties.validation_rule else {
            continue;
        };
        let accepted = scope.evaluate_prepared(rule, field)?;
        if !matches!(accepted, FieldValue::Boolean(true)) {
            let message = field
                .properties
                .validation_text
                .clone()
                .unwrap_or_else(|| format!("validation rule failed: {rule}"));
            return Err(format!("field {:?}: {message}", field.name));
        }
    }
    Ok(())
}

/// Evaluates a standalone expression (default values see no row fields)
/// against a field for precise diagnostics.
fn eval_standalone(source: &str, field: &FieldDef, role: &str) -> Result<FieldValue, String> {
    expr::parse(source)
        .and_then(|parsed| parsed.evaluate(&Scope::new()))
        .map_err(|e| format!("field {:?}: {role} {source:?}: {e}", field.name))
}

/// Builds the expression scope of one row (qualified entries; bare
/// lookups resolve through the qualified ones).
pub(crate) fn row_scope(table: &TableDef, values: &[FieldValue]) -> Scope {
    let mut scope = Scope::new();
    for (field, value) in table.fields.iter().zip(values) {
        scope.set_field(table.name.clone(), field.name.clone(), value.clone());
    }
    scope
}

/// Private evaluation helper on the scope: parses and evaluates with the
/// field name in the error.
trait ScopeEvaluate {
    fn evaluate_prepared(&self, source: &str, field: &FieldDef) -> Result<FieldValue, String>;
}

impl ScopeEvaluate for Scope {
    fn evaluate_prepared(&self, source: &str, field: &FieldDef) -> Result<FieldValue, String> {
        expr::parse(source)
            .and_then(|parsed| parsed.evaluate(self))
            .map_err(|e| format!("field {:?}: expression {source:?}: {e}", field.name))
    }
}

impl crate::core::Core {
    /// Evaluates one expression (the read side of `Command::RunExpression`):
    /// with a context the expression sees the row's fields, without one it
    /// evaluates over an empty scope.
    pub fn run_expression(
        &mut self,
        context: Option<ExpressionContext>,
        expression: &str,
    ) -> Result<FieldValue, String> {
        let value = match context {
            None => expr::parse(expression)
                .and_then(|parsed| parsed.evaluate(&Scope::new()))
                .map_err(|e| e.to_string())?,
            Some(ExpressionContext { table, row }) => {
                let Some(def) = self.table_definition(&table) else {
                    return Err(crate::port::PortError::TableNotFound(table).to_string());
                };
                let def = def.clone();
                let current = self
                    .port
                    .read_rows(&def.name, crate::port::RowRange { start: row, len: 1 })
                    .map_err(|e| e.to_string())?;
                let Some(row_values) = current.into_iter().next() else {
                    return Err(format!("row {row} of {:?} is out of range", def.name));
                };
                expr::parse(expression)
                    .and_then(|parsed| parsed.evaluate(&row_scope(&def, &row_values)))
                    .map_err(|e| e.to_string())?
            }
        };
        self.state.expression_result = Some(value.clone());
        Ok(value)
    }

    pub(crate) fn run_expression_command(
        &mut self,
        context: Option<ExpressionContext>,
        expression: &str,
    ) -> Vec<Event> {
        match self.run_expression(context, expression) {
            Ok(value) => vec![Event::ExpressionEvaluated {
                result: crate::present::display_value(&value),
            }],
            Err(reason) => {
                self.state.expression_result = None;
                failed(reason)
            }
        }
    }

    /// The typed result of the last successful expression run (the agent
    /// seam; the event carries the display form).
    pub fn expression_result(&self) -> Option<&FieldValue> {
        self.state.expression_result.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::NumericKind;

    fn table() -> TableDef {
        TableDef::new(
            "T",
            vec![
                FieldDef::new("qty", FieldKind::Number(NumericKind::LongInteger)),
                FieldDef::new("price", FieldKind::Currency),
            ],
        )
    }

    #[test]
    fn row_scope_resolves_bare_and_qualified_names() {
        let scope = row_scope(
            &table(),
            &[FieldValue::Integer(2), FieldValue::Currency(300)],
        );
        let parsed = expr::parse("[qty] + [T].[price]").expect("parses");
        assert!(parsed.evaluate(&scope).is_ok());
    }

    #[test]
    fn prepare_insert_enforces_arity() {
        let reason =
            prepare_insert(&table(), &[FieldValue::Integer(1)]).expect_err("arity must fail");
        assert!(reason.contains("expected 2 values"), "{reason}");
    }

    #[test]
    fn rules_see_engine_assigned_autonumbers_as_null() {
        let mut table = table();
        table.fields.insert(
            0,
            FieldDef::new(
                "id",
                FieldKind::AutoNumber(crate::schema::AutoKind::LongInteger),
            ),
        );
        table.fields[1].properties.validation_rule = Some("[id] Is Null".into());
        let out = prepare_insert(
            &table,
            &[
                FieldValue::Null,
                FieldValue::Integer(1),
                FieldValue::Currency(1),
            ],
        )
        .expect("engine-assigned slots stay Null for the rule");
        assert_eq!(out[0], FieldValue::Null);
    }
}
