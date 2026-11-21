//! Object Details view suite (step 92.5, item L48; the Type column is
//! step 93.3, item L64).
//!
//! The flat details list renders each row's localized kind as a trailing
//! Type column — the row keeps its name label and the kind rides the
//! accessibility value — and keeps the pane sort (Name/Type) and the
//! search narrowing. The current UI language is process-wide, so the
//! language passes here run serialized on a file-local guard and every
//! test restores the language it found (the ribbon-overflow precedent).

use std::path::Path;
use std::sync::Mutex;

use datacraft_core::command::Command;
use datacraft_core::macro_def::MacroDef;
use datacraft_core::nav::NavSortOrder;
use datacraft_core::port::MockPort;
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::i18n::{self, Lang};
use datacraft_ui::layout_probe;
use datacraft_ui::nav_groups::{NavView, prefs_key};
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};

/// Serializes every test in this binary: the language is process state.
static LANG_GUARD: Mutex<()> = Mutex::new(());

/// Runs `f` under the guard with the language switched, then restores.
fn with_lang<T>(lang: Lang, f: impl FnOnce() -> T) -> T {
    let _guard = LANG_GUARD.lock().unwrap_or_else(|error| error.into_inner());
    let before = i18n::lang();
    i18n::set_lang(lang);
    let result = f();
    i18n::set_lang(before);
    result
}

/// A table plus a macro: two kinds whose Name and Type orders differ —
/// `mac` sorts before `Zeta` by name, the Type rank puts the table first.
fn details_app() -> App {
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "Zeta",
        vec![FieldDef::new(
            "Id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    )]);
    port.with_macros(vec![MacroDef {
        name: "mac".into(),
        actions: vec![],
        definition_decoded: false,
    }]);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    let key = prefs_key(Path::new("shop.mdb"));
    app.set_nav_view(&key, NavView::Details);
    app
}

fn details_harness(app: App) -> Harness<'static, App> {
    let mut harness = Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            layout_probe::clear();
            datacraft_ui::nav_pane::nav_pane_ui(app, ui);
        },
        app,
    );
    // Tall enough that the whole scroll content is inside the viewport —
    // egui culls off-viewport nodes from the accessibility tree.
    harness.set_size(egui::vec2(600.0, 1600.0));
    harness
}

/// Semantic tree dump: role + label + value + disabled state of every
/// labelled node, in traversal order. Deterministic across DPI and fonts.
fn tree_dump(harness: &Harness<'_, App>) -> String {
    let mut lines = Vec::new();
    for node in harness.root().children_recursive() {
        let node = node.accesskit_node();
        let label = node.label().unwrap_or_default();
        let value = node.value().unwrap_or_default();
        if label.is_empty() && value.is_empty() {
            continue;
        }
        let disabled = if node.is_disabled() { " disabled" } else { "" };
        lines.push(format!("{:?}|{label}|{value}{disabled}", node.role()));
    }
    lines.join("\n")
}

/// Clicks into the search field (press + release at the probe rect) and
/// types `text` (the nav-search suite's pattern).
fn type_into_search(harness: &mut Harness<'static, App>, text: &str) {
    harness.run();
    let field = layout_probe::drain()
        .iter()
        .find(|(id, _)| id == "nav.search")
        .expect("the search field records its rect")
        .1;
    harness.hover_at(field.center());
    harness.drag_at(field.center());
    harness.drop_at(field.center());
    harness.run();
    harness.event(egui::Event::Text(text.to_owned()));
    harness.run();
    harness.run();
}

/// The details list carries each row's localized kind as a trailing Type
/// column (step 93.3, item L64): the name stays the row's label, the kind
/// rides the accessibility value — English resolves the identity.
#[test]
fn the_details_list_carries_the_type_column_in_english() {
    with_lang(Lang::English, || {
        let mut harness = details_harness(details_app());
        harness.run();
        let dump = tree_dump(&harness);
        assert!(
            dump.contains("Button|Zeta|Table"),
            "the table row carries its kind value:\n{dump}"
        );
        assert!(
            dump.contains("Button|mac|Macro"),
            "the macro row carries its kind value:\n{dump}"
        );
    });
}

/// The Type column follows the language (L64): the German catalog
/// resolves the kind labels.
#[test]
fn the_type_column_follows_the_language() {
    with_lang(Lang::German, || {
        let mut harness = details_harness(details_app());
        harness.run();
        let dump = tree_dump(&harness);
        assert!(
            dump.contains("Button|Zeta|Tabelle"),
            "the table row carries the German kind value:\n{dump}"
        );
        assert!(
            dump.contains("Button|mac|Makro"),
            "the macro row carries the German kind value:\n{dump}"
        );
    });
}

/// The category tree keeps plain rows — the kind value renders only in
/// the details view (step 93.3, item L64).
#[test]
fn the_tree_view_keeps_plain_rows() {
    with_lang(Lang::English, || {
        let mut app = details_app();
        let key = prefs_key(Path::new("shop.mdb"));
        app.set_nav_view(&key, NavView::Tree);
        let mut harness = details_harness(app);
        harness.run();
        let dump = tree_dump(&harness);
        assert!(
            dump.contains("Button|Zeta|"),
            "the tree row stays listed:\n{dump}"
        );
        assert!(
            !dump.contains("Button|Zeta|Table"),
            "the tree row carries no kind value:\n{dump}"
        );
    });
}

/// The pane sort reorders the details list (L64 keeps the nav sort):
/// Name lists `mac` first, Type puts the table first.
#[test]
fn the_details_list_reorders_with_the_pane_sort() {
    with_lang(Lang::English, || {
        let mut app = details_app();
        app.nav_sort = NavSortOrder::Name;
        let mut harness = details_harness(app);
        harness.run();
        let dump = tree_dump(&harness);
        let macro_row = dump
            .find("Button|mac|Macro")
            .expect("the macro row is listed");
        let table_row = dump
            .find("Button|Zeta|Table")
            .expect("the table row is listed");
        assert!(
            macro_row < table_row,
            "Name sort lists the macro first:\n{dump}"
        );

        let mut app = details_app();
        app.nav_sort = NavSortOrder::Type;
        let mut harness = details_harness(app);
        harness.run();
        let dump = tree_dump(&harness);
        let macro_row = dump
            .find("Button|mac|Macro")
            .expect("the macro row is listed");
        let table_row = dump
            .find("Button|Zeta|Table")
            .expect("the table row is listed");
        assert!(
            table_row < macro_row,
            "Type sort lists the table first:\n{dump}"
        );
    });
}

/// Searching narrows the details list (L64 keeps the search filter): the
/// non-match hides, the match keeps its kind value.
#[test]
fn the_details_list_narrows_with_the_search() {
    with_lang(Lang::English, || {
        let mut harness = details_harness(details_app());
        type_into_search(&mut harness, "zeta");
        let dump = tree_dump(&harness);
        assert!(
            dump.contains("Button|Zeta|Table"),
            "the match stays:\n{dump}"
        );
        assert!(
            !dump.contains("Button|mac|"),
            "the non-match hides:\n{dump}"
        );
    });
}
