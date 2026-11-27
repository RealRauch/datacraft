//! Mechanical t()-bypass guard over the UI crate's paint sites (step 94.1,
//! item T12; the AGENTS §6 i18n rule, LESSONS #26 "prose rules decay").
//!
//! The gap this closes: `panes.rs`'s `pane_header` (step 93.1) rendered a
//! visible pane title as a raw English literal — it compiled, painted
//! English in every language and passed the whole suite, because the
//! catalog-parity tests cover key/translation consistency, not call sites.
//! The pilot's title ("SQL") hid the gap because the English key equals the
//! German translation (LESSONS #7, a smooth fixture).
//!
//! The rule the scan enforces: at an egui paint call, a static string
//! literal may not sit in the user-visible-text argument position. Text
//! must reach the call through `t()`/`tr()` (localized) or be dynamic
//! (`format!`-built, a variable, a `&'static str` key). Deliberate painted
//! glyphs that never localize live in the explicit [`ALLOWED`] list below —
//! registered here, where the scan lives, each with the step that
//! introduced it (the glyph-coverage test's precedent, T11).
//!
//! Boundaries (a tripwire, not a full Rust parser): the scan is line-based
//! and reads the argument starting at the token's configured position, so
//! rustfmt-kept inline calls are seen — the fmt gate keeps paint calls on
//! one line (verified at step 94.1) — while wrapped literals on
//! continuation lines are not seen. Comment lines are skipped. The tree
//! walk skips this file: its fixtures are the self-test's intentional
//! literals (the debt-marker scan excludes its own workflow the same way).

/// A raw string literal detected at an egui paint call.
struct Violation {
    /// Source path relative to the crate's `src` directory, `/`-separated.
    file: String,
    /// One-based line number of the paint call.
    line: usize,
    /// The literal as it appears in the source, quotes included.
    literal: String,
    /// The paint token that carried the literal.
    token: &'static str,
}

/// Paint calls the scan watches as `(token, argument position of the
/// user-visible text)`. Token matching is word-bounded, so `small_button(`
/// never trips the `button(` token and `SelectableLabel::new(` never trips
/// `Label::new(`.
const PAINT_TOKENS: &[(&str, usize)] = &[
    ("label(", 0),
    ("button(", 0),
    ("small_button(", 0),
    ("menu_button(", 0),
    ("strong(", 0),
    ("heading(", 0),
    ("monospace(", 0),
    ("weak(", 0),
    ("selectable_label(", 1),
    ("toggle_value(", 1),
    ("on_hover_text(", 0),
    ("on_disabled_hover_text(", 0),
    ("Button::new(", 0),
    ("Label::new(", 0),
    ("SelectableLabel::new(", 0),
    ("RichText::new(", 0),
];

/// Deliberate painted literals (T12): glyphs that never localize and the
/// one empty button surface whose affordance is painted separately. An
/// entry is `(file, literal)` — the exact source text with quotes, the path
/// `/`-separated relative to `src`.
const ALLOWED: &[(&str, &str)] = &[
    // The Access new-record asterisk in the datasheet's trailing row (step
    // 11.7; `trailing.rs` is the 48.2 split) — a painted selector glyph.
    ("datasheet/trailing.rs", "\"*\""),
    // M121: the runtime combo paints its caret beside an empty button
    // surface — the label deliberately carries no text (`control.rs` is
    // the phase-102 split of `form_runtime.rs`).
    ("form_runtime/control.rs", "\"\""),
    // M56/M38: record-navigation prev/next arrows in the form's
    // navigation row (prev left of next).
    ("shell.rs", "\"\\u{25c0}\""),
    ("shell.rs", "\"\\u{25b6}\""),
    // Report preview page navigation (steps 10.4/91.11; `preview.rs` is
    // the phase-102 split of `report_design.rs` — F19).
    ("report_design/preview.rs", "\"\\u{25c0}\""),
    ("report_design/preview.rs", "\"\\u{25b6}\""),
    // U5: the Views split-button's overflow ellipsis.
    ("ribbon/views.rs", "\"…\""),
    // M82: the help glyph opens the shortcuts page.
    ("ribbon.rs", "\"?\""),
];

/// True when the violation is a registered painted literal.
fn is_allowed(violation: &Violation) -> bool {
    ALLOWED
        .iter()
        .any(|(file, literal)| *file == violation.file && *literal == violation.literal)
}

/// Scans one source file's text and returns every raw paint literal.
fn scan_file(rel_path: &str, source: &str) -> Vec<Violation> {
    let mut violations = Vec::new();
    for (index, line) in source.lines().enumerate() {
        if line.trim_start().starts_with("//") {
            continue;
        }
        for (token, literal) in scan_line(line) {
            violations.push(Violation {
                file: rel_path.to_string(),
                line: index + 1,
                literal,
                token,
            });
        }
    }
    violations
}

/// Finds raw string literals in paint-call argument positions on one line.
fn scan_line(line: &str) -> Vec<(&'static str, String)> {
    let mut found = Vec::new();
    for &(token, arg_index) in PAINT_TOKENS {
        let mut from = 0;
        while let Some(pos) = line[from..].find(token) {
            let abs = from + pos;
            from = abs + token.len();
            if !word_boundary(line, abs) {
                continue;
            }
            if let Some(literal) = argument_literal(line, from, arg_index) {
                found.push((token, literal.to_string()));
            }
        }
    }
    found
}

/// True when the token match at `pos` is not the tail of a longer
/// identifier (`small_button(` must not read as `button(`).
fn word_boundary(line: &str, pos: usize) -> bool {
    if pos == 0 {
        return true;
    }
    let before = line.as_bytes()[pos - 1];
    !before.is_ascii_alphanumeric() && before != b'_'
}

/// Extracts the string literal at `arg_index`'s argument start, if the
/// argument opens with a quote. Top-level commas split the arguments.
fn argument_literal(line: &str, args_from: usize, arg_index: usize) -> Option<&str> {
    if arg_index == 0 {
        return first_quoted(line, args_from);
    }
    let bytes = line.as_bytes();
    let mut depth = 0usize;
    let mut commas = 0usize;
    let mut i = args_from;
    while i < bytes.len() {
        match bytes[i] {
            b'(' => depth += 1,
            b')' => {
                if depth == 0 {
                    return None;
                }
                depth -= 1;
            }
            b',' if depth == 0 => {
                commas += 1;
                if commas == arg_index {
                    return first_quoted(line, i + 1);
                }
            }
            _ => {}
        }
        i += 1;
    }
    None
}

/// The quoted literal starting at (or after whitespace from) `from`.
fn first_quoted(line: &str, from: usize) -> Option<&str> {
    let bytes = line.as_bytes();
    let mut i = from;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'"' {
        Some(quoted(line, i))
    } else {
        None
    }
}

/// The literal token from its opening quote to the closing quote; an
/// unterminated run returns the rest of the line so it stays visible.
fn quoted(line: &str, start: usize) -> &str {
    let bytes = line.as_bytes();
    let mut i = start + 1;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b'"' => return &line[start..=i],
            _ => i += 1,
        }
    }
    &line[start..]
}

/// Recursively collects the crate's Rust sources under `dir`.
fn collect_rs_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
    let entries = std::fs::read_dir(dir).expect("read UI source directory");
    for entry in entries {
        let path = entry.expect("read directory entry").path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn a_raw_literal_in_a_paint_call_is_flagged() {
    let source = "fn paint(ui: &mut egui::Ui) {\n    ui.label(\"Uncatalogued\");\n}\n";
    let violations = scan_file("self_test.rs", source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].token, "label(");
    assert_eq!(violations[0].literal, "\"Uncatalogued\"");
    assert_eq!(violations[0].line, 2);
}

#[test]
fn localized_and_dynamic_arguments_are_clean() {
    let source = concat!(
        "ui.label(t(\"Save\"));\n",
        "ui.label(tr(\"Rows\", count));\n",
        "ui.label(&format!(\"{} rows\", count));\n",
        "ui.label(title);\n",
        "ui.strong(t(title_key));\n",
        "egui::Button::new(t(\"Close\"));\n",
        "ui.weak(t(\"No entries\"));\n",
    );
    assert!(scan_file("self_test.rs", source).is_empty());
}

#[test]
fn second_position_labels_are_checked() {
    let source = concat!(
        "ui.selectable_label(on, \"Raw second\");\n",
        "ui.selectable_label(on, t(\"Fine\"));\n",
        "ui.toggle_value(&mut flag, \"Raw toggle\");\n",
        "ui.toggle_value(&mut flag, t(\"Fine\"));\n",
    );
    let violations = scan_file("self_test.rs", source);
    let literals: Vec<&str> = violations.iter().map(|v| v.literal.as_str()).collect();
    assert_eq!(literals, ["\"Raw second\"", "\"Raw toggle\""]);
}

#[test]
fn lookalike_identifiers_do_not_trip_the_scan() {
    let source = concat!(
        "relabel(\"an ordinary identifier\");\n",
        "view_label(\"also ordinary\");\n",
        "ui.small_button(\"\\u{25c0}\").clicked();\n",
    );
    let violations = scan_file("self_test.rs", source);
    // `relabel(`/`view_label(` are word-bounded away from `label(`, and the
    // `button(` token never matches inside `small_button(`.
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].token, "small_button(");
    assert_eq!(violations[0].literal, "\"\\u{25c0}\"");
}

#[test]
fn comments_are_skipped_and_constructors_are_unwrapped() {
    let source = concat!(
        "// ui.label(\"a comment example\");\n",
        "egui::Button::new(egui::RichText::new(\"?\").strong());\n",
    );
    let violations = scan_file("self_test.rs", source);
    assert_eq!(violations.len(), 1);
    assert_eq!(violations[0].token, "RichText::new(");
    assert_eq!(violations[0].literal, "\"?\"");
}

#[test]
fn allowlisted_painted_glyphs_are_clean() {
    let arrow = scan_file("shell.rs", "ui.small_button(\"\\u{25c0}\").clicked();\n");
    assert_eq!(arrow.len(), 1);
    assert!(is_allowed(&arrow[0]));
    let asterisk = scan_file(
        "datasheet/trailing.rs",
        "ui.label(egui::RichText::new(\"*\").strong());\n",
    );
    assert_eq!(asterisk.len(), 1);
    assert!(is_allowed(&asterisk[0]));
    // The same glyph in an unregistered file stays a violation.
    let misplaced = scan_file(
        "nav_pane.rs",
        "ui.label(egui::RichText::new(\"*\").strong());\n",
    );
    assert_eq!(misplaced.len(), 1);
    assert!(!is_allowed(&misplaced[0]));
}

#[test]
fn every_ui_paint_literal_is_catalogued_or_allowlisted() {
    let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs_files(&src_dir, &mut files);
    assert!(
        files.len() > 40,
        "the walk saw only {} source files — the scan must cover the whole UI crate",
        files.len()
    );
    assert!(
        files.iter().any(|p| p.ends_with("panes.rs")),
        "the walk missed panes.rs — the T12 regression file must stay covered"
    );
    let mut violations = Vec::new();
    for path in &files {
        let rel = path
            .strip_prefix(&src_dir)
            .expect("walk paths live under src")
            .to_string_lossy()
            .replace('\\', "/");
        if rel == "i18n/bypass.rs" {
            continue;
        }
        let source = std::fs::read_to_string(path).expect("read UI source file");
        violations.extend(scan_file(&rel, &source));
    }
    let unexpected: Vec<String> = violations
        .iter()
        .filter(|v| !is_allowed(v))
        .map(|v| format!("{}:{} {} {}", v.file, v.line, v.token, v.literal))
        .collect();
    assert!(
        unexpected.is_empty(),
        "raw literals at egui paint calls outside the allowlist — wrap in \
         t()/tr() or register the painted glyph in ALLOWED:\n{}",
        unexpected.join("\n")
    );
}
