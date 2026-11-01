//! DataCraft desktop shell (eframe/egui).
//!
//! Phase 5 skeleton: the four-region shell (ribbon, navigation pane,
//! tabbed workspace, status bar) driven by the headless core
//! (`docs/ARCHITECTURE.md` §5). The ribbon grows in step 5.2, the
//! navigation pane in 5.3 and the workspace in 5.4.
//!
//! Coverage policy: this crate is rendering glue only — logic lives in
//! `datacraft-core`; the crate is excluded from the coverage gate
//! (exception registry in `docs/TESTING.md`).

mod app;
mod datasheet;
mod design;
mod designer_palette;
mod flows;
mod form_design;
mod form_runtime;
mod nav_pane;
mod qbe;
mod relationships;
mod report_design;
mod ribbon;
mod sql_window;
mod theme;
mod workspace;

use eframe::egui;

use crate::app::App;

/// Embedded 64 px brand render (assets/app-icon, Crafting Apps office-symbol
/// grammar — see `assets/app-icon/README.md`).
const ICON_PNG: &[u8] = include_bytes!("../../../assets/app-icon/datacraft-64.png");

/// Decodes the embedded brand icon into an egui window icon.
/// Returns `None` on decode failure — the window simply starts without an
/// icon instead of refusing to start.
fn window_icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(ICON_PNG).ok()?.to_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        width,
        height,
        rgba: img.into_raw(),
    })
}

fn main() -> eframe::Result {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1280.0, 800.0])
        .with_min_inner_size([960.0, 600.0]);
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "DataCraft",
        options,
        Box::new(|_cc| {
            Ok(Box::new(App::new(Box::new(
                datacraft_storage::detecting::DetectingPort::new(),
            ))))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_icon_decodes_to_square_rgba() {
        let icon = window_icon().expect("embedded brand png must decode");
        assert_eq!(icon.width, 64);
        assert_eq!(icon.height, 64);
        assert_eq!(icon.rgba.len(), 64 * 64 * 4);
    }

    #[test]
    fn app_starts_with_a_closed_visible_shell() {
        let app = App::new(Box::new(datacraft_core::port::MockPort::new()));
        assert!(app.snapshot().database.is_none());
        assert!(app.snapshot().nav_pane_visible);
        assert!(app.snapshot().tabs.is_empty());
    }

    #[test]
    fn app_runs_sql_and_caches_the_result() {
        let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
        app.dispatch(datacraft_core::command::Command::OpenDatabase {
            path: "memory.mock".into(),
        });
        app.set_sql_text("SELECT 1 AS one");
        app.run_sql();
        assert!(app.sql_error().is_none(), "{:?}", app.sql_error());
        let result = app.sql_result().expect("result is cached");
        assert_eq!(
            result.rows[0][0],
            datacraft_core::schema::FieldValue::Integer(1)
        );
    }

    #[test]
    fn app_caches_sql_errors_precisely() {
        let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
        app.dispatch(datacraft_core::command::Command::OpenDatabase {
            path: "memory.mock".into(),
        });
        app.set_sql_text("DELETE FROM nothing");
        app.run_sql();
        assert!(app.sql_result().is_none());
        assert!(
            app.sql_error().unwrap_or_default().contains("only SELECT"),
            "{:?}",
            app.sql_error()
        );
    }

    #[test]
    fn the_new_table_flow_creates_and_opens_the_default_table() {
        let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
        app.dispatch(datacraft_core::command::Command::OpenDatabase {
            path: "memory.mock".into(),
        });
        app.create_table_flow();
        let snapshot = app.snapshot();
        let tables = &snapshot.database.as_ref().expect("database").tables;
        assert_eq!(tables[0].name, "Table1");
        assert_eq!(tables[0].fields.len(), 2);
        assert!(
            snapshot
                .tabs
                .iter()
                .any(|tab| tab.object_kind == "table" && tab.object_name == "Table1"),
            "{:?}",
            snapshot.tabs
        );
        assert!(app.last_error().is_none(), "{:?}", app.last_error());
    }

    #[test]
    fn delete_flows_dispatch_and_surface_precise_errors() {
        use datacraft_core::ObjectKind;
        let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
        app.dispatch(datacraft_core::command::Command::OpenDatabase {
            path: "memory.mock".into(),
        });
        app.dispatch(datacraft_core::command::Command::CreateForm {
            name: "Card".into(),
        });
        app.delete_object(ObjectKind::Form, "Card");
        assert!(
            app.snapshot()
                .database
                .as_ref()
                .expect("database")
                .forms
                .is_empty()
        );
        app.delete_object(ObjectKind::Form, "Missing");
        assert!(
            app.last_error().unwrap_or_default().contains("not found"),
            "{:?}",
            app.last_error()
        );
    }

    #[test]
    fn the_designer_palette_places_fields_and_toolbox_controls() {
        use datacraft_core::ObjectKind;
        use datacraft_core::command::Command;
        use datacraft_core::form::ControlKind;
        use datacraft_core::schema::{FieldDef, FieldKind, TableDef};

        let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
        app.dispatch(Command::OpenDatabase {
            path: "memory.mock".into(),
        });
        app.dispatch(Command::CreateTable {
            definition: TableDef::new(
                "Customers",
                vec![
                    FieldDef::new("name", FieldKind::Text { max_len: 50 }),
                    FieldDef::new("active", FieldKind::Boolean { tri_state: false }),
                ],
            ),
        });
        app.dispatch(Command::CreateForm {
            name: "Card".into(),
        });

        app.set_record_source(ObjectKind::Form, "Card", "Customers");
        app.add_designer_field(ObjectKind::Form, "Card", "name");
        app.add_toolbox_control(ObjectKind::Form, "Card", "button");
        let form = app.snapshot().database.as_ref().expect("db").forms[0].clone();
        assert_eq!(form.controls.len(), 2);
        assert_eq!(
            form.controls[0].kind,
            ControlKind::TextBox {
                bound_field: Some("name".into())
            }
        );
        assert_eq!(
            form.controls[1].kind,
            ControlKind::Button {
                caption: "Button".into()
            }
        );
        assert!(app.last_error().is_none(), "{:?}", app.last_error());

        app.dispatch(Command::CreateReport {
            name: "List".into(),
        });
        app.set_record_source(ObjectKind::Report, "List", "Customers");
        app.add_designer_field(ObjectKind::Report, "List", "active");
        let report = app.snapshot().database.as_ref().expect("db").reports[0].clone();
        assert_eq!(
            report.controls[0].kind,
            ControlKind::CheckBox {
                bound_field: Some("active".into())
            }
        );

        // The palette refuses a source outside the schema whitelist.
        app.set_record_source(ObjectKind::Form, "Card", "Ghost");
        assert!(
            app.last_error().unwrap_or_default().contains("Ghost"),
            "{:?}",
            app.last_error()
        );
    }

    #[test]
    fn dispatch_refreshes_the_snapshot() {
        let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
        app.dispatch(datacraft_core::command::Command::ToggleNavPane);
        assert!(!app.snapshot().nav_pane_visible);
        app.dispatch(datacraft_core::command::Command::ToggleNavPane);
        assert!(app.snapshot().nav_pane_visible);
    }
}
