//! Static VBA analysis (step 17.3, item L5).
//!
//! Headless and deterministic: a coarse call graph (qualified
//! `Module.Procedure` calls, `New Module` instantiation, unambiguous bare
//! procedure calls), SQL string literals, table/query references and
//! unused modules. The scan is deliberately conservative — no parser and
//! no execution (ADR-2).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::vba::{VbaModule, VbaModuleKind, VbaProject};

/// Result of the static analysis over one project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VbaAnalysis {
    /// One entry per module, in project order.
    pub call_graph: Vec<VbaModuleCalls>,
    /// SQL literals in module/source order.
    pub sql_literals: Vec<VbaSqlLiteral>,
    /// Referenced tables (canonical schema names, sorted).
    pub table_references: Vec<String>,
    /// Referenced queries (canonical names, sorted).
    pub query_references: Vec<String>,
    /// Standard/class modules without an incoming edge, in project order.
    pub unused_modules: Vec<String>,
}

/// Call targets of one module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VbaModuleCalls {
    /// Calling module.
    pub module: String,
    /// Called modules (deduplicated, sorted case-insensitively).
    pub callees: Vec<String>,
}

/// One SQL string literal embedded in a module.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VbaSqlLiteral {
    /// Module that contains the literal.
    pub module: String,
    /// The literal text (without the surrounding quotes).
    pub sql: String,
}

/// Analyzes a project against the open database's table and query names.
pub fn analyze(project: &VbaProject, tables: &[String], queries: &[String]) -> VbaAnalysis {
    let modules: Vec<(String, &VbaModule)> = project
        .modules
        .iter()
        .map(|module| (module.name.to_lowercase(), module))
        .collect();
    let procedures = procedure_map(&project.modules);
    let mut table_refs = BTreeMap::new();
    let mut query_refs = BTreeMap::new();
    let mut call_graph = Vec::with_capacity(project.modules.len());
    let mut sql_literals = Vec::new();

    for module in &project.modules {
        let masked = mask_definitions(&module.source);
        let mut callees: Vec<String> = Vec::new();
        for target in calls(&masked, &modules, &procedures) {
            if !callees
                .iter()
                .any(|existing| existing.eq_ignore_ascii_case(&target))
            {
                callees.push(target);
            }
        }
        callees.sort_by_key(|name| name.to_lowercase());
        call_graph.push(VbaModuleCalls {
            module: module.name.clone(),
            callees,
        });

        let literals = string_literals(&module.source);
        for literal in &literals {
            if looks_like_sql(literal) {
                sql_literals.push(VbaSqlLiteral {
                    module: module.name.clone(),
                    sql: literal.clone(),
                });
            }
            collect_references(literal, tables, &mut table_refs);
            collect_references(literal, queries, &mut query_refs);
        }
        for name in bracket_names(&module.source) {
            collect_references(&name, tables, &mut table_refs);
            collect_references(&name, queries, &mut query_refs);
        }
    }

    let incoming: Vec<&str> = call_graph
        .iter()
        .flat_map(|entry| {
            entry
                .callees
                .iter()
                .filter(|callee| !callee.eq_ignore_ascii_case(&entry.module))
                .map(String::as_str)
        })
        .collect();
    let unused_modules = project
        .modules
        .iter()
        .filter(|module| {
            matches!(module.kind, VbaModuleKind::Standard | VbaModuleKind::Class)
                && !incoming
                    .iter()
                    .any(|callee| callee.eq_ignore_ascii_case(&module.name))
        })
        .map(|module| module.name.clone())
        .collect();

    VbaAnalysis {
        call_graph,
        sql_literals,
        table_references: table_refs.into_values().collect(),
        query_references: query_refs.into_values().collect(),
        unused_modules,
    }
}

/// Lowercase procedure name → defining module (first definition wins).
fn procedure_map(modules: &[VbaModule]) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for module in modules {
        for line in module.source.lines() {
            if let Some(name) = procedure_name(line) {
                map.entry(name.to_lowercase())
                    .or_insert_with(|| module.name.clone());
            }
        }
    }
    map
}

/// Procedure name of a definition line (`Sub`, `Function`, `Property`).
fn procedure_name(line: &str) -> Option<String> {
    let mut rest = line.trim();
    loop {
        let lower = rest.to_ascii_lowercase();
        let Some(prefix) = ["public ", "private ", "friend ", "static "]
            .iter()
            .find(|prefix| lower.starts_with(**prefix))
        else {
            break;
        };
        rest = &rest[prefix.len()..];
    }
    let lower = rest.to_ascii_lowercase();
    let after = [
        "sub ",
        "function ",
        "property get ",
        "property let ",
        "property set ",
    ]
    .iter()
    .find_map(|prefix| lower.starts_with(prefix).then(|| &rest[prefix.len()..]))?;
    let name: String = after
        .chars()
        .take_while(|c| c.is_alphanumeric() || *c == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// Blanks definition lines so the definition name is not read as a call.
fn mask_definitions(source: &str) -> String {
    source
        .lines()
        .map(|line| {
            let trimmed = line.trim_start();
            if procedure_name(line).is_some() || trimmed.to_ascii_lowercase().starts_with("rem ") {
                ""
            } else {
                line
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Word(String),
    Dot,
    LParen,
}

/// Tokens outside comments and string literals.
fn tokens(source: &str) -> Vec<Token> {
    let chars: Vec<char> = source.chars().collect();
    let mut out = Vec::new();
    let mut index = 0usize;
    while index < chars.len() {
        let c = chars[index];
        match c {
            '\'' => {
                while index < chars.len() && chars[index] != '\n' {
                    index += 1;
                }
            }
            '"' => {
                index += 1;
                while index < chars.len() {
                    if chars[index] == '"' {
                        if index + 1 < chars.len() && chars[index + 1] == '"' {
                            index += 2;
                        } else {
                            index += 1;
                            break;
                        }
                    } else {
                        index += 1;
                    }
                }
            }
            '.' => {
                out.push(Token::Dot);
                index += 1;
            }
            '(' => {
                out.push(Token::LParen);
                index += 1;
            }
            c if c.is_alphanumeric() || c == '_' => {
                let start = index;
                while index < chars.len() && (chars[index].is_alphanumeric() || chars[index] == '_')
                {
                    index += 1;
                }
                out.push(Token::Word(chars[start..index].iter().collect()));
            }
            _ => index += 1,
        }
    }
    out
}

/// Modules called by one module body.
fn calls(
    source: &str,
    modules: &[(String, &VbaModule)],
    procedures: &BTreeMap<String, String>,
) -> Vec<String> {
    let tokens = tokens(source);
    let mut out = Vec::new();
    for (position, token) in tokens.iter().enumerate() {
        let Token::Word(word) = token else {
            continue;
        };
        let next = tokens.get(position + 1);
        if word.eq_ignore_ascii_case("new") {
            if let Some(Token::Word(target)) = next
                && let Some(module) = module_name(modules, target)
            {
                out.push(module.to_owned());
            }
            continue;
        }
        if matches!(next, Some(Token::Dot)) {
            if let Some(Token::Word(_member)) = tokens.get(position + 2)
                && let Some(module) = module_name(modules, word)
            {
                out.push(module.to_owned());
            }
            continue;
        }
        if matches!(next, Some(Token::LParen))
            && let Some(module) = procedures.get(&word.to_lowercase())
        {
            out.push(module.clone());
        }
    }
    out
}

fn module_name<'a>(modules: &'a [(String, &VbaModule)], name: &str) -> Option<&'a str> {
    modules
        .iter()
        .find(|(lower, _)| lower == &name.to_lowercase())
        .map(|(_, module)| module.name.as_str())
}

/// Double-quoted string literals in source order (`""` escapes a quote).
fn string_literals(source: &str) -> Vec<String> {
    let chars: Vec<char> = source.chars().collect();
    let mut out = Vec::new();
    let mut index = 0usize;
    while index < chars.len() {
        if chars[index] == '\'' {
            while index < chars.len() && chars[index] != '\n' {
                index += 1;
            }
            continue;
        }
        if chars[index] != '"' {
            index += 1;
            continue;
        }
        index += 1;
        let mut literal = String::new();
        while index < chars.len() {
            if chars[index] == '"' {
                if index + 1 < chars.len() && chars[index + 1] == '"' {
                    literal.push('"');
                    index += 2;
                } else {
                    index += 1;
                    break;
                }
            } else {
                literal.push(chars[index]);
                index += 1;
            }
        }
        out.push(literal);
    }
    out
}

/// Whether a string literal starts like a SQL statement.
fn looks_like_sql(literal: &str) -> bool {
    let lower = literal.trim_start().to_ascii_lowercase();
    if ["select ", "select*", "transform ", "parameters "]
        .iter()
        .any(|prefix| lower.starts_with(prefix))
    {
        return true;
    }
    let statement = ["insert ", "update ", "delete "]
        .iter()
        .any(|prefix| lower.starts_with(prefix));
    statement
        && [" into ", " from ", " set ", " where ", " values "]
            .iter()
            .any(|clause| lower.contains(clause))
}

/// Adds every table/query name referenced by `text`.
fn collect_references(text: &str, names: &[String], out: &mut BTreeMap<String, String>) {
    let lower = text.to_lowercase();
    for name in names {
        let lower_name = name.to_lowercase();
        let bracketed = format!("[{lower_name}]");
        let matched = lower.trim() == lower_name
            || lower.contains(&bracketed)
            || (!lower_name.contains(' ')
                && lower
                    .split(|c: char| !(c.is_alphanumeric() || c == '_'))
                    .any(|word| word == lower_name));
        if matched {
            out.insert(lower_name, name.clone());
        }
    }
}

/// Bracket identifiers outside string literals and comments.
fn bracket_names(source: &str) -> Vec<String> {
    let chars: Vec<char> = source.chars().collect();
    let mut out = Vec::new();
    let mut index = 0usize;
    while index < chars.len() {
        match chars[index] {
            '\'' => {
                while index < chars.len() && chars[index] != '\n' {
                    index += 1;
                }
            }
            '"' => {
                index += 1;
                while index < chars.len() {
                    if chars[index] == '"' {
                        if index + 1 < chars.len() && chars[index + 1] == '"' {
                            index += 2;
                        } else {
                            index += 1;
                            break;
                        }
                    } else {
                        index += 1;
                    }
                }
            }
            '[' => {
                let start = index + 1;
                index = start;
                while index < chars.len() && chars[index] != ']' {
                    index += 1;
                }
                if index < chars.len() {
                    out.push(chars[start..index].iter().collect());
                    index += 1;
                }
            }
            _ => index += 1,
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn procedure_names_handle_modifiers_and_property_accessors() {
        assert_eq!(procedure_name("Public Sub Main()"), Some("Main".to_owned()));
        assert_eq!(
            procedure_name("Private Static Function Helper() As Long"),
            Some("Helper".to_owned())
        );
        assert_eq!(
            procedure_name("Friend Property Get Value() As String"),
            Some("Value".to_owned())
        );
        assert_eq!(procedure_name("Dim x As Long"), None);
    }

    #[test]
    fn string_literals_and_sql_detection_are_conservative() {
        let source = "a = \"one\"\"two\"\nb = \" SELECT * FROM [T] \"\nc = 'x' \"not a string?\"";
        let literals = string_literals(source);
        assert_eq!(literals, vec!["one\"two", " SELECT * FROM [T] "]);
        assert!(looks_like_sql(" SELECT * FROM [T] "));
        assert!(looks_like_sql("select 1"));
        assert!(!looks_like_sql("hello"));
        assert!(!looks_like_sql("delete everything by hand"));
    }
}
