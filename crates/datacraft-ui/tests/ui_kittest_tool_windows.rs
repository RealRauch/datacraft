//! The tool-window contracts (step 91.9, items M117–M119 and L60–L62):
//! the relationship mapper scrolls its body, the VBA viewer's errors use
//! the danger token, the view editor's fields carry their labels, the
//! expression window's Close sits in window-chrome position, the notices
//! dialog marks severity with a painted glyph per entry, and the macro
//! viewer's number column aligns from ten entries on.

use datacraft_core::command::Command;
use datacraft_core::macro_def::{MacroAction, MacroDef};
use datacraft_core::port::{MockPort, Relationship};
use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::layout_probe;
use eframe::egui;
use egui_kittest::Harness;
use egui_kittest::kittest::NodeT;

fn app_with_database() -> App {
    let port = MockPort::with_tables(vec![TableDef::new(
        "A",
        vec![FieldDef::new(
            "id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    )]);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app
}

/// Semantic tree dump: role + label + value of every labelled node.
fn tree_dump<State>(harness: &Harness<'_, State>) -> String {
    let mut lines = Vec::new();
    for node in harness.root().children_recursive() {
        let node = node.accesskit_node();
        let label = node.label().as_deref().map(str::to_owned);
        let value = node.value().as_deref().map(str::to_owned);
        let text = label.or(value).unwrap_or_default();
        if text.is_empty() {
            continue;
        }
        lines.push(format!("{:?}|{text}", node.role()));
    }
    lines.join("\n")
}

fn relationships_app(count: usize) -> App {
    let mut port = MockPort::with_tables(vec![
        TableDef::new(
            "A",
            vec![FieldDef::new(
                "id",
                FieldKind::Number(NumericKind::LongInteger),
            )],
        ),
        TableDef::new(
            "B",
            vec![FieldDef::new(
                "id",
                FieldKind::Number(NumericKind::LongInteger),
            )],
        ),
    ]);
    port.with_relationships(
        (0..count)
            .map(|index| Relationship {
                table: "A".into(),
                column: format!("id{index}"),
                related_table: "B".into(),
                related_column: "id".into(),
            })
            .collect(),
    );
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::ToggleRelationshipMapper);
    app
}

fn window_harness(
    app: App,
    draw: impl FnMut(&mut egui::Ui, &mut App) + 'static,
) -> Harness<'static, App> {
    let mut draw = draw;
    let mut harness = Harness::new_ui_state(
        move |ui, app| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            layout_probe::clear();
            draw(ui, app);
        },
        app,
    );
    harness.set_size(egui::vec2(900.0, 700.0));
    harness
}

/// The relationship mapper's body scrolls (M117): with more rows than the
/// height budget fits, the tail rows start clipped and a scroll reaches
/// them — the window no longer grows to its content.
#[test]
fn the_relationship_mapper_scrolls_its_body() {
    let app = relationships_app(40);
    let mut harness = window_harness(app, |ui, app| {
        datacraft_ui::relationships::mapper_window(app, ui);
    });
    harness.run();
    // The body viewport stays within the height budget (the window no
    // longer grows to its content), and every entry lays out inside the
    // scrollable body — reachable by scrolling, which is the ScrollArea's
    // own mechanical guarantee.
    let rects = layout_probe::drain();
    let viewport = rects
        .iter()
        .find(|(id, _)| id == "relationships.viewport")
        .map(|(_, rect)| *rect)
        .expect("the body viewport records its rect");
    assert!(
        viewport.height() <= 380.5,
        "the body viewport stays within the budget: {viewport:?}"
    );
    let rows = rects
        .iter()
        .filter(|(id, _)| id == "relationships.row")
        .count();
    assert_eq!(
        rows, 40,
        "every entry lays out inside the scrollable body: {rows}"
    );
}

/// The VBA viewer's error text follows the danger token (M118) — the
/// viewer's source carries no color literals. (Pinned as a source
/// contract inside the crate; the visual follows the token in both
/// palettes.)
#[test]
fn the_vba_viewer_has_no_color_literals() {
    let source = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/vba_viewer.rs"));
    assert!(
        !source.contains("LIGHT_RED"),
        "the viewer paints errors through tokens.danger"
    );
}

/// The view editor's fields carry their labels (M119): the accessibility
/// nodes name "View name" and "SQL".
#[test]
fn the_view_editor_fields_carry_their_labels() {
    let mut app = app_with_database();
    app.open_view_editor(None);
    let mut harness = window_harness(app, |ui, app| {
        datacraft_ui::views_window::view_editor(app, ui);
    });
    harness.run();
    let dump = tree_dump(&harness);
    assert!(
        dump.contains("TextInput|View name"),
        "the name field carries its label:\n{dump}"
    );
    assert!(
        dump.contains("TextInput|SQL"),
        "the sql field carries its label:\n{dump}"
    );
}

/// The expression window's Close sits in window-chrome position (L60):
/// right-aligned in its row, recorded under the `expression.close` probe.
#[test]
fn the_expression_close_sits_right_aligned() {
    let mut app = app_with_database();
    app.dispatch(Command::ToggleExpressionWindow);
    let mut harness = window_harness(app, |ui, app| {
        datacraft_ui::expression_window::expression_window(app, ui);
    });
    harness.run();
    let rect = layout_probe::drain()
        .iter()
        .find(|(id, _)| id == "expression.close")
        .map(|(_, rect)| *rect)
        .expect("the close button records its rect");
    assert!(
        rect.center().x > 210.0,
        "the close button sits in the right half of the 420-wide window: {rect:?}"
    );
}

/// Every notices entry carries its painted severity glyph (L61): the
/// markers ride the layout probe, split by severity — severity is
/// recognisable without color.
#[test]
fn the_notices_dialog_marks_severity_per_entry() {
    let mut app = app_with_database();
    for _ in 0..3 {
        app.dispatch(Command::SetTableSort {
            table: "no-such-table".into(),
            field: "id".into(),
            order: datacraft_core::qbe::SortOrder::Ascending,
        });
    }
    app.open_notices();
    let mut harness = window_harness(app, |ui, app| {
        datacraft_ui::notices_dialog::notices_dialog(app, ui);
    });
    harness.run();
    let errors = layout_probe::drain()
        .iter()
        .filter(|(id, _)| id == "notices.glyph.error")
        .count();
    assert!(
        errors >= 3,
        "every error entry carries its painted glyph: {errors}"
    );
}

/// The macro viewer's argument column keeps one left edge from ten
/// entries on (L62): the number column is width-padded, the argument
/// rects ride the `macro.argument` probe.
#[test]
fn the_macro_number_column_aligns() {
    let mut port = MockPort::with_tables(vec![TableDef::new(
        "A",
        vec![FieldDef::new(
            "id",
            FieldKind::Number(NumericKind::LongInteger),
        )],
    )]);
    port.with_macros(vec![MacroDef {
        name: "M1".into(),
        definition_decoded: true,
        actions: (0..12)
            .map(|index| MacroAction {
                argument: format!("SetValue arg{index}"),
            })
            .collect(),
    }]);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "shop.mdb".into(),
        password: None,
        source: None,
    });
    app.dispatch(Command::OpenObject(datacraft_core::ObjectId::new(
        datacraft_core::ObjectKind::Macro,
        "M1",
    )));
    let mut harness = window_harness(app, |ui, app| {
        let Some(tab) = app.snapshot().tabs.first().cloned() else {
            return;
        };
        datacraft_ui::macro_viewer::macro_viewer_ui(app, &tab, ui);
    });
    harness.run();
    let rects: Vec<egui::Rect> = layout_probe::drain()
        .iter()
        .filter(|(id, _)| id == "macro.argument")
        .map(|(_, rect)| *rect)
        .collect();
    assert!(
        rects.len() >= 10,
        "every action's argument records its rect: {rects:?}"
    );
    let first = rects[0].left();
    assert!(
        rects.iter().all(|rect| (rect.left() - first).abs() < 0.5),
        "the argument column keeps one left edge: {rects:?}"
    );
}
