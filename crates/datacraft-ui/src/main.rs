//! DataCraft desktop shell (eframe/egui).
//!
//! The four-region shell (ribbon, navigation pane, tabbed workspace,
//! status bar) driven by the headless core (`docs/ARCHITECTURE.md` §5):
//! datasheet with the new-record row, read-only design grid, QBE grid,
//! SQL window, form/report designers with palettes, relationship mapper
//! and the sidecar actions.
//!
//! Coverage policy: this crate is rendering glue only — logic lives in
//! `datacraft-core`; the crate is excluded from the coverage gate
//! (exception registry in `docs/TESTING.md`).

mod app;
mod appearance;
mod backstage;
mod datasheet;
mod design;
mod designer_palette;
mod flows;
mod form_design;
mod form_runtime;
mod i18n;
mod icons;
mod labels;
mod macro_viewer;
mod nav_pane;
mod password_prompt;
mod qbe;
mod relationships;
mod report_design;
mod ribbon;
mod settings;
mod shell;
mod sql_window;
mod theme;
mod vba_viewer;
mod window;
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
    let settings_path = crate::settings::default_path();
    let settings = settings_path
        .as_deref()
        .map(crate::settings::Settings::load)
        .unwrap_or_default();

    let mut viewport = crate::window::viewport(settings.window);
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
        Box::new(move |cc| {
            crate::theme::fonts::install(&cc.egui_ctx);
            let mut app = App::new(Box::new(datacraft_storage::detecting::DetectingPort::new()));
            app.set_settings_path(settings_path);
            app.apply_settings(settings);
            Ok(Box::new(app))
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
            password: None,
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
            password: None,
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
            password: None,
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
            password: None,
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
    fn opening_a_database_prefills_the_sidecar_path_and_saving_reports_a_notice() {
        use datacraft_core::command::Command;

        // The status notices are UI text: pin the language while asserting.
        let _guard = crate::i18n::lang_lock();
        crate::i18n::set_lang(crate::i18n::Lang::English);

        let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
        app.dispatch(Command::OpenDatabase {
            path: "shop.mdb".into(),
            password: None,
        });
        assert_eq!(app.sidecar_path, "shop.datacraft");

        let dir = std::env::temp_dir().join(format!("datacraft-sidecar-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("shop.datacraft");
        app.sidecar_path = path.to_string_lossy().into_owned();
        app.dispatch(Command::CreateForm {
            name: "Card".into(),
        });
        app.dispatch(Command::SaveDesignDocument { path: path.clone() });
        assert!(app.last_error().is_none(), "{:?}", app.last_error());
        assert!(path.exists(), "sidecar written");
        assert!(
            app.last_notice()
                .unwrap_or_default()
                .contains("Design saved"),
            "{:?}",
            app.last_notice()
        );

        app.dispatch(Command::DeleteForm {
            name: "Card".into(),
        });
        app.dispatch(Command::LoadDesignDocument { path: path.clone() });
        assert!(
            app.last_notice()
                .unwrap_or_default()
                .contains("Design loaded"),
            "{:?}",
            app.last_notice()
        );
        assert_eq!(app.forms().len(), 1);

        // A failure clears the previous success notice.
        app.dispatch(Command::LoadDesignDocument {
            path: dir.join("missing.datacraft"),
        });
        assert!(app.last_error().is_some());
        assert!(app.last_notice().is_none(), "{:?}", app.last_notice());
        let _ = std::fs::remove_dir_all(&dir);
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
            password: None,
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
    fn the_datasheet_new_row_inserts_through_the_bus() {
        use datacraft_core::ObjectId;
        use datacraft_core::ObjectKind;
        use datacraft_core::command::Command;
        use datacraft_core::schema::{
            AutoKind, FieldDef, FieldKind, FieldValue, NumericKind, TableDef,
        };

        let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
        app.dispatch(Command::OpenDatabase {
            path: "memory.mock".into(),
            password: None,
        });
        app.dispatch(Command::CreateTable {
            definition: TableDef::new(
                "Customers",
                vec![
                    FieldDef::new("id", FieldKind::AutoNumber(AutoKind::LongInteger)),
                    FieldDef::new("name", FieldKind::Text { max_len: 20 }),
                    FieldDef::new("score", FieldKind::Number(NumericKind::Double)),
                ],
            ),
        });
        app.dispatch(Command::OpenObject(ObjectId::new(
            ObjectKind::Table,
            "Customers",
        )));
        let tab = app.snapshot().tabs[0].id;

        // The buffer pads to the column count.
        assert_eq!(app.new_row_cells(tab, 3), vec!["", "", ""]);
        app.set_new_row_cells(tab, vec![String::new(), "Ada".into(), "1.5".into()]);
        assert!(app.insert_new_row(tab, "Customers"));
        let rows = app.rows_for(tab).expect("rows reloaded");
        assert_eq!(rows[0][1], FieldValue::Text("Ada".into()));
        assert_eq!(rows[0][2], FieldValue::Float(1.5));
        assert_eq!(app.new_row_cells(tab, 3), vec!["", "", ""]);

        // A mistyped cell names the field instead of inserting.
        app.set_new_row_cells(tab, vec![String::new(), "Grace".into(), "abc".into()]);
        assert!(!app.insert_new_row(tab, "Customers"));
        assert!(
            app.last_error().unwrap_or_default().contains("\"score\""),
            "{:?}",
            app.last_error()
        );
        assert_eq!(app.rows_for(tab).map(Vec::len), Some(1));
    }

    #[test]
    fn dispatch_refreshes_the_snapshot() {
        let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
        app.dispatch(datacraft_core::command::Command::ToggleNavPane);
        assert!(!app.snapshot().nav_pane_visible);
        app.dispatch(datacraft_core::command::Command::ToggleNavPane);
        assert!(app.snapshot().nav_pane_visible);
    }

    #[test]
    fn a_password_required_open_raises_the_prompt_and_retries() {
        use datacraft_core::command::Command;
        let mut port = datacraft_core::port::MockPort::new();
        port.with_password("secret");
        let mut app = App::new(Box::new(port));
        app.dispatch(Command::OpenDatabase {
            path: "shop.mdb".into(),
            password: None,
        });
        assert!(app.password_prompt().is_some(), "prompt is raised");
        app.set_password_input("wrong");
        app.submit_password();
        assert!(
            app.password_prompt()
                .and_then(|prompt| prompt.error)
                .is_some(),
            "the wrong password keeps the prompt open"
        );
        app.set_password_input("secret");
        app.submit_password();
        assert!(app.password_prompt().is_none(), "the prompt closes");
        assert!(app.snapshot().database.is_some());
    }

    #[test]
    fn startup_settings_apply_the_language_and_survive_a_restart() {
        use crate::settings::{LanguageSetting, Settings};

        let _guard = crate::i18n::lang_lock();
        let dir =
            std::env::temp_dir().join(format!("datacraft-startup-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("ui.json");

        Settings {
            language: LanguageSetting::German,
            ..Settings::default()
        }
        .save(&path)
        .expect("save");

        let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
        app.set_settings_path(Some(path.clone()));
        app.apply_settings(Settings::load(&path));
        assert_eq!(app.language_setting(), LanguageSetting::German);
        assert_eq!(crate::i18n::t("File"), "Datei");

        crate::i18n::set_lang(crate::i18n::Lang::English);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn switching_the_language_applies_live_and_persists_the_choice() {
        use crate::settings::LanguageSetting;

        let _guard = crate::i18n::lang_lock();
        let dir =
            std::env::temp_dir().join(format!("datacraft-language-switch-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join("ui.json");

        let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
        app.set_settings_path(Some(path.clone()));
        app.set_language_setting(LanguageSetting::German);
        assert_eq!(app.language_setting(), LanguageSetting::German);
        assert_eq!(crate::i18n::t("File"), "Datei");
        assert!(app.last_error().is_none(), "{:?}", app.last_error());

        // The next start reads the same choice (acceptance: restart).
        let loaded = crate::settings::Settings::load(&path);
        let mut restarted = App::new(Box::new(datacraft_core::port::MockPort::new()));
        restarted.apply_settings(loaded);
        assert_eq!(restarted.language_setting(), LanguageSetting::German);
        assert_eq!(crate::i18n::t("File"), "Datei");

        restarted.set_language_setting(LanguageSetting::English);
        assert_eq!(crate::i18n::t("File"), "File");
        crate::i18n::set_lang(crate::i18n::Lang::English);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_failed_settings_write_surfaces_a_precise_reason() {
        use crate::settings::LanguageSetting;

        let _guard = crate::i18n::lang_lock();
        crate::i18n::set_lang(crate::i18n::Lang::English);
        let dir =
            std::env::temp_dir().join(format!("datacraft-settings-fail-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");

        let mut app = App::new(Box::new(datacraft_core::port::MockPort::new()));
        // A directory path cannot be written as a file. Keep the language
        // English so the surfaced message itself is deterministic.
        app.set_settings_path(Some(dir.clone()));
        app.set_language_setting(LanguageSetting::English);
        assert!(
            app.last_error()
                .unwrap_or_default()
                .contains("could not be saved"),
            "{:?}",
            app.last_error()
        );

        crate::i18n::set_lang(crate::i18n::Lang::English);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
