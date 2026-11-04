//! Catalog parity tests (step 18.3, item I3; ADR-4).
//!
//! Mechanical guards for the German catalog: unique non-empty entries, an
//! unchanged placeholder set per template, and coverage in both directions —
//! every key the UI can render is translated, and the catalog holds no entry
//! the UI never asks for.

use std::collections::BTreeSet;

use datacraft_core::palette::TOOLBOX;
use datacraft_core::ribbon::ribbon_spec;

use super::de::entries;

/// Every catalog key is unique, non-empty and actually translated.
#[test]
fn catalog_keys_are_unique_and_translations_non_empty() {
    let mut seen = BTreeSet::new();
    for &(english, german) in entries() {
        assert!(
            !german.trim().is_empty(),
            "empty translation for {english:?}"
        );
        assert!(seen.insert(english), "duplicate entry for {english:?}");
    }
}

/// Templates keep the English placeholder set exactly — a drifted `{name}`
/// would render a literal brace or drop a value silently.
#[test]
fn translations_keep_the_english_placeholder_set() {
    for &(english, german) in entries() {
        assert_eq!(
            placeholders(english),
            placeholders(german),
            "placeholder drift for {english:?}"
        );
    }
}

/// Both directions: every key the UI can render is translated, and the
/// catalog holds no entry the UI never asks for.
#[test]
fn every_ui_key_is_translated_and_no_entry_is_orphaned() {
    let mut used = scanned_keys();
    used.extend(dynamic_keys());
    let catalog: BTreeSet<String> = entries()
        .map(|(english, _)| (*english).to_owned())
        .collect();

    let missing: Vec<&String> = used.difference(&catalog).collect();
    assert!(missing.is_empty(), "untranslated UI keys: {missing:?}");
    let orphaned: Vec<&String> = catalog.difference(&used).collect();
    assert!(
        orphaned.is_empty(),
        "catalog entries used nowhere: {orphaned:?}"
    );
}

/// Keys the paint code passes through variables rather than literals:
/// ribbon and toolbox specs plus the constants shared with the tests.
fn dynamic_keys() -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    let tabs = ribbon_spec()
        .tabs
        .iter()
        .chain(ribbon_spec().contextual_tabs.iter().map(|entry| &entry.tab));
    for tab in tabs {
        keys.insert(tab.label.to_owned());
        for group in &tab.groups {
            keys.insert(group.label.to_owned());
            for action in &group.actions {
                keys.insert(action.label.to_owned());
            }
        }
    }
    for entry in &ribbon_spec().quick_access {
        keys.insert(entry.label.to_owned());
    }
    for page in crate::backstage::BackstagePage::ALL {
        keys.insert(page.label().to_owned());
    }
    for entry in TOOLBOX {
        keys.insert(entry.label.to_owned());
    }
    for key in [
        crate::nav_pane::SECTION_TABLES,
        crate::nav_pane::SECTION_QUERIES,
        crate::nav_pane::SECTION_FORMS,
        crate::nav_pane::SECTION_REPORTS,
        crate::nav_pane::SECTION_MACROS,
        crate::nav_pane::SECTION_MODULES,
        crate::shell::STATUS_TABLE,
        crate::shell::STATUS_TABLES,
        crate::relationships::LABEL_REFERENCING_TABLE,
        crate::relationships::LABEL_REFERENCING_FIELD,
        crate::relationships::LABEL_REFERENCED_TABLE,
        crate::relationships::LABEL_REFERENCED_FIELD,
    ] {
        keys.insert(key.to_owned());
    }
    keys.insert(crate::password_prompt::WRONG_PASSWORD.to_owned());
    keys
}

/// All `t(...)`, `tr(...)` and `plural(...)` key literals of the UI sources
/// (the kernel and the catalog itself are not UI call sites).
fn scanned_keys() -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for source in ui_sources() {
        collect_call_keys(&source, &mut keys);
    }
    keys
}

fn ui_sources() -> Vec<String> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut sources = Vec::new();
    collect_sources(&root, &mut sources);
    sources
}

/// Recursively collects `.rs` files (submodules live in directories);
/// the kernel and the catalog itself are not UI call sites.
fn collect_sources(directory: &std::path::Path, sources: &mut Vec<String>) {
    for entry in std::fs::read_dir(directory).expect("src directory") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            if path.file_name().and_then(|name| name.to_str()) == Some("i18n") {
                continue;
            }
            collect_sources(&path, sources);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        if path.file_name().and_then(|name| name.to_str()) == Some("i18n.rs") {
            continue;
        }
        sources.push(std::fs::read_to_string(&path).expect("read ui source"));
    }
}

fn collect_call_keys(source: &str, keys: &mut BTreeSet<String>) {
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let Some((name, args)) = call_at(&chars, i) else {
            i += 1;
            continue;
        };
        match name {
            "t" | "tr" => {
                if let Some((literal, _)) = string_at(&chars, skip_ws(&chars, args)) {
                    keys.insert(literal);
                }
            }
            _ => collect_plural_keys(&chars, args, keys),
        }
        i = args;
    }
}

/// `plural` carries both keys as its trailing literals; the count argument
/// holds no string, so every literal in the call is a key.
fn collect_plural_keys(chars: &[char], start: usize, keys: &mut BTreeSet<String>) {
    let mut depth = 0;
    let mut j = start;
    while j < chars.len() {
        match chars[j] {
            '"' => {
                let (literal, next) = string_at(chars, j).expect("string literal");
                keys.insert(literal);
                j = next;
            }
            '(' => {
                depth += 1;
                j += 1;
            }
            ')' if depth == 0 => return,
            ')' => {
                depth -= 1;
                j += 1;
            }
            _ => j += 1,
        }
    }
}

/// The `t`/`tr`/`plural` call starting at `i`, with the index after its
/// opening parenthesis.
fn call_at(chars: &[char], i: usize) -> Option<(&'static str, usize)> {
    if i > 0 && is_ident(chars[i - 1]) {
        return None;
    }
    for name in ["plural", "tr", "t"] {
        let name_chars: Vec<char> = name.chars().collect();
        if !chars[i..].starts_with(&name_chars) {
            continue;
        }
        let mut j = i + name_chars.len();
        while j < chars.len() && chars[j].is_whitespace() {
            j += 1;
        }
        if j < chars.len() && chars[j] == '(' {
            return Some((name, j + 1));
        }
    }
    None
}

fn is_ident(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn skip_ws(chars: &[char], mut i: usize) -> usize {
    while i < chars.len() && chars[i].is_whitespace() {
        i += 1;
    }
    i
}

fn string_at(chars: &[char], i: usize) -> Option<(String, usize)> {
    if chars.get(i) != Some(&'"') {
        return None;
    }
    let mut literal = String::new();
    let mut j = i + 1;
    while j < chars.len() {
        match chars[j] {
            '\\' => {
                literal.push(chars[j]);
                if let Some(&escaped) = chars.get(j + 1) {
                    literal.push(escaped);
                }
                j += 2;
            }
            '"' => return Some((literal, j + 1)),
            c => {
                literal.push(c);
                j += 1;
            }
        }
    }
    None
}

fn placeholders(text: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let mut rest = text;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            break;
        };
        names.insert(after[..close].to_owned());
        rest = &after[close + 1..];
    }
    names
}
