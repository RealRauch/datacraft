//! The structured filter builder (step 87.3, item M104): conditions and
//! AND/OR groups with visible parentheses, ordered sort columns, and
//! the serialization onto the existing criteria map. The execution path
//! and the wildcard rule stay unchanged — serialization writes the same
//! per-column criteria strings the datasheet's view order already
//! applies, so every condition round-trips through the real criteria
//! parser (step 7.3). OR groups are outside that dialect: the
//! serialization refuses precisely (the columns AND together), and an
//! invalid draft never runs.
//!
//! Sort columns stay ordered in the model; the existing single-column
//! sort path applies the first entry (documented fence — a multi-column
//! sort would change the execution path).

use std::collections::BTreeMap;
use std::fmt;

use datacraft_types::parse_date;

use crate::qbe::SortOrder;

/// One comparison operator of a builder condition (step 87.3, M104) —
/// the criteria dialect's supported subset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuilderOp {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    /// The Access wildcard pattern; the criteria form is `Like "pattern"`.
    Like,
}

impl BuilderOp {
    /// The operator's criteria prefix (Like takes the quoted pattern —
    /// [`condition_criteria`] adds it).
    pub fn criteria_prefix(self) -> &'static str {
        match self {
            Self::Equal => "=",
            Self::NotEqual => "<>",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::Like => "Like",
        }
    }

    /// The operator's display sign (the parenthesized form).
    fn display_sign(self) -> &'static str {
        match self {
            Self::Equal => "=",
            Self::NotEqual => "<>",
            Self::Less => "<",
            Self::LessEqual => "<=",
            Self::Greater => ">",
            Self::GreaterEqual => ">=",
            Self::Like => "Like",
        }
    }

    /// The operator behind a criteria prefix; `None` for the bare form
    /// (implicit equality) and the Like form.
    fn from_prefix(prefix: &str) -> Option<Self> {
        match prefix {
            "<>" => Some(Self::NotEqual),
            "<=" => Some(Self::LessEqual),
            ">=" => Some(Self::GreaterEqual),
            "=" | "" => Some(Self::Equal),
            "<" => Some(Self::Less),
            ">" => Some(Self::Greater),
            _ => None,
        }
    }
}

/// One condition: a column, a comparison and the raw literal text (the
/// text mode's own form — coercion stays with the parser on run).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuilderCondition {
    /// The column the condition applies to.
    pub column: String,
    /// The comparison operator.
    pub op: BuilderOp,
    /// The literal as written.
    pub literal: String,
}

/// How a group's children combine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Connective {
    And,
    Or,
}

/// One node of the filter tree: a condition or a group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterNode {
    Condition(BuilderCondition),
    Group {
        connective: Connective,
        children: Vec<FilterNode>,
    },
}

impl fmt::Display for FilterNode {
    /// The parenthesized form (step 87.3, M104 — the grouping is
    /// visible): groups wrap in parentheses, literals that read as
    /// numbers, dates or yes/no render bare, everything else quoted.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Condition(condition) => {
                if condition.op == BuilderOp::Like {
                    return write!(f, "{} Like \"{}\"", condition.column, condition.literal);
                }
                let literal = bare_or_quoted(&condition.literal);
                write!(
                    f,
                    "{} {} {}",
                    condition.column,
                    condition.op.display_sign(),
                    literal
                )
            }
            Self::Group {
                connective,
                children,
            } => {
                let keyword = match connective {
                    Connective::And => " AND ",
                    Connective::Or => " OR ",
                };
                let inner: Vec<String> = children.iter().map(|child| child.to_string()).collect();
                write!(f, "({})", inner.join(keyword))
            }
        }
    }
}

/// The join keyword without the trailing spaces (used inside the
/// parenthesized form's tail).
fn bare_or_quoted(literal: &str) -> String {
    let bare = literal.parse::<f64>().is_ok()
        || parse_date(literal).is_some()
        || matches!(
            literal.to_lowercase().as_str(),
            "yes" | "no" | "true" | "false"
        );
    if bare {
        literal.to_owned()
    } else {
        format!("\"{literal}\"")
    }
}

/// Why a filter tree cannot serialize onto the criteria map (step 87.3,
/// M104 — the dialect fence, typed for the agent surfaces).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterBuilderError {
    /// An OR group — the criteria columns AND together; the draft must
    /// not run.
    OrNotExpressible,
    /// Two conditions of one AND group address the same column: the
    /// input carries one criteria per column.
    DuplicateColumn { column: String },
    /// A criteria form the builder cannot model (the dialect's
    /// recognized Access forms) — the text mode edits it.
    OutsideBuilderModel { field: String, criteria: String },
}

impl fmt::Display for FilterBuilderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OrNotExpressible => write!(
                f,
                "an OR group is outside the criteria dialect — the columns \
                 AND together; apply its branches in text mode instead"
            ),
            Self::DuplicateColumn { column } => write!(
                f,
                "the column {column} carries more than one condition in one \
                 AND group — the criteria input holds one per column"
            ),
            Self::OutsideBuilderModel { field, criteria } => write!(
                f,
                "the criteria of {field} ({criteria}) is outside the \
                 builder's model — edit it in text mode"
            ),
        }
    }
}

impl std::error::Error for FilterBuilderError {}

/// One ordered sort column of the model (step 87.3, M104).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuilderSort {
    /// The column to sort by.
    pub column: String,
    /// The direction.
    pub order: SortOrder,
}

/// The condition's criteria string: the operator prefix and the bare
/// literal (Like keeps the quoted pattern — the parser's wildcard form,
/// step 70.1/H58 unchanged).
fn condition_criteria(condition: &BuilderCondition) -> String {
    if condition.op == BuilderOp::Like {
        return format!("Like \"{}\"", condition.literal);
    }
    format!(
        "{} {}",
        condition.op.criteria_prefix(),
        condition.literal.trim()
    )
}

/// Serializes the filter tree onto the existing criteria map (step
/// 87.3, item M104): AND conditions land one per column — the map the
/// datasheet's view order applies. OR groups and duplicate columns
/// refuse precisely: an invalid draft never runs.
pub fn serialize_filters(
    root: &FilterNode,
) -> Result<BTreeMap<String, String>, FilterBuilderError> {
    let mut filters = BTreeMap::new();
    let mut collect = |condition: &BuilderCondition| -> Result<(), FilterBuilderError> {
        let column = condition.column.clone();
        if filters.contains_key(&column) {
            return Err(FilterBuilderError::DuplicateColumn { column });
        }
        filters.insert(column, condition_criteria(condition));
        Ok(())
    };
    for_and_conditions(root, &mut collect)?;
    Ok(filters)
}

/// Walks an AND-shaped tree, feeding every condition to `collect`.
/// An OR group anywhere refuses (the dialect fence); the closure's
/// error bubbles (duplicate columns).
fn for_and_conditions(
    node: &FilterNode,
    collect: &mut dyn FnMut(&BuilderCondition) -> Result<(), FilterBuilderError>,
) -> Result<(), FilterBuilderError> {
    match node {
        FilterNode::Condition(condition) => collect(condition),
        FilterNode::Group {
            connective,
            children,
        } => {
            if *connective == Connective::Or {
                return Err(FilterBuilderError::OrNotExpressible);
            }
            for child in children {
                for_and_conditions(child, collect)?;
            }
            Ok(())
        }
    }
}

/// How a criteria map reads back: a modeled AND group, or the precise
/// reason a column's criteria is outside the builder's model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterRead {
    /// The modeled tree: one AND group over the map's columns.
    Model(FilterNode),
    /// The column's criteria form is not modelable — the text mode
    /// edits it.
    Failed(FilterBuilderError),
}

/// Reads the criteria map back into the builder (step 87.3, M104):
/// one AND group, one condition per column; operator forms and Like
/// preserved, a bare literal normalizes to the explicit `=` form.
pub fn deserialize_filters(filters: &BTreeMap<String, String>) -> FilterRead {
    let mut children = Vec::new();
    for (field, criteria) in filters {
        let Some(condition) = condition_from_criteria(field, criteria) else {
            return FilterRead::Failed(FilterBuilderError::OutsideBuilderModel {
                field: field.clone(),
                criteria: criteria.clone(),
            });
        };
        children.push(FilterNode::Condition(condition));
    }
    FilterRead::Model(and_group(children))
}

fn and_group(children: Vec<FilterNode>) -> FilterNode {
    FilterNode::Group {
        connective: Connective::And,
        children,
    }
}

/// One criteria string as a condition: `Like "pattern"`, an operator
/// prefix, or the bare literal (implicit equality). `None` — the
/// dialect's recognized but unmodelable forms.
fn condition_from_criteria(field: &str, criteria: &str) -> Option<BuilderCondition> {
    let trimmed = criteria.trim();
    let lowercase = trimmed.to_lowercase();
    if lowercase.starts_with("like ") {
        let pattern = trimmed[5..].trim();
        let literal = pattern
            .strip_prefix('"')
            .and_then(|rest| rest.strip_suffix('"'))?;
        return Some(BuilderCondition {
            column: field.to_owned(),
            op: BuilderOp::Like,
            literal: literal.to_owned(),
        });
    }
    const UNSUPPORTED: [&str; 6] = [
        "between ",
        "in (",
        "in(",
        "is null",
        "is not null",
        "not like ",
    ];
    if UNSUPPORTED.iter().any(|form| lowercase.starts_with(form)) {
        return None;
    }
    let (prefix, rest) = if let Some(rest) = trimmed.strip_prefix("<>") {
        ("<>", rest)
    } else if let Some(rest) = trimmed.strip_prefix("<=") {
        ("<=", rest)
    } else if let Some(rest) = trimmed.strip_prefix(">=") {
        (">=", rest)
    } else if let Some(rest) = trimmed.strip_prefix('=') {
        ("=", rest)
    } else if let Some(rest) = trimmed.strip_prefix('<') {
        ("<", rest)
    } else if let Some(rest) = trimmed.strip_prefix('>') {
        (">", rest)
    } else {
        ("", trimmed)
    };
    Some(BuilderCondition {
        column: field.to_owned(),
        op: BuilderOp::from_prefix(prefix)?,
        literal: rest.trim().to_owned(),
    })
}

/// The existing single-column sort path applies the first sort of the
/// model (step 87.3, M104 — the documented fence; a multi-column sort
/// would change the execution path).
pub fn serialize_sorts(sorts: &[BuilderSort]) -> Option<(String, SortOrder)> {
    sorts.first().map(|sort| (sort.column.clone(), sort.order))
}
