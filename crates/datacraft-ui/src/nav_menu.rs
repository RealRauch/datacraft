//! Nav-row context menu (step 21.5, item U9; the selection's collective
//! action step 92.4, item L48; split from `nav_pane.rs` for the size
//! gate, item L30).
//!
//! A row that is part of a multi-selection offers the collective action
//! ("Open {count} selected") instead of the single-object entries; every
//! other row keeps the Open / Design / Rename / Delete menu with its
//! capability gating.

use eframe::egui;

use datacraft_core::{ObjectId, ObjectKind};

use crate::i18n::{t, tr};

/// The intents one row's context menu collected.
#[derive(Debug, Default)]
pub(crate) struct RowMenuIntent {
    /// Open the row as a datasheet.
    pub open: bool,
    /// Open the row in Design mode (only offered for design-capable kinds).
    pub design: bool,
    /// Open the rename dialog.
    pub rename: bool,
    /// Delete the object.
    pub delete: bool,
    /// The collective action: open every selected object.
    pub open_selection: Option<Vec<ObjectId>>,
    /// The collective action: create a local group from the selection
    /// (step 92.5, item L48).
    pub new_group: Option<Vec<ObjectId>>,
}

/// Renders the row's context menu and returns the collected intents.
/// `selection` carries the active multi-selection when the row is one of
/// its members.
pub(crate) fn row_menu(
    app: &crate::app::App,
    ui: &mut egui::Ui,
    header: &'static str,
    name: &str,
    kind: ObjectKind,
    selection: Option<&[ObjectId]>,
) -> RowMenuIntent {
    let mut intent = RowMenuIntent::default();
    // The selection's collective action (step 92.4, item L48): a member
    // of a multi-selection offers opening every picked object instead of
    // the single-object menu.
    if let Some(selection) = selection {
        let count = selection.len();
        if ui
            .button(tr(
                "Open {count} selected",
                &[("count", &count.to_string())],
            ))
            .clicked()
        {
            intent.open_selection = Some(selection.to_vec());
            ui.close();
        }
        if ui
            .button(tr(
                "New group from {count} selected",
                &[("count", &count.to_string())],
            ))
            .clicked()
        {
            intent.new_group = Some(selection.to_vec());
            ui.close();
        }
        ui.separator();
        return intent;
    }
    let open = ui
        .push_id(format!("nav.{header}.{name}.menu.open"), |ui| {
            ui.button(t("Open"))
        })
        .inner
        .clicked();
    if open {
        intent.open = true;
        ui.close();
    }
    if kind.has_design_view() {
        let design = ui
            .push_id(format!("nav.{header}.{name}.menu.design"), |ui| {
                ui.button(t("Design"))
            })
            .inner
            .clicked();
        if design {
            intent.design = true;
            ui.close();
        }
    }
    if renamable(kind) {
        // Tables gate on the rename capability (ADR-7); sidecar objects
        // are always renamable.
        let reason = if kind == ObjectKind::Table {
            crate::availability::restricted(
                app.snapshot(),
                datacraft_core::availability::ids::NAV_RENAME,
            )
        } else {
            None
        };
        let rename = ui
            .push_id(
                format!("nav.{header}.{name}.menu.rename"),
                |ui| match reason {
                    Some(reason) => ui
                        .add_enabled(false, egui::Button::new(t("Rename")))
                        .on_hover_text(crate::availability::reason_text(reason)),
                    None => ui.button(t("Rename")),
                },
            )
            .inner
            .clicked();
        if rename {
            intent.rename = true;
            ui.close();
        }
    }
    if deletable(kind) {
        // Capability-disabled for the open source (ADR-7): the entry
        // renders muted with the reason, no click.
        let reason = crate::availability::restricted(
            app.snapshot(),
            datacraft_core::availability::ids::NAV_DELETE,
        );
        let remove = ui
            .push_id(
                format!("nav.{header}.{name}.menu.delete"),
                |ui| match reason {
                    Some(reason) => ui
                        .add_enabled(
                            false,
                            egui::Button::new(tr("Delete {name}", &[("name", name)])),
                        )
                        .on_hover_text(crate::availability::reason_text(reason)),
                    None => ui.button(tr("Delete {name}", &[("name", name)])),
                },
            )
            .inner
            .clicked();
        if remove {
            intent.delete = true;
            ui.close();
        }
    }
    intent
}

/// Macros and modules are a read-only inventory (steps 17.1/17.2): no
/// delete affordance.
fn deletable(kind: ObjectKind) -> bool {
    !matches!(kind, ObjectKind::Macro | ObjectKind::Module)
}

/// Tables, queries, forms and reports rename (M81); the read-only
/// inventories and the native view editor (M93) keep their own surface.
fn renamable(kind: ObjectKind) -> bool {
    matches!(
        kind,
        ObjectKind::Table | ObjectKind::Query | ObjectKind::Form | ObjectKind::Report
    )
}
