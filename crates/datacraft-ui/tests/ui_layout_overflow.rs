//! Ribbon overflow pins (step 88.1, item M98).
//!
//! The current UI language is process-wide, so the language passes here
//! run in their own binary and serialize on a file-local guard; every
//! test restores the language it found. Rect contracts assert the
//! overflow boundary at the acceptance sizes, the narrow-width pins
//! prove every action stays reachable (row or overflow menu), and the
//! dispatch pin proves the menu items fire the same commands as the
//! row buttons.

use std::sync::Mutex;

use datacraft_core::command::Command;
use datacraft_core::port::MockPort;
use datacraft_core::ribbon::ribbon_spec;
use datacraft_core::schema::{FieldDef, FieldKind, FieldValue, NumericKind, TableDef};
use datacraft_ui::app::App;
use datacraft_ui::i18n::{self, Lang, translate};
use datacraft_ui::layout_probe;
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

fn app_with_tables() -> App {
    let port = MockPort::with_tables(vec![TableDef::new(
        "ServiceOrders",
        vec![
            FieldDef::new("Id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("Description", FieldKind::Text { max_len: 50 }),
        ],
    )]);
    let mut app = App::new(Box::new(port));
    app.dispatch(Command::OpenDatabase {
        path: "database.mdb".into(),
        password: None,
        source: None,
    });
    app
}

fn ribbon_rects(mut app: App, ribbon_tab: &'static str, size: egui::Vec2) -> Vec<(String, egui::Rect)> {
    app.active_ribbon_tab = ribbon_tab;
    let mut harness = Harness::new_ui_state(
        move |ui, app: &mut App| {
            datacraft_ui::theme::fonts::install(ui.ctx());
            datacraft_ui::theme::apply(ui.ctx());
            layout_probe::clear();
            datacraft_ui::ribbon::ribbon_ui(app, ui);
            layout_probe::record("probe.window", ui.max_rect());
        },
        app,
    );
    harness.set_size(size);
    harness.run();
    layout_probe::drain()
}

/// Rect ids recorded for one tab's groups (and its overflow control).
fn group_rects<'a>(rects: &'a [(String, egui::Rect)], tab: &str) -> Vec<&'a (String, egui::Rect)> {
    rects
        .iter()
        .filter(|(id, _)| {
            id.starts_with(&format!("ribbon.{tab}."))
                && !id.ends_with(".overflow")
                && id.as_str() != format!("ribbon.{tab}.").as_str()
        })
        .collect()
}

fn window_rect(rects: &[(String, egui::Rect)]) -> egui::Rect {
    rects
        .iter()
        .find(|(id, _)| id == "probe.window")
        .map(|(_, rect)| *rect)
        .expect("window rect recorded")
}

fn overflow_rect(rects: &[(String, egui::Rect)], tab: &str) -> Option<egui::Rect> {
    rects
        .iter()
        .find(|(id, _)| id == &format!("ribbon.{tab}.overflow"))
        .map(|(_, rect)| *rect)
}

/// All labels visible in the tree, including open popup menus.
fn tree_labels(harness: &Harness<'_, App>) -> Vec<String> {
    let mut labels = Vec::new();
    for node in harness.root().children_recursive() {
        let node = node.accesskit_node();
        let label = node.label().unwrap_or_default();
        let value = node.value().unwrap_or_default();
        if !label.is_empty() {
            labels.push(label.to_owned());
        }
        if !value.is_empty() {
            labels.push(value);
        }
    }
    labels
}

/// The overflow boundary follows the reachability contract at the
/// acceptance sizes in both languages: a tab either paints all of its
/// groups inside the window, or collapses the suffix behind the
/// overflow control — which itself stays inside the window (M98).
#[test]
fn overflow_boundary_at_the_acceptance_sizes() {
    for lang in [Lang::English, Lang::German] {
        with_lang(lang, || {
            for size in [egui::vec2(1280.0, 800.0), egui::vec2(960.0, 600.0), egui::vec2(800.0, 600.0)] {
                for tab in ribbon_spec().tabs.iter().filter(|tab| !tab.groups.is_empty()) {
                    let rects = ribbon_rects(app_with_tables(), tab.id, size);
                    let window = window_rect(&rects);
                    let painted = group_rects(&rects, tab.id);
                    let overflow = overflow_rect(&rects, tab.id);
                    assert_eq!(
                        painted.len() + usize::from(overflow.is_some()),
                        tab.groups.len(),
                        "{} on {}: {} groups painted, overflow {} — the plan must keep every \
                         group either in the row or behind the control",
                        tab.id,
                        lang_debug(lang),
                        painted.len(),
                        overflow.is_some(),
                    );
                    for (id, rect) in &painted {
                        assert!(
                            rect.right() <= window.right() + 1.0,
                            "group {id} paints at {rect:?}, beyond the window {window:?} at {size:?} ({})",
                            lang_debug(lang),
                        );
                    }
                    if let Some(rect) = overflow {
                        assert!(
                            rect.right() <= window.right() + 1.0,
                            "the overflow control paints at {rect:?}, beyond the window \
                             {window:?} at {size:?} ({})",
                            lang_debug(lang),
                        );
                        assert!(
                            painted.len() < tab.groups.len(),
                            "an overflow control without hidden groups is noise at {size:?}"
                        );
                    } else {
                        assert_eq!(
                            painted.len(),
                            tab.groups.len(),
                            "{} groups painted without an overflow control on {} at {size:?} ({})",
                            painted.len(),
                            tab.id,
                            lang_debug(lang),
                        );
                    }
                    if size == egui::vec2(1280.0, 800.0) {
                        assert!(
                            overflow.is_none(),
                            "{} overflows at the shipped default size {size:?} ({}) — the fit \
                             math collapses groups prematurely",
                            tab.id,
                            lang_debug(lang),
                        );
                    }
                }
            }
        });
    }
}

fn lang_debug(lang: Lang) -> &'static str {
    match lang {
        Lang::English => "en",
        Lang::German => "de",
    }
}

/// At narrow widths the collapsed groups' actions stay reachable: the
/// overflow menu (opened through the localized control) lists every
/// hidden action beside the ones still painted in the row (M98).
#[test]
fn narrow_widths_keep_every_action_reachable_through_the_menu() {
    // English at 320px: nothing but the control may fit; German at 640px:
    // a prefix fits and the rest collapses. Both must cover every action.
    for (lang, size, control) in [
        (Lang::English, egui::vec2(320.0, 400.0), "More"),
        (Lang::German, egui::vec2(640.0, 400.0), "Mehr"),
    ] {
        with_lang(lang, || {
            let tab = ribbon_spec()
                .tabs
                .iter()
                .find(|tab| tab.id == "create")
                .expect("create tab in the spec");
            let mut app = app_with_tables();
            app.active_ribbon_tab = "create";
            let mut harness = Harness::new_ui_state(
                move |ui, app: &mut App| {
                    datacraft_ui::theme::fonts::install(ui.ctx());
                    datacraft_ui::theme::apply(ui.ctx());
                    layout_probe::clear();
                    datacraft_ui::ribbon::ribbon_ui(app, ui);
                    layout_probe::record("probe.window", ui.max_rect());
                },
                app,
            );
            harness.set_size(size);
            harness.run();
            let rects = layout_probe::drain();
            let overflow = overflow_rect(&rects, "create");
            assert!(
                overflow.is_some(),
                "the create tab must overflow at {size:?} ({})",
                lang_debug(lang),
            );
            let window = window_rect(&rects);
            assert!(
                overflow.unwrap().right() <= window.right() + 1.0,
                "the overflow control must stay inside the window ({})",
                lang_debug(lang),
            );

            harness.get_by_label(control).click();
            harness.run();
            let labels = tree_labels(&harness);
            for group in &tab.groups {
                for action in &group.actions {
                    let expected = translate(lang, action.label);
                    assert!(
                        labels.iter().any(|label| label == expected),
                        "action {:?} ({expected:?}) is unreachable at {size:?} ({}) — neither \
                         painted in the row nor offered in the overflow menu; labels: {labels:?}",
                        action.id,
                        lang_debug(lang),
                    );
                }
            }
        });
    }
}

/// A menu item dispatches the same command its row button would: at a
/// width where Report Design collapsed into the overflow menu, clicking
/// it there creates the report exactly like the visible-row path (M98).
#[test]
fn overflow_menu_item_dispatches_the_row_command() {
    with_lang(Lang::English, || {
        let mut app = app_with_tables();
        app.active_ribbon_tab = "create";
        assert!(app.reports().is_empty(), "the pin starts without reports");
        let mut harness = Harness::new_ui_state(
            move |ui, app: &mut App| {
                datacraft_ui::theme::fonts::install(ui.ctx());
                datacraft_ui::theme::apply(ui.ctx());
                layout_probe::clear();
                datacraft_ui::ribbon::ribbon_ui(app, ui);
            },
            app,
        );
        harness.set_size(egui::vec2(320.0, 400.0));
        harness.run();
        harness.get_by_label("More").click();
        harness.run();
        harness.get_by_label("Report Design").click();
        harness.run();
        let reports = harness.state().reports();
        assert_eq!(
            reports.len(),
            1,
            "the overflow menu item must dispatch the same CreateReport command as the row \
             button; reports: {reports:?}"
        );
        assert_eq!(reports[0].name, "Report1");
    });
}
