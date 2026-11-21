//! Navigation-pane local groups and view preference (step 92.5, item
//! L48).
//!
//! The pane grows the last two L48 surfaces: an **Object Details** view
//! (one flat, searchable list across the categories) and **local groups
//! with colours** — the reference client's custom groups. Both are
//! UI-owned preferences stored in `ui.json` keyed by the database path
//! (ADR-5); the `.datacraft` sidecar and the database file never see
//! them. Groups are created from the pane's multi-selection (step 92.4),
//! carry a palette colour and list their objects with the pane's row
//! language.

use std::path::Path;

use eframe::egui;

use datacraft_core::ObjectId;
use datacraft_core::state::TabMode;

use serde::{Deserialize, Serialize};

use crate::app::App;
use crate::i18n::{t, tr};

/// The pane's view mode: the category tree (the default) or the flat
/// Object Details list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NavView {
    /// The category sections (Tables, Queries, …).
    #[default]
    Tree,
    /// One flat, searchable list across the categories.
    Details,
}

impl NavView {
    /// Maps the persisted value; unknown or absent values stay in the
    /// tree view.
    pub fn from_value(value: Option<&str>) -> Self {
        match value {
            Some("details") => Self::Details,
            _ => Self::Tree,
        }
    }

    /// The persisted value.
    pub fn as_value(self) -> &'static str {
        match self {
            Self::Tree => "tree",
            Self::Details => "details",
        }
    }
}

/// The fixed group palette (UI-owned; the swatches read in both palettes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NavColour {
    /// Red swatch.
    Red,
    /// Orange swatch.
    Orange,
    /// Yellow swatch.
    Yellow,
    /// Green swatch.
    Green,
    /// Blue swatch.
    Blue,
    /// Purple swatch.
    Purple,
    /// Teal swatch.
    Teal,
    /// The neutral default.
    #[default]
    Grey,
}

impl NavColour {
    /// The palette in rotation order.
    pub const ALL: &'static [NavColour] = &[
        Self::Red,
        Self::Orange,
        Self::Yellow,
        Self::Green,
        Self::Blue,
        Self::Purple,
        Self::Teal,
        Self::Grey,
    ];

    /// Maps the persisted value; unknown or absent values take the
    /// neutral default.
    pub fn from_value(value: Option<&str>) -> Self {
        match value {
            Some("red") => Self::Red,
            Some("orange") => Self::Orange,
            Some("yellow") => Self::Yellow,
            Some("green") => Self::Green,
            Some("blue") => Self::Blue,
            Some("purple") => Self::Purple,
            Some("teal") => Self::Teal,
            _ => Self::Grey,
        }
    }

    /// The persisted value.
    pub fn as_value(self) -> &'static str {
        match self {
            Self::Red => "red",
            Self::Orange => "orange",
            Self::Yellow => "yellow",
            Self::Green => "green",
            Self::Blue => "blue",
            Self::Purple => "purple",
            Self::Teal => "teal",
            Self::Grey => "grey",
        }
    }

    /// The localized swatch name.
    pub fn label(self) -> &'static str {
        match self {
            Self::Red => t("Red"),
            Self::Orange => t("Orange"),
            Self::Yellow => t("Yellow"),
            Self::Green => t("Green"),
            Self::Blue => t("Blue"),
            Self::Purple => t("Purple"),
            Self::Teal => t("Teal"),
            Self::Grey => t("Grey"),
        }
    }

    /// The swatch ink (fixed — legible in both palettes).
    pub fn ink(self) -> egui::Color32 {
        match self {
            Self::Red => egui::Color32::from_rgb(0xC0, 0x39, 0x2B),
            Self::Orange => egui::Color32::from_rgb(0xCA, 0x6F, 0x1E),
            Self::Yellow => egui::Color32::from_rgb(0xB7, 0x95, 0x00),
            Self::Green => egui::Color32::from_rgb(0x1E, 0x8E, 0x3E),
            Self::Blue => egui::Color32::from_rgb(0x1A, 0x73, 0xE8),
            Self::Purple => egui::Color32::from_rgb(0x8E, 0x44, 0xAD),
            Self::Teal => egui::Color32::from_rgb(0x2E, 0x8B, 0x8B),
            Self::Grey => egui::Color32::from_rgb(0x8A, 0x8A, 0x8A),
        }
    }

    /// The palette colour a group at `index` takes (deterministic, wraps).
    pub fn for_position(index: usize) -> Self {
        Self::ALL[index % Self::ALL.len()]
    }
}

/// One local group: the user's name, a palette colour and its members.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NavGroup {
    /// The group name (unique per database).
    pub name: String,
    /// The group colour.
    pub colour: NavColour,
    /// The member objects (cross-kind).
    pub objects: Vec<ObjectId>,
}

impl NavGroup {
    /// Removes one object from the group.
    pub fn retain_object(&mut self, object: &ObjectId) {
        self.objects.retain(|candidate| candidate != object);
    }
}

/// The per-database pane preferences.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct NavPrefs {
    /// The pane's view mode.
    pub view: NavView,
    /// The local groups, in creation order.
    pub groups: Vec<NavGroup>,
}

/// The per-database preference key: the database path itself.
pub fn prefs_key(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

impl App {
    /// The stored pane preferences of one database; a database without a
    /// record uses the defaults.
    pub fn nav_prefs(&self, key: &str) -> NavPrefs {
        self.settings.nav.get(key).cloned().unwrap_or_default()
    }

    /// Switches the pane's view mode for one database.
    pub fn set_nav_view(&mut self, key: &str, view: NavView) {
        let mut prefs = self.nav_prefs(key);
        if prefs.view == view {
            return;
        }
        prefs.view = view;
        self.settings.nav.insert(key.to_owned(), prefs);
    }

    /// Creates a local group from the selection; the palette colour is
    /// assigned by position. A name that already exists is refused
    /// silently (the dialog stays open).
    pub fn add_nav_group(&mut self, key: &str, name: impl Into<String>, objects: Vec<ObjectId>) {
        let name = name.into();
        let mut prefs = self.nav_prefs(key);
        if name.trim().is_empty() || prefs.groups.iter().any(|group| group.name == name) {
            return;
        }
        let colour = NavColour::for_position(prefs.groups.len());
        prefs.groups.push(NavGroup {
            name,
            colour,
            objects,
        });
        self.settings.nav.insert(key.to_owned(), prefs);
    }

    /// Deletes a local group.
    pub fn delete_nav_group(&mut self, key: &str, name: &str) {
        let mut prefs = self.nav_prefs(key);
        prefs.groups.retain(|group| group.name != name);
        self.settings.nav.insert(key.to_owned(), prefs);
    }

    /// Recolours a local group.
    pub fn set_nav_group_colour(&mut self, key: &str, name: &str, colour: NavColour) {
        let mut prefs = self.nav_prefs(key);
        if let Some(group) = prefs.groups.iter_mut().find(|group| group.name == name) {
            group.colour = colour;
        }
        self.settings.nav.insert(key.to_owned(), prefs);
    }

    /// Removes one object from a local group.
    pub fn remove_from_nav_group(&mut self, key: &str, group: &str, object: &ObjectId) {
        let mut prefs = self.nav_prefs(key);
        if let Some(target) = prefs
            .groups
            .iter_mut()
            .find(|candidate| candidate.name == group)
        {
            target.retain_object(object);
        }
        self.settings.nav.insert(key.to_owned(), prefs);
    }

    /// The pending "new group" draft (the dialog's buffer).
    pub fn group_draft(&self) -> Option<&GroupDraft> {
        self.group_draft.as_ref()
    }

    /// Opens the group dialog over the current selection.
    pub fn open_group_dialog(&mut self, objects: Vec<ObjectId>) {
        self.group_draft = Some(GroupDraft {
            objects,
            name: String::new(),
        });
    }

    /// Replaces the draft name.
    pub fn set_group_draft_name(&mut self, name: &str) {
        if let Some(draft) = &mut self.group_draft {
            draft.name = name.to_owned();
        }
    }

    /// Creates the group from the draft (nothing without a database or a
    /// name) and closes the dialog.
    pub fn create_group_from_draft(&mut self) {
        let Some(draft) = self.group_draft.clone() else {
            return;
        };
        let Some(db) = self.snapshot().database.as_ref() else {
            return;
        };
        let key = prefs_key(&db.path);
        self.add_nav_group(&key, draft.name.trim().to_owned(), draft.objects);
        self.group_draft = None;
    }

    /// Closes the dialog without creating.
    pub fn close_group_dialog(&mut self) {
        self.group_draft = None;
    }
}

/// The group dialog's draft.
#[derive(Debug, Clone, PartialEq)]
pub struct GroupDraft {
    /// The objects the group starts with.
    pub objects: Vec<ObjectId>,
    /// The edited group name.
    pub name: String,
}

/// Renders the group-creation dialog (when open).
pub fn group_dialog(app: &mut App, ui: &mut egui::Ui) {
    let Some(draft) = app.group_draft().cloned() else {
        return;
    };
    let mut name = draft.name.clone();
    let mut close = false;
    let mut create = false;
    egui::Window::new(t("New Group"))
        .resizable(false)
        .default_width(360.0)
        .default_pos(crate::tool_window::centered_pos(
            ui.ctx(),
            egui::vec2(360.0, 170.0),
        ))
        .show(ui, |ui| {
            ui.label(tr(
                "{count} selected objects",
                &[("count", &draft.objects.len().to_string())],
            ));
            let label = ui.label(t("Group name"));
            let field = ui.text_edit_singleline(&mut name).labelled_by(label.id);
            if field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                create = true;
            }
            ui.horizontal(|ui| {
                if ui.button(t("Create")).clicked() {
                    create = true;
                }
                if ui.button(t("Cancel")).clicked() {
                    close = true;
                }
            });
        });
    app.set_group_draft_name(&name);
    if create {
        app.create_group_from_draft();
    }
    if close {
        app.close_group_dialog();
    }
}

/// The intents the pane's group surface collected (applied after the
/// snapshot borrow ends).
#[derive(Debug, Default)]
pub(crate) struct GroupsIntent {
    /// Whether any group content rendered (the search empty-state rule).
    pub matched: bool,
    /// Open one group object (datasheet or design).
    pub open: Option<(ObjectId, Option<TabMode>)>,
    /// Remove one object from one group.
    pub remove: Option<(String, ObjectId)>,
    /// Delete one group.
    pub delete_group: Option<String>,
    /// Recolour one group.
    pub colour: Option<(String, NavColour)>,
}

/// Renders the pane header's view dropdown; returns the requested view.
pub(crate) fn view_menu(ui: &mut egui::Ui, prefs: &NavPrefs) -> Option<NavView> {
    let mut intent = None;
    let current = match prefs.view {
        NavView::Tree => t("All Access Objects"),
        NavView::Details => t("Object Details"),
    };
    ui.menu_button(current, |ui| {
        for (view, label) in [
            (NavView::Tree, t("All Access Objects")),
            (NavView::Details, t("Object Details")),
        ] {
            if ui
                .add_enabled(prefs.view != view, egui::Button::new(label))
                .clicked()
            {
                intent = Some(view);
                ui.close();
            }
        }
    });
    intent
}

/// Renders the Custom Groups section (when any group exists): one
/// coloured collapsible per group listing its objects with the pane's
/// row language, the group menu carrying recolour and delete, the object
/// menu carrying remove-from-group.
pub(crate) fn groups_section(ui: &mut egui::Ui, prefs: &NavPrefs, query: &str) -> GroupsIntent {
    let mut intent = GroupsIntent::default();
    let groups: Vec<&NavGroup> = prefs
        .groups
        .iter()
        .filter(|group| {
            group
                .objects
                .iter()
                .any(|object| crate::nav_search::matches(query, &object.name))
        })
        .collect();
    if groups.is_empty() {
        return intent;
    }
    intent.matched = true;
    ui.strong(t("Custom Groups"));
    for group in groups {
        egui::CollapsingHeader::new(
            egui::RichText::new(&group.name)
                .strong()
                .color(group.colour.ink()),
        )
        .default_open(true)
        .show(ui, |ui| {
            for object in &group.objects {
                ui.push_id(format!("nav.group.{}.{}", group.name, object.name), |ui| {
                    let row_height = ui
                        .spacing()
                        .interact_size
                        .y
                        .max(crate::icons::ICON_SIZE + 4.0);
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), row_height),
                        egui::Sense::hover(),
                    );
                    let row_id = ui.scope_id().with(("nav.group.row", &object.name));
                    let row = ui.interact(rect, row_id, egui::Sense::click());
                    row.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::Role::Button,
                            true,
                            false,
                            object.name.clone(),
                        )
                    });
                    crate::nav_row::paint_row(
                        ui,
                        rect,
                        row_height,
                        &object.name,
                        object.kind,
                        false,
                        None,
                    );
                    if row.clicked() {
                        intent.open = Some((object.clone(), None));
                    }
                    row.context_menu(|ui| {
                        if ui.button(t("Open")).clicked() {
                            intent.open = Some((object.clone(), None));
                            ui.close();
                        }
                        if object.kind.has_design_view() && ui.button(t("Design")).clicked() {
                            intent.open = Some((object.clone(), Some(TabMode::Design)));
                            ui.close();
                        }
                        if ui.button(t("Remove from group")).clicked() {
                            intent.remove = Some((group.name.clone(), object.clone()));
                            ui.close();
                        }
                    });
                });
            }
        })
        .header_response
        .context_menu(|ui| {
            ui.menu_button(t("Colour"), |ui| {
                for colour in NavColour::ALL {
                    if ui
                        .add_enabled(group.colour != *colour, egui::Button::new(colour.label()))
                        .clicked()
                    {
                        intent.colour = Some((group.name.clone(), *colour));
                        ui.close();
                    }
                }
            });
            if ui.button(t("Delete group")).clicked() {
                intent.delete_group = Some(group.name.clone());
                ui.close();
            }
        });
    }
    intent
}
