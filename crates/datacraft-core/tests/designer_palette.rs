//! Designer palettes and record-source selection (step 11.6).
//!
//! The toolbox mirrors the Access control palette, the field list maps a
//! source table's fields onto bound controls, and placement flows into the
//! next free grid slot. All mapping and layout math lives in core so the
//! design views stay pure paint (ARCHITECTURE §8); the record source stays
//! schema-whitelisted (LESSONS #5).

use std::path::PathBuf;

use datacraft_core::canvas::Rect;
use datacraft_core::command::Command;
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::form::{ControlKind, FormDef};
use datacraft_core::palette::{
    TOOLBOX, field_palette, free_slot, toolbox_control, unique_control_name,
};
use datacraft_core::port::MockPort;
use datacraft_core::report::ReportDef;
use datacraft_core::schema::{
    AutoKind, BinaryKind, FieldDef, FieldKind, NumericKind, TableDef,
};

fn customers() -> TableDef {
    TableDef::new(
        "Customers",
        vec![
            FieldDef::new("id", FieldKind::Number(NumericKind::LongInteger)),
            FieldDef::new("name", FieldKind::Text { max_len: 50 }),
            FieldDef::new("active", FieldKind::Boolean { tri_state: false }),
            FieldDef::new(
                "photo",
                FieldKind::Binary(BinaryKind::OleObject),
            ),
        ],
    )
}

fn opened_core() -> Core {
    let mut core = Core::new(Box::new(MockPort::new()));
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
    });
    core.dispatch(Command::CreateTable {
        definition: customers(),
    });
    core
}

fn opened_form() -> Core {
    let mut core = opened_core();
    core.dispatch(Command::CreateForm {
        name: "Card".into(),
    });
    core
}

fn opened_report() -> Core {
    let mut core = opened_core();
    core.dispatch(Command::CreateReport {
        name: "List".into(),
    });
    core
}

fn form(core: &Core) -> FormDef {
    core.inspect().database.unwrap().forms[0].clone()
}

fn report(core: &Core) -> ReportDef {
    core.inspect().database.unwrap().reports[0].clone()
}

fn failed_reason(events: &[Event]) -> String {
    match events {
        [Event::Failed { reason }] => reason.clone(),
        other => panic!("expected single Failed event, got {other:?}"),
    }
}

fn overlaps(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.width && b.x < a.x + a.width && a.y < b.y + b.height && b.y < a.y + a.height
}

#[test]
fn the_toolbox_mirrors_the_access_control_palette() {
    let ids: Vec<&str> = TOOLBOX.iter().map(|entry| entry.id).collect();
    assert_eq!(
        ids,
        vec!["label", "text_box", "check_box", "combo_box", "button", "subform"]
    );

    let rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 100.0,
        height: 24.0,
    };
    let kinds: Vec<ControlKind> = TOOLBOX
        .iter()
        .map(|entry| toolbox_control(entry.id, "ctl", rect).expect("known entry").kind)
        .collect();
    assert_eq!(
        kinds,
        vec![
            ControlKind::Label {
                text: "Label".into()
            },
            ControlKind::TextBox { bound_field: None },
            ControlKind::CheckBox { bound_field: None },
            ControlKind::ComboBox { bound_field: None },
            ControlKind::Button {
                caption: "Button".into()
            },
            ControlKind::SubForm { form: String::new() },
        ]
    );
    assert!(toolbox_control("ghost", "ctl", rect).is_none());
}

#[test]
fn the_field_list_maps_kinds_onto_designer_controls() {
    let entries = field_palette(&customers());
    let by_name = |name: &str| {
        entries
            .iter()
            .find(|entry| entry.name == name)
            .expect("field listed")
            .clone()
    };

    assert_eq!(entries.len(), 4);
    assert_eq!(by_name("id").kind_display, "Long Integer");
    assert!(by_name("id").bindable);
    assert_eq!(by_name("name").kind_display, "Short Text");
    assert_eq!(by_name("active").kind_display, "Yes/No");
    assert_eq!(by_name("photo").kind_display, "OLE Object");
    // No toolbox control exists for OLE/attachment columns: the field is
    // listed but not addable (precise refusal instead of a wrong control).
    assert!(!by_name("photo").bindable);
}

#[test]
fn field_kind_mapping_covers_every_domain_kind() {
    use datacraft_core::palette::field_control_kind;
    let cases: &[(FieldKind, bool)] = &[
        (FieldKind::Text { max_len: 10 }, true),
        (FieldKind::Memo, true),
        (FieldKind::Number(NumericKind::LongInteger), true),
        (FieldKind::DateTime, true),
        (FieldKind::Currency, true),
        (FieldKind::AutoNumber(AutoKind::LongInteger), true),
        (FieldKind::Boolean { tri_state: true }, true),
        (FieldKind::Guid, true),
        (FieldKind::Binary(BinaryKind::OleObject), false),
        (FieldKind::Binary(BinaryKind::Attachment), false),
    ];
    for (kind, bindable) in cases {
        let control = field_control_kind(kind);
        assert_eq!(control.is_some(), *bindable, "{kind:?}");
        if let Some(ControlKind::CheckBox { .. }) = control {
            assert!(matches!(kind, FieldKind::Boolean { .. }), "{kind:?}");
        }
    }
}

#[test]
fn control_names_stay_unique_case_insensitively() {
    assert_eq!(unique_control_name("name", &[]), "name");
    assert_eq!(unique_control_name("name", &["NAME".into()]), "name2");
    assert_eq!(
        unique_control_name("name", &["name".into(), "Name2".into()]),
        "name3"
    );
}

#[test]
fn free_slots_flow_right_of_occupied_controls() {
    let occupied = vec![Rect {
        x: 0.0,
        y: 0.0,
        width: 120.0,
        height: 24.0,
    }];
    let slot = free_slot(8.0, (120.0, 24.0), &occupied, (640.0, 400.0)).expect("free slot");
    assert_eq!(
        slot,
        Rect {
            x: 128.0,
            y: 0.0,
            width: 120.0,
            height: 24.0
        }
    );
    assert!(!overlaps(slot, occupied[0]));
}

#[test]
fn a_full_canvas_has_no_free_slot() {
    let occupied = vec![Rect {
        x: 0.0,
        y: 0.0,
        width: 120.0,
        height: 24.0,
    }];
    assert!(free_slot(8.0, (120.0, 24.0), &occupied, (120.0, 24.0)).is_none());
}

#[test]
fn adding_a_field_requires_a_record_source() {
    let mut core = opened_form();
    let reason = failed_reason(&core.dispatch(Command::AddFormField {
        form: "Card".into(),
        field: "name".into(),
    }));
    assert!(reason.contains("source"), "{reason}");
}

#[test]
fn adding_a_field_places_a_bound_control_in_the_next_free_slot() {
    let mut core = opened_form();
    core.dispatch(Command::SetFormSource {
        form: "Card".into(),
        table: "Customers".into(),
    });
    assert_eq!(
        core.dispatch(Command::AddFormField {
            form: "Card".into(),
            field: "name".into(),
        }),
        vec![Event::ControlAdded {
            form: "Card".into(),
            control: "name".into(),
        }]
    );
    let def = form(&core);
    let placed = &def.controls[0];
    assert_eq!(
        placed.kind,
        ControlKind::TextBox {
            bound_field: Some("name".into())
        }
    );
    assert_eq!(
        placed.rect,
        Rect {
            x: 0.0,
            y: 0.0,
            width: 120.0,
            height: 24.0
        }
    );

    // A second field lands beside it; a repeated field gets a unique name.
    core.dispatch(Command::AddFormField {
        form: "Card".into(),
        field: "active".into(),
    });
    let events = core.dispatch(Command::AddFormField {
        form: "Card".into(),
        field: "NAME".into(),
    });
    assert_eq!(
        events,
        vec![Event::ControlAdded {
            form: "Card".into(),
            control: "name2".into(),
        }]
    );
    let def = form(&core);
    assert_eq!(def.controls.len(), 3);
    assert!(!overlaps(def.controls[0].rect, def.controls[1].rect));
    assert!(!overlaps(def.controls[0].rect, def.controls[2].rect));
    assert!(!overlaps(def.controls[1].rect, def.controls[2].rect));
    assert_eq!(
        def.controls[2].kind,
        ControlKind::TextBox {
            bound_field: Some("name".into())
        }
    );
    assert_eq!(def.controls[1].kind, ControlKind::CheckBox { bound_field: Some("active".into()) });
}

#[test]
fn adding_an_unknown_field_is_rejected_with_its_name() {
    let mut core = opened_form();
    core.dispatch(Command::SetFormSource {
        form: "Card".into(),
        table: "Customers".into(),
    });
    let reason = failed_reason(&core.dispatch(Command::AddFormField {
        form: "Card".into(),
        field: "ghost".into(),
    }));
    assert!(reason.contains("ghost"), "{reason}");
}

#[test]
fn binary_fields_have_no_designer_control() {
    let mut core = opened_form();
    core.dispatch(Command::SetFormSource {
        form: "Card".into(),
        table: "Customers".into(),
    });
    let reason = failed_reason(&core.dispatch(Command::AddFormField {
        form: "Card".into(),
        field: "photo".into(),
    }));
    assert!(reason.contains("no designer control"), "{reason}");
    assert!(reason.contains("OLE Object"), "{reason}");
}

#[test]
fn report_fields_place_inside_the_page() {
    let mut core = opened_report();
    let reason = failed_reason(&core.dispatch(Command::AddReportField {
        report: "List".into(),
        field: "name".into(),
    }));
    assert!(reason.contains("source"), "{reason}");

    core.dispatch(Command::SetReportSource {
        report: "List".into(),
        table: "Customers".into(),
    });
    assert_eq!(
        core.dispatch(Command::AddReportField {
            report: "List".into(),
            field: "name".into(),
        }),
        vec![Event::ReportControlAdded {
            report: "List".into(),
            control: "name".into(),
        }]
    );
    let def = report(&core);
    let placed = &def.controls[0];
    assert!(placed.rect.x + placed.rect.width <= def.page.width);
    assert!(placed.rect.y + placed.rect.height <= def.page.height);
    assert_eq!(
        placed.kind,
        ControlKind::TextBox {
            bound_field: Some("name".into())
        }
    );
}

#[test]
fn field_palette_commands_round_trip_through_json() {
    for command in [
        Command::AddFormField {
            form: "Card".into(),
            field: "name".into(),
        },
        Command::AddReportField {
            report: "List".into(),
            field: "name".into(),
        },
    ] {
        let json = serde_json::to_string(&command).expect("serialize command");
        let back: Command = serde_json::from_str(&json).expect("deserialize command");
        assert_eq!(back, command);
    }
}
