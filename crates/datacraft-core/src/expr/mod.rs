//! Expression service (step 15.1, item H41).
//!
//! A small, total evaluator for Access-style expressions: field
//! references, literals, arithmetic and concatenation, comparison and
//! three-valued logic with Jet NULL semantics, the IIf/Nz/Format family,
//! date and string functions, and `Like` with Access wildcards.
//!
//! Documented subset and decisions:
//!
//! - Precedence (high → low): `^` (right-associative, binds tighter than
//!   unary minus), unary `-`, `* /`, `\`, `Mod`, `+ -`, `&`, comparisons
//!   (`= <> < <= > >=`, `Like`, `Between … And`, `In (…)`, `Is [Not]
//!   Null`), `Not`, `And`, `Or`.
//! - Arithmetic with any `Null` operand is `Null`; `/`, `\` and `Mod` by
//!   zero are runtime errors; `\` and `Mod` round their operands to
//!   integers first (VBA banker's rounding) and truncate toward zero.
//! - `&` treats `Null` as the empty string; text `+` text concatenates
//!   and propagates `Null`; text next to a number coerces like Access.
//! - Comparisons with `Null` are `Null`; text order is the engine's
//!   case-sensitive byte order (same family order as `query_criteria`);
//!   `Like` matches case-insensitively (Access `Option Compare Database`).
//! - Booleans coerce as VBA (`True = -1`); `IIf` takes the false branch on
//!   a `Null` condition (Jet behavior). `Now`/`Date` are absent on
//!   purpose — the evaluator is deterministic.
//! - `Decimal` participates in cross-family arithmetic as its raw scaled
//!   integer; date arithmetic widens to float (OLE Automation days).

mod coerce;
mod eval;
mod format;
mod funcs;
mod lexer;
mod like;
mod parser;

use crate::schema::FieldValue;

/// A parsed expression (AST of the documented subset).
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    /// A literal value (`Null`, boolean, number, text, `#date#`).
    Literal(FieldValue),
    /// A field reference: `[Table].[Field]`, `Table.Field` or `Field`.
    Field {
        /// Optional qualifying table name.
        table: Option<String>,
        /// Field name.
        name: String,
    },
    /// Unary negation or logical `Not`.
    Unary(UnaryOp, Box<Expression>),
    /// Arithmetic, concatenation or two-valued-independent logic.
    Binary(BinaryOp, Box<Expression>, Box<Expression>),
    /// Comparison or `Like`.
    Compare(CompareOp, Box<Expression>, Box<Expression>),
    /// `operand Between low And high`.
    Between(Box<Expression>, Box<Expression>, Box<Expression>),
    /// `operand In (a, b, …)`.
    InList(Box<Expression>, Vec<Expression>),
    /// `operand Is [Not] Null`.
    IsNull {
        /// The tested expression.
        operand: Box<Expression>,
        /// `true` for `Is Not Null`.
        negated: bool,
    },
    /// A function call; unknown names fail at evaluation time.
    Call {
        /// Function name (case-insensitive).
        name: String,
        /// Argument expressions.
        args: Vec<Expression>,
    },
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// Arithmetic negation.
    Negate,
    /// Logical negation (three-valued).
    Not,
}

/// Arithmetic and concatenation operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    /// `+` — addition or text concatenation with Null propagation.
    Add,
    /// `-`
    Subtract,
    /// `*`
    Multiply,
    /// `/` — always widens to float (currency pair stays currency).
    Divide,
    /// `\` — integer division, operands rounded first, truncating.
    IntegerDivide,
    /// `Mod` — remainder, operands rounded first, sign of the dividend.
    Modulo,
    /// `^` — float exponentiation, right-associative.
    Power,
    /// `&` — concatenation, `Null` renders as the empty string.
    Concat,
    /// `And` — three-valued.
    And,
    /// `Or` — three-valued.
    Or,
}

/// Comparison operators including `Like`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompareOp {
    /// `=`
    Equal,
    /// `<>`
    NotEqual,
    /// `<`
    Less,
    /// `<=`
    LessEqual,
    /// `>`
    Greater,
    /// `>=`
    GreaterEqual,
    /// `Like` with Access wildcards, case-insensitive.
    Like,
}

/// Why an expression failed; every variant carries a precise detail.
#[derive(Debug, Clone, PartialEq)]
pub enum ExprError {
    /// The source contains a malformed token.
    Lex {
        /// Byte offset of the offending character.
        position: usize,
        /// What went wrong.
        detail: String,
    },
    /// The token stream is not a valid expression.
    Parse {
        /// Byte offset where parsing stopped.
        position: usize,
        /// What went wrong.
        detail: String,
    },
    /// The expression is valid but evaluation failed (unknown field or
    /// function, type mismatch, division by zero, …).
    Runtime {
        /// What went wrong.
        detail: String,
    },
}

impl std::fmt::Display for ExprError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lex { position, detail } => {
                write!(f, "lexical error at byte {position}: {detail}")
            }
            Self::Parse { position, detail } => {
                write!(f, "parse error at byte {position}: {detail}")
            }
            Self::Runtime { detail } => write!(f, "{detail}"),
        }
    }
}

impl std::error::Error for ExprError {}

/// The evaluation environment: named field values (Access compares object
/// names case-insensitively, lookup follows insertion order — a bare name
/// resolves against any entry, a qualified name only against its table).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Scope {
    entries: Vec<(Option<String>, String, FieldValue)>,
}

impl Scope {
    /// Creates an empty scope.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds (or shadows) an unqualified field value.
    pub fn set(&mut self, name: impl Into<String>, value: FieldValue) -> &mut Self {
        self.entries.push((None, name.into(), value));
        self
    }

    /// Adds a table-qualified field value.
    pub fn set_field(
        &mut self,
        table: impl Into<String>,
        name: impl Into<String>,
        value: FieldValue,
    ) -> &mut Self {
        self.entries.push((Some(table.into()), name.into(), value));
        self
    }

    /// Resolves a field reference; `None` means "not found".
    pub(crate) fn resolve(&self, table: Option<&str>, name: &str) -> Option<&FieldValue> {
        let want = name.to_lowercase();
        self.entries
            .iter()
            .filter(|(entry_table, entry_name, _)| {
                entry_name.to_lowercase() == want
                    && match (table, entry_table.as_deref()) {
                        (Some(qualified), Some(entry)) => {
                            qualified.to_lowercase() == entry.to_lowercase()
                        }
                        (Some(_), None) => false,
                        (None, _) => true,
                    }
            })
            .map(|(_, _, value)| value)
            .next()
    }
}

/// Parses an expression source into its AST.
pub fn parse(source: &str) -> Result<Expression, ExprError> {
    parser::parse(source)
}

/// Family-safe ordering of two values — the report bands' `Min`/`Max`
/// reuse the evaluator's comparison rules (step 15.2).
pub fn field_ord(a: &FieldValue, b: &FieldValue) -> Result<std::cmp::Ordering, ExprError> {
    coerce::ord_cmp(a, b)
}

/// Float view of a value for aggregation (numeric text coerces; dates
/// count as their OLE days).
pub fn numeric_view(value: &FieldValue) -> Result<f64, ExprError> {
    Ok(f64::from(coerce::to_number(value)?))
}

impl Expression {
    /// Evaluates against a field scope; failures are precise runtime
    /// errors (unknown field or function, type mismatch, division by
    /// zero, …).
    pub fn evaluate(&self, scope: &Scope) -> Result<FieldValue, ExprError> {
        eval::evaluate(self, scope)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scope_resolves_qualified_before_bare_entries() {
        let mut scope = Scope::new();
        scope.set("Qty", FieldValue::Integer(1));
        scope.set_field("Order Details", "Qty", FieldValue::Integer(2));
        assert_eq!(
            scope.resolve(None, "qty"),
            Some(&FieldValue::Integer(1)),
            "bare lookup follows insertion order"
        );
        assert_eq!(
            scope.resolve(Some("order details"), "QTY"),
            Some(&FieldValue::Integer(2))
        );
        assert_eq!(scope.resolve(Some("Other"), "Qty"), None);
        assert_eq!(scope.resolve(None, "Ghost"), None);
    }

    #[test]
    fn errors_render_their_positions() {
        let lex = ExprError::Lex {
            position: 3,
            detail: "boom".into(),
        };
        let parse = ExprError::Parse {
            position: 7,
            detail: "bang".into(),
        };
        assert_eq!(lex.to_string(), "lexical error at byte 3: boom");
        assert_eq!(parse.to_string(), "parse error at byte 7: bang");
    }
}
