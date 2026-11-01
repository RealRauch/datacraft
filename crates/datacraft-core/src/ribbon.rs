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

/// The static ribbon specification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RibbonSpec {
    /// Tabs in render order (File first, like Access).
    pub tabs: Vec<RibbonTab>,
}

/// The ribbon of the Phase 5 skeleton.
pub static RIBBON: LazyLock<RibbonSpec> = LazyLock::new(|| RibbonSpec {
    tabs: vec![
        RibbonTab {
            id: "file",
            label: "File",
            groups: vec![
                RibbonGroup {
                    id: "open",
                    label: "Open",
                    actions: vec![RibbonAction {
                        id: "open_database",
                        label: "Open Database",
                        state: ActionState::Enabled,
                    }],
                },
                RibbonGroup {
                    id: "database",
                    label: "Database",
                    actions: vec![RibbonAction {
                        id: "close_database",
                        label: "Close Database",
                        state: ActionState::Enabled,
                    }],
                },
            ],
        },
        RibbonTab {
            id: "home",
            label: "Home",
            groups: vec![RibbonGroup {
                id: "view",
                label: "View",
                actions: vec![RibbonAction {
                    id: "toggle_nav_pane",
                    label: "Navigation Pane",
                    state: ActionState::Enabled,
                }],
            }],
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
                            state: ActionState::DisabledIn { phase: "Phase 6" },
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
            groups: vec![RibbonGroup {
                id: "relationships",
                label: "Relationships",
                actions: vec![RibbonAction {
                    id: "relationships",
                    label: "Relationships",
                    state: ActionState::Enabled,
                }],
            }],
        },
    ],
});

/// The shared ribbon specification.
pub fn ribbon_spec() -> &'static RibbonSpec {
    &RIBBON
}
