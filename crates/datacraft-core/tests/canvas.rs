//! Contract suite for the WYSIWYG canvas (step 8.1, item H28).
//!
//! Pure geometry: snap-to-grid is idempotent, hit-testing is the inverse
//! of rect geometry, forms are first-class design objects in the state,
//! snapshot and sidecar.

use std::path::PathBuf;

use datacraft_core::canvas::{Rect, hit_test, snap, snap_rect};
use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::form::FormDef;
use datacraft_core::port::MockPort;
use datacraft_core::schema::SchemaDocument;
use proptest::prelude::*;

// ---------------------------------------------------------------------------
// snap-to-grid math
// ---------------------------------------------------------------------------

#[test]
fn snap_rounds_to_the_grid_and_is_idempotent() {
    assert_eq!(snap(37.0, 8.0), 40.0);
    assert_eq!(snap(36.0, 8.0), 40.0);
    assert_eq!(snap(32.0, 8.0), 32.0);
    assert_eq!(snap(-3.0, 8.0), 0.0, "coordinates are non-negative");
    let once = snap(37.0, 8.0);
    assert_eq!(snap(once, 8.0), once);
}

#[test]
fn snap_rect_snaps_origin_and_size() {
    let rect = snap_rect(
        Rect {
            x: 37.0,
            y: 5.0,
            width: 61.0,
            height: 15.0,
        },
        8.0,
    );
    assert_eq!(
        rect,
        Rect {
            x: 40.0,
            y: 8.0,
            width: 64.0,
            height: 16.0
        }
    );
}

#[test]
fn hit_test_finds_the_topmost_rect() {
    let rects = vec![
        Rect {
            x: 0.0,
            y: 0.0,
            width: 100.0,
            height: 50.0,
        },
        Rect {
            x: 10.0,
            y: 10.0,
            width: 20.0,
            height: 20.0,
        },
    ];
    assert_eq!(hit_test(&rects, 15.0, 15.0), Some(1), "later rects win");
    assert_eq!(hit_test(&rects, 5.0, 5.0), Some(0));
    assert_eq!(hit_test(&rects, 500.0, 500.0), None);
}

proptest! {
    /// Snap is idempotent for any coordinate and grid divisor.
    #[test]
    fn snap_is_idempotent(value in -100.0f32..10_000.0, grid in 1.0f32..64.0) {
        let once = snap(value, grid);
        prop_assert_eq!(snap(once, grid), once);
    }

    /// A snapped rect stays snapped and non-degenerate.
    #[test]
    fn snapped_rects_are_stable(
        x in 0.0f32..4_000.0,
        y in 0.0f32..4_000.0,
        w in 1.0f32..1_000.0,
        h in 1.0f32..1_000.0,
        grid in 1.0f32..32.0,
    ) {
        let snapped = snap_rect(Rect { x, y, width: w, height: h }, grid);
        prop_assert_eq!(snap_rect(snapped, grid), snapped);
        prop_assert!(snapped.width > 0.0 && snapped.height > 0.0);
    }

    /// Hit-testing a point inside a rect returns an index whose rect
    /// contains it; outside points return nothing.
    #[test]
    fn hit_test_agrees_with_containment(
        rects in proptest::collection::vec(
            (0.0f32..500.0, 0.0f32..500.0, 10.0f32..200.0, 10.0f32..200.0)
                .prop_map(|(x, y, w, h)| Rect { x, y, width: w, height: h }),
            0..8,
        ),
        px in 0.0f32..800.0,
        py in 0.0f32..800.0,
    ) {
        if let Some(i) = hit_test(&rects, px, py) {
            let r = &rects[i];
            prop_assert!(px >= r.x && px < r.x + r.width && py >= r.y && py < r.y + r.height);
        }
    }
}

// ---------------------------------------------------------------------------
// forms in the command bus
// ---------------------------------------------------------------------------

fn opened_core() -> Core {
    let mut core = Core::new(Box::new(MockPort::new()));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
    });
    core
}

#[test]
fn create_form_is_observable_and_lands_in_the_snapshot() {
    let mut core = opened_core();
    assert_eq!(
        core.dispatch(Command::CreateForm {
            name: "Customer Card".into()
        }),
        vec![Event::FormCreated {
            name: "Customer Card".into()
        }]
    );
    let db = core.inspect().database.expect("open");
    assert_eq!(db.forms.len(), 1);
    assert_eq!(db.forms[0].name, "Customer Card");
    assert!(db.forms[0].controls.is_empty());
    assert_eq!(db.forms[0].grid, 8.0, "Access-like default snap grid");
}

#[test]
fn duplicate_form_names_are_rejected() {
    let mut core = opened_core();
    core.dispatch(Command::CreateForm {
        name: "Card".into(),
    });
    let reason = match core
        .dispatch(Command::CreateForm {
            name: "card".into(),
        })
        .as_slice()
    {
        [Event::Failed { reason }] => reason.clone(),
        other => panic!("expected Failed, got {other:?}"),
    };
    assert!(reason.contains("duplicate"), "{reason}");
}

#[test]
fn delete_form_is_observable() {
    let mut core = opened_core();
    core.dispatch(Command::CreateForm {
        name: "Card".into(),
    });
    assert_eq!(
        core.dispatch(Command::DeleteForm {
            name: "Card".into()
        }),
        vec![Event::FormDropped {
            name: "Card".into()
        }]
    );
    assert!(core.inspect().database.unwrap().forms.is_empty());
}

#[test]
fn forms_need_an_open_database() {
    let reason = match Core::new(Box::new(MockPort::new()))
        .dispatch(Command::CreateForm {
            name: "Card".into(),
        })
        .as_slice()
    {
        [Event::Failed { reason }] => reason.clone(),
        other => panic!("expected Failed, got {other:?}"),
    };
    assert!(reason.contains("no database"), "{reason}");
}

// ---------------------------------------------------------------------------
// sidecar persistence
// ---------------------------------------------------------------------------

#[test]
fn the_sidecar_persists_forms() {
    let mut form = FormDef::new("Card");
    form.controls.push(datacraft_core::form::ControlDef {
        name: "title".into(),
        kind: datacraft_core::form::ControlKind::Label {
            text: "Customers".into(),
        },
        rect: Rect {
            x: 16.0,
            y: 16.0,
            width: 120.0,
            height: 24.0,
        },
    });

    let mut doc = SchemaDocument::new("shop");
    doc.forms.push(form.clone());
    let back = SchemaDocument::from_json(&doc.to_json()).unwrap();
    assert_eq!(back.forms, vec![form]);
}
