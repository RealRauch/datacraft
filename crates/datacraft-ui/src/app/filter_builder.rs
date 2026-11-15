//! The filter builder's draft (step 87.4, item M104): the window edits
//! a per-tab draft — nothing dispatches until Apply. The draft seeds
//! from the live criteria map (the builder-to-criteria round trip); a
//! live filter the model cannot read opens the text mode as the
//! escape hatch.

use std::collections::BTreeMap;

use datacraft_core::filter_builder::{
    BuilderCondition, BuilderSort, FilterRead, deserialize_filters,
};
use datacraft_core::ids::TabId;
use datacraft_core::qbe::SortOrder;
use datacraft_core::snapshot::TabSnapshot;

use super::App;

/// The builder window's pane (step 87.4, item M104): the structured
/// conditions or the raw criteria text — the text mode is the escape
/// hatch the dialect's unmodelable forms point at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuilderPane {
    Builder,
    Text,
}

/// The builder window's draft (step 87.4, item M104). Editing mutates
/// only this state; Apply dispatches the existing bus commands.
#[derive(Debug, Clone)]
pub struct FilterBuilderDraft {
    /// The active pane.
    pub mode: BuilderPane,
    /// The AND list the builder edits (the core model's flat shape —
    /// OR is the dialect fence and never enters the draft).
    pub conditions: Vec<BuilderCondition>,
    /// The sort line (the existing single-column path; an empty column
    /// or `None` order clears the sort).
    pub sort: Option<BuilderSort>,
    /// The text mode's raw criteria per column (seeded from the live
    /// map in every pane — Apply reads the active pane's side).
    pub text: BTreeMap<String, String>,
}

impl App {
    pub(crate) fn filter_builder_draft(
        &mut self,
        tab_id: TabId,
    ) -> Option<&mut FilterBuilderDraft> {
        self.filter_builder_drafts.get_mut(&tab_id)
    }

    /// Opens the builder window for a tab: the draft seeds from the
    /// live criteria map — a modelable map seeds the conditions, an
    /// unmodelable form opens the text mode carrying the raw values
    /// (the escape hatch, step 87.4, item M104). The sort line seeds
    /// from the tab's live sort.
    pub(crate) fn open_filter_builder(&mut self, tab: &TabSnapshot) {
        let live = tab.filters.clone();
        let (mode, conditions) = match deserialize_filters(&live) {
            FilterRead::Model(node) => {
                let conditions = match node {
                    datacraft_core::filter_builder::FilterNode::Group {
                        connective: datacraft_core::filter_builder::Connective::And,
                        children,
                    } => children
                        .into_iter()
                        .filter_map(|child| match child {
                            datacraft_core::filter_builder::FilterNode::Condition(condition) => {
                                Some(condition)
                            }
                            _ => None,
                        })
                        .collect(),
                    datacraft_core::filter_builder::FilterNode::Condition(condition) => {
                        vec![condition]
                    }
                    _ => Vec::new(),
                };
                (crate::app::filter_builder::BuilderPane::Builder, conditions)
            }
            FilterRead::Failed(_) => (crate::app::filter_builder::BuilderPane::Text, Vec::new()),
        };
        let sort = tab.sort_field.clone().map(|column| BuilderSort {
            column,
            order: tab.sort_order.unwrap_or(SortOrder::Ascending),
        });
        self.filter_builder_drafts.insert(
            tab.id,
            FilterBuilderDraft {
                mode,
                conditions,
                sort,
                text: live,
            },
        );
    }

    pub(crate) fn close_filter_builder(&mut self, tab_id: TabId) {
        self.filter_builder_drafts.remove(&tab_id);
    }
}
