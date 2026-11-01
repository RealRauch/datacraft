use super::*;
use datacraft_core::ObjectKind;
use datacraft_core::ribbon::ribbon_spec;
use eframe::egui;
use eframe::egui::Color32;

/// Paints one icon into a headless pass and reports dispatch + shapes.
fn paint_once(name: &str) -> (bool, usize) {
    let ctx = egui::Context::default();
    let mut dispatched = false;
    let mut output = ctx.run_pass(egui::RawInput::default(), |ctx| {
        let painter = ctx.layer_painter(egui::LayerId::background());
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 10.0), egui::vec2(16.0, 16.0));
        dispatched = paint(&painter, rect, name, ink(), accent());
    });
    // Headless: there is no painter that would apply the font-atlas delta.
    output.textures_delta.clear();
    (dispatched, output.shapes.len())
}

fn ink() -> Color32 {
    Color32::from_rgb(0x24, 0x24, 0x24)
}

fn accent() -> Color32 {
    Color32::from_rgb(0x0F, 0x8B, 0x7E)
}

/// Every ribbon action id from the core spec resolves to an icon.
#[test]
fn every_ribbon_action_has_an_icon() {
    for tab in &ribbon_spec().tabs {
        for group in &tab.groups {
            for action in &group.actions {
                let (dispatched, shapes) = paint_once(action.id);
                assert!(dispatched, "ribbon action {:?} has no icon", action.id);
                assert!(shapes > 0, "ribbon action {:?} paints nothing", action.id);
            }
        }
    }
}

/// Every Quick Access Toolbar entry resolves to an icon (step 21.2).
#[test]
fn every_quick_access_action_has_an_icon() {
    for entry in &ribbon_spec().quick_access {
        let (dispatched, shapes) = paint_once(entry.id);
        assert!(dispatched, "QAT action {:?} has no icon", entry.id);
        assert!(shapes > 0, "QAT action {:?} paints nothing", entry.id);
    }
}

/// Every object kind (nav pane, document tabs) resolves to an icon.
#[test]
fn every_object_kind_has_an_icon() {
    for kind in [
        ObjectKind::Table,
        ObjectKind::Query,
        ObjectKind::Form,
        ObjectKind::Report,
        ObjectKind::Macro,
        ObjectKind::Module,
    ] {
        let (dispatched, shapes) = paint_once(kind.slug());
        assert!(dispatched, "object kind {:?} has no icon", kind.slug());
        assert!(shapes > 0, "object kind {:?} paints nothing", kind.slug());
    }
}

/// An unknown key is a no-op, never a panic or a stray shape.
#[test]
fn unknown_names_paint_nothing() {
    let (dispatched, shapes) = paint_once("no_such_icon");
    assert!(!dispatched);
    assert_eq!(shapes, 0);
}
