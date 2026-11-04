//! Ribbon specification (step 5.2, item H18; ARCHITECTURE §5).
//!
//! Static data: tabs, groups, actions. The UI renders buttons from this
//! spec and maps enabled actions onto commands; disabled actions declare
//! the phase that lands them. Test ids compose per the §5 convention:
//! `ribbon.<tab>.<group>.<action>`.

use std::sync::LazyLock;

/// Whether an action dispatches now or arrives with a later phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionState {
    /// The UI maps this action to a command today.
    Enabled,
    /// Rendered disabled with a tooltip naming the phase.
    DisabledIn {
        /// Phase that lands the action, e.g. `"Phase 6"`.
        phase: &'static str,
    },
}

/// One ribbon action (a button).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RibbonAction {
    /// Stable snake_case id within its group.
    pub id: &'static str,
    /// Button label.
    pub label: &'static str,
    /// Dispatchability.
    pub state: ActionState,
}

/// One ribbon group (visually separated cluster).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RibbonGroup {
    /// Stable snake_case id within its tab.
    pub id: &'static str,
    /// Group caption.
    pub label: &'static str,
    /// Actions in render order.
    pub actions: Vec<RibbonAction>,
}

/// One ribbon tab.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RibbonTab {
    /// Stable snake_case id.
    pub id: &'static str,
    /// Tab caption.
    pub label: &'static str,
    /// Groups in render order.
    pub groups: Vec<RibbonGroup>,
}

/// One contextual tab (step 21.1, item U5): rendered after the base tabs
/// while a tab of the matching object kind is active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextualTab {
    /// Object-kind slug the tab belongs to (`"table"`, `"query"`, …).
    pub context: &'static str,
    /// The tab itself.
    pub tab: RibbonTab,
}

/// The static ribbon specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RibbonSpec {
    /// Base tabs in render order (File first, like Access).
    pub tabs: Vec<RibbonTab>,
    /// Contextual tabs, appended after `tabs` while their kind is active.
    pub contextual_tabs: Vec<ContextualTab>,
    /// Quick Access Toolbar entries in render order (step 21.2, item U6).
    /// Entries reuse ribbon action ids so both surfaces dispatch through the
    /// same mapping.
    pub quick_access: Vec<RibbonAction>,
}

/// The Views group shared by Home and every contextual tab: the Access
/// split-button that owns the only mode-switch affordance (step 21.1).
fn views_group() -> RibbonGroup {
    RibbonGroup {
        id: "views",
        label: "Views",
        actions: vec![RibbonAction {
            id: "view_mode",
            label: "View",
            state: ActionState::Enabled,
        }],
    }
}

/// One contextual tab carrying the shared Views group.
fn contextual_tab(context: &'static str, id: &'static str, label: &'static str) -> ContextualTab {
    ContextualTab {
        context,
        tab: RibbonTab {
            id,
            label,
            groups: vec![views_group()],
        },
    }
}

/// The ribbon specification of the current feature set.
pub static RIBBON: LazyLock<RibbonSpec> = LazyLock::new(|| RibbonSpec {
    tabs: vec![
        RibbonTab {
            id: "file",
            label: "File",
            // The File tab opens the full-page backstage (step 21.3, U7);
            // its flows live there, not in ribbon groups.
            groups: vec![],
        },
        RibbonTab {
            id: "home",
            label: "Home",
            groups: vec![
                RibbonGroup {
                    id: "view",
                    label: "View",
                    actions: vec![RibbonAction {
                        id: "toggle_nav_pane",
                        label: "Navigation Pane",
                        state: ActionState::Enabled,
                    }],
                },
                views_group(),
            ],
        },
        RibbonTab {
            id: "create",
            label: "Create",
            groups: vec![
                RibbonGroup {
                    id: "tables",
                    label: "Tables",
                    actions: vec![
                        RibbonAction {
                            id: "table",
                            label: "Table",
                            state: ActionState::Enabled,
                        },
                        RibbonAction {
                            id: "table_design",
                            label: "Table Design",
                            state: ActionState::DisabledIn { phase: "Phase 6" },
                        },
                    ],
                },
                RibbonGroup {
                    id: "queries",
                    label: "Queries",
                    actions: vec![RibbonAction {
                        id: "query_design",
                        label: "Query Design",
                        state: ActionState::Enabled,
                    }],
                },
                RibbonGroup {
                    id: "forms",
                    label: "Forms",
                    actions: vec![RibbonAction {
                        id: "form_design",
                        label: "Form Design",
                        state: ActionState::Enabled,
                    }],
                },
                RibbonGroup {
                    id: "reports",
                    label: "Reports",
                    actions: vec![RibbonAction {
                        id: "report_design",
                        label: "Report Design",
                        state: ActionState::Enabled,
                    }],
                },
            ],
        },
        RibbonTab {
            id: "external_data",
            label: "External Data",
            groups: vec![RibbonGroup {
                id: "import",
                label: "Import",
                actions: vec![RibbonAction {
                    id: "import_file",
                    label: "Import File",
                    state: ActionState::DisabledIn { phase: "Phase 9" },
                }],
            }],
        },
        RibbonTab {
            id: "database_tools",
            label: "Database Tools",
            groups: vec![
                RibbonGroup {
                    id: "relationships",
                    label: "Relationships",
                    actions: vec![RibbonAction {
                        id: "relationships",
                        label: "Relationships",
                        state: ActionState::Enabled,
                    }],
                },
                RibbonGroup {
                    id: "queries",
                    label: "Queries",
                    actions: vec![
                        RibbonAction {
                            id: "query_sql",
                            label: "SQL",
                            state: ActionState::Enabled,
                        },
                        RibbonAction {
                            id: "query_expression",
                            label: "Expression",
                            state: ActionState::Enabled,
                        },
                    ],
                },
            ],
        },
    ],
    contextual_tabs: vec![
        contextual_tab("table", "table_tools", "Table Tools"),
        contextual_tab("query", "query_tools", "Query Tools"),
        contextual_tab("form", "form_tools", "Form Tools"),
        contextual_tab("report", "report_tools", "Report Tools"),
    ],
    quick_access: vec![
        RibbonAction {
            id: "save_design",
            label: "Save",
            state: ActionState::Enabled,
        },
        RibbonAction {
            id: "undo",
            label: "Undo",
            state: ActionState::Enabled,
        },
        RibbonAction {
            id: "redo",
            label: "Redo",
            state: ActionState::Enabled,
        },
    ],
});

/// The shared ribbon specification.
pub fn ribbon_spec() -> &'static RibbonSpec {
    &RIBBON
}

/// Tabs to render for the active object kind: the base tabs followed by
/// the matching contextual tab, if the kind has one (step 21.1, item U5).
pub fn rendered_tabs(active_kind: Option<&str>) -> Vec<&'static RibbonTab> {
    let spec = ribbon_spec();
    let mut tabs: Vec<&'static RibbonTab> = spec.tabs.iter().collect();
    if let Some(kind) = active_kind {
        tabs.extend(
            spec.contextual_tabs
                .iter()
                .filter(|entry| entry.context == kind)
                .map(|entry| &entry.tab),
        );
    }
    tabs
}
