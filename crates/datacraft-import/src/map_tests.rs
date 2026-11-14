//! Unit pins for the mapping rules (step 78.1, item M88).

use super::*;

fn prop(name: &str, value: RawValue) -> RawProp {
    RawProp {
        name: Some(name.to_owned()),
        value,
    }
}

fn text(name: &str, value: &str) -> RawProp {
    prop(name, RawValue::Text(value.to_owned()))
}

fn number(name: &str, value: i64) -> RawProp {
    prop(name, RawValue::Number { value, short: true })
}

fn control(type_name: &str, name: &str, props: Vec<RawProp>) -> RawControl {
    RawControl {
        name: name.to_owned(),
        type_name: Some(type_name.to_owned()),
        type_label: type_name.to_owned(),
        props,
    }
}

fn unknown_control(name: &str, label: &str) -> RawControl {
    RawControl {
        name: name.to_owned(),
        type_name: None,
        type_label: label.to_owned(),
        props: Vec::new(),
    }
}

fn form_object(props: Vec<RawProp>, controls: Vec<RawControl>) -> RawObject {
    RawObject {
        name: "F".to_owned(),
        is_form: true,
        props,
        controls,
    }
}

fn mapped_form(object: &RawObject) -> FormDef {
    let (design, report) = map_object(object, &[], &[]);
    assert!(
        report.unsupported_controls.is_empty(),
        "{:?}",
        report.unsupported_controls
    );
    match design {
        Some(Design::Form(form)) => form,
        other => panic!("expected a form, got {other:?}"),
    }
}

#[test]
fn defaults_fill_the_confirmed_access_values() {
    let object = form_object(
        vec![number("Width", 8884)],
        vec![
            control(
                "Detail",
                "Detail",
                vec![
                    prop("Name", RawValue::Text("Detail".into())),
                    number("Height", 1500),
                ],
            ),
            control(
                "TextBox",
                "txtName",
                vec![
                    prop("Name", RawValue::Text("txtName".into())),
                    number("Top", 300),
                    number("Height", 315),
                ],
            ),
        ],
    );
    let form = mapped_form(&object);
    assert_eq!(form.size.width, 444.2);
    assert_eq!(form.size.height, 75.0);
    let rect = form.controls[0].rect;
    assert_eq!((rect.x, rect.y), (0.0, 15.0), "Left defaults to 0");
    assert_eq!(
        (rect.width, rect.height),
        (72.0, 15.75),
        "TextBox Width defaults to 1440 twips"
    );
}

#[test]
fn unsigned_reinterprets_two_byte_values() {
    assert_eq!(
        unsigned(&prop(
            "Right",
            RawValue::Number {
                value: -15_536,
                short: true
            }
        )),
        Some(50_000)
    );
    assert_eq!(
        unsigned(&prop(
            "Long",
            RawValue::Number {
                value: -1,
                short: false
            }
        )),
        None
    );
    assert_eq!(
        unsigned(&prop(
            "Long",
            RawValue::Number {
                value: 70_000,
                short: false
            }
        )),
        None
    );
    assert_eq!(unsigned(&text("Name", "x")), None);
}

#[test]
fn unknown_and_unmapped_controls_are_reported() {
    let object = form_object(
        Vec::new(),
        vec![
            unknown_control("List1", "0x1234"),
            control("ListBox", "List2", Vec::new()),
        ],
    );
    let (_, report) = map_object(&object, &[], &[]);
    let labels: Vec<&str> = report
        .unsupported_controls
        .iter()
        .map(|entry| entry.control_type.as_str())
        .collect();
    assert_eq!(labels, ["0x1234", "ListBox"]);
    assert_eq!(report.imported_controls, 0);
}

#[test]
fn missing_height_is_reported_and_the_control_is_skipped() {
    let object = form_object(
        Vec::new(),
        vec![control(
            "TextBox",
            "txtName",
            vec![
                prop("Name", RawValue::Text("txtName".into())),
                number("Left", 100),
                number("Top", 100),
                number("Width", 1440),
            ],
        )],
    );
    let (design, report) = map_object(&object, &[], &[]);
    assert!(report.unsupported_properties.iter().any(|entry| {
        entry.control.as_deref() == Some("txtName") && entry.property == "Height"
    }));
    assert_eq!(report.imported_controls, 0);
    let Some(Design::Form(form)) = design else {
        panic!("expected a form");
    };
    assert!(form.controls.is_empty());
}

#[test]
fn calculated_sources_are_unbound_and_reported() {
    let object = form_object(
        Vec::new(),
        vec![control(
            "TextBox",
            "txtTotal",
            vec![
                prop("Name", RawValue::Text("txtTotal".into())),
                number("Left", 0),
                number("Top", 0),
                number("Width", 1440),
                number("Height", 315),
                text("ControlSource", "=[Price]*[Qty]"),
            ],
        )],
    );
    let form = mapped_form(&object);
    assert_eq!(
        form.controls[0].kind,
        ControlKind::TextBox { bound_field: None }
    );
    let (_, report) = map_object(&object, &[], &[]);
    assert!(
        report
            .unsupported_properties
            .iter()
            .any(|entry| entry.property == "ControlSource")
    );
}

#[test]
fn combo_row_sources_are_reported_as_a_limitation() {
    let object = form_object(
        Vec::new(),
        vec![control(
            "ComboBox",
            "cboCategory",
            vec![
                prop("Name", RawValue::Text("cboCategory".into())),
                number("Left", 0),
                number("Top", 0),
                number("Width", 1440),
                number("Height", 315),
                text("ControlSource", "CategoryId"),
                text("RowSource", "SELECT Id, Name FROM Categories"),
            ],
        )],
    );
    let form = mapped_form(&object);
    assert_eq!(
        form.controls[0].kind,
        ControlKind::ComboBox {
            bound_field: Some("CategoryId".to_owned()),
            row_source: String::new(),
            display_field: String::new(),
        }
    );
    let (_, report) = map_object(&object, &[], &[]);
    assert!(
        report
            .unsupported_properties
            .iter()
            .any(|entry| entry.property == "RowSource")
    );
}

#[test]
fn subform_sources_are_mapped_or_reported() {
    let object = form_object(
        Vec::new(),
        vec![
            control(
                "SubReport",
                "subDetails",
                vec![
                    prop("Name", RawValue::Text("subDetails".into())),
                    number("Left", 0),
                    number("Top", 0),
                    number("Width", 1440),
                    number("Height", 315),
                    text("SourceObject", "Form.Details"),
                ],
            ),
            control(
                "SubReport",
                "subBroken",
                vec![
                    prop("Name", RawValue::Text("subBroken".into())),
                    number("Left", 0),
                    number("Top", 0),
                    number("Width", 1440),
                    number("Height", 315),
                    text("SourceObject", "SELECT * FROM Details"),
                ],
            ),
        ],
    );
    let form = mapped_form(&object);
    assert_eq!(
        form.controls[0].kind,
        ControlKind::SubForm {
            form: "Details".to_owned()
        }
    );
    assert_eq!(form.controls.len(), 1, "the expression source is skipped");
    let (_, report) = map_object(&object, &[], &[]);
    assert!(
        report
            .unsupported_properties
            .iter()
            .any(|entry| entry.property == "SourceObject")
    );
}

#[test]
fn the_record_source_must_be_a_known_plain_name() {
    let object = form_object(vec![text("RecordSource", "Items In Stock")], Vec::new());
    let queries = vec!["Items In Stock".to_owned()];
    let (design, report) = map_object(&object, &[], &queries);
    let Some(Design::Form(form)) = design else {
        panic!("expected a form");
    };
    assert_eq!(form.source, "Items In Stock");
    assert!(report.unsupported_properties.is_empty());

    let object = form_object(
        vec![text("RecordSource", "SELECT * FROM Items")],
        Vec::new(),
    );
    let (design, report) = map_object(&object, &[], &[]);
    let Some(Design::Form(form)) = design else {
        panic!("expected a form");
    };
    assert!(form.source.is_empty());
    assert!(
        report
            .unsupported_properties
            .iter()
            .any(|entry| entry.property == "RecordSource")
    );
}

#[test]
fn form_size_falls_back_to_the_control_extents() {
    let object = form_object(
        Vec::new(),
        vec![control(
            "Label",
            "lbl",
            vec![
                prop("Name", RawValue::Text("lbl".into())),
                number("Left", 100),
                number("Top", 30),
                number("Width", 40),
                number("Height", 10),
            ],
        )],
    );
    let form = mapped_form(&object);
    assert_eq!(form.size.width, 7.0);
    assert_eq!(form.size.height, 2.0);
}

#[test]
fn zero_extent_controls_are_reported() {
    let object = form_object(
        Vec::new(),
        vec![control(
            "Label",
            "lbl",
            vec![
                prop("Name", RawValue::Text("lbl".into())),
                number("Left", 0),
                number("Top", 0),
                number("Width", 0),
                number("Height", 10),
            ],
        )],
    );
    let (_, report) = map_object(&object, &[], &[]);
    assert!(
        report
            .unsupported_properties
            .iter()
            .any(|entry| entry.property == "Width/Height")
    );
}

#[test]
fn ignored_properties_are_collected_sorted_and_unique() {
    let object = form_object(
        vec![text("Caption", "F"), text("FontName", "Arial")],
        vec![control(
            "Label",
            "lbl",
            vec![
                prop("Name", RawValue::Text("lbl".into())),
                number("Left", 0),
                number("Top", 0),
                number("Width", 1440),
                number("Height", 315),
                text("FontName", "Arial"),
                number("TabIndex", 1),
            ],
        )],
    );
    let (_, report) = map_object(&object, &[], &[]);
    assert_eq!(
        report.ignored_properties,
        ["Caption", "FontName", "TabIndex"]
    );
}

#[test]
fn reports_get_the_default_page_and_grid() {
    let object = RawObject {
        name: "R".to_owned(),
        is_form: false,
        props: Vec::new(),
        controls: vec![control(
            "Label",
            "lbl",
            vec![
                prop("Name", RawValue::Text("lbl".into())),
                number("Left", 0),
                number("Top", 0),
                number("Width", 1440),
                number("Height", 315),
                text("Caption", "Title"),
            ],
        )],
    };
    let (design, _) = map_object(&object, &[], &[]);
    match design {
        Some(Design::Report(report)) => {
            assert_eq!(report.page, PageSetup::default());
            assert_eq!(report.grid, 8.0);
            assert_eq!(
                report.controls[0].kind,
                ControlKind::Label {
                    text: "Title".to_owned()
                }
            );
        }
        other => panic!("expected a report, got {other:?}"),
    }
}
