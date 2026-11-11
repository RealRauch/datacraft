//! The shared grid's input types (step 69.8, L30 split from `grid.rs`).
//!
//! Paint-free descriptors: one grid pass consumes [`Grid`] and
//! [`GridCallbacks`]; the in-place editor slot and its intents travel with
//! them. `grid_ui` in [`super::grid`] renders from these.

use datacraft_core::availability::DisabledReason;
use datacraft_core::grid::GridMetrics;
use datacraft_core::qbe::SortOrder;

/// Everything one grid pass needs; the widths are mutated in place by the
/// header's drag handles and written back by the caller. `id_salt` names
/// the grid's focus anchor (keyboard navigation, step 40.1, item M38) and
/// must be unique per rendered grid; `frozen` is the effective count of
/// frozen leading columns (step 40.2, item M39); `delete_reason` restricts
/// the record-deletion affordance for the open source (step 40.3, M44);
/// `edit` is the armed in-place cell editor (step 40.4, L23) and
/// `edit_restricted` keeps the cells inert on sources that cannot write.
pub(crate) struct Grid<'a> {
    pub metrics: GridMetrics,
    pub columns: &'a [String],
    pub body: &'a [Vec<String>],
    pub cursor: usize,
    pub widths: &'a mut [f32],
    pub sort_orders: &'a [SortOrder],
    /// Active column filters of the tab (empty for query grids); the
    /// header paints their indicator and names the criteria on hover
    /// (step 69.6, item M84).
    pub filters: &'a std::collections::BTreeMap<String, String>,
    pub id_salt: String,
    pub frozen: usize,
    pub delete_reason: Option<DisabledReason>,
    pub edit: Option<CellEditSlot<'a>>,
    pub edit_restricted: bool,
}

/// The armed in-place cell editor (step 40.4, item L23): the cell being
/// edited and its buffer, owned by the tab's presentation state.
pub(crate) struct CellEditSlot<'a> {
    pub row: usize,
    pub col: usize,
    pub buffer: &'a mut String,
}

/// One cell-editing intent reported by [`super::grid::grid_ui`] to the
/// caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum EditIntent {
    /// The clicked cell of the selected record arms the editor.
    Start { row: usize, col: usize },
    /// Enter or a click-away commits the buffer for the cell.
    Commit { row: usize, col: usize },
    /// Escape discards the buffer.
    Cancel,
}

/// The shared Access grid: teal header band with resize handles and sort
/// affordances, alternating row shading, a clickable row selector (reports
/// the clicked row to `on_select`), an accent marker on the cursor row,
/// keyboard record navigation (step 40.1, item M38): arrow keys walk one
/// record, Home/End jump to the ends, PageUp/PageDown by the viewport, and
/// the virtualized window follows the cursor — plus freeze columns (step
/// 40.2, item M39): leading columns stay pinned in header and body while
/// the rest scrolls horizontally; the header context menu freezes and
/// unfreezes through `on_freeze` (the count, 0 = unfreeze). Record
/// deletion (step 40.3, item M44): the row selector's context menu offers
/// "Delete Record" — or a capability-disabled reason — and the Del key
/// deletes the selected record; both report through `on_delete` (the
/// zero-based live-row index) after the caller's own confirmation.
/// In-place cell editing (step 40.4, item L23): a click into a cell of
/// the selected record arms an in-cell editor ([`EditIntent::Start`]),
/// Enter or a click-away commits ([`EditIntent::Commit`]), Escape
/// discards ([`EditIntent::Cancel`]); restricted sources keep the cells
/// inert.
/// The grid's paint callbacks plus the header menu's filter drafts
/// (step 44.5, M54 — bundled so `grid_ui` stays under the arity lint):
/// selection, QBE sort writes (query grids), freeze, record deletion,
/// in-place cell editing and the table column filters.
pub(crate) struct GridCallbacks<'a> {
    /// Row selection (display index).
    pub on_select: &'a mut dyn FnMut(usize),
    /// Header sort (query grids write the QBE sort cell).
    pub on_sort: Option<&'a mut dyn FnMut(usize, SortOrder)>,
    /// Freeze/unfreeze through the header context menu.
    pub on_freeze: Option<&'a mut dyn FnMut(usize)>,
    /// Record deletion (after the caller's confirmation).
    pub on_delete: Option<&'a mut dyn FnMut(usize)>,
    /// In-place cell editing intents.
    pub on_edit: Option<&'a mut dyn FnMut(EditIntent)>,
    /// Opens a column's filter dialog from the header menu (step 45.1,
    /// M55).
    pub on_filter_dialog: Option<&'a mut dyn FnMut(usize)>,
}

/// Bundles the callbacks for [`super::grid::grid_ui`] (keeps the call sites
/// tidy).
#[allow(clippy::too_many_arguments)]
pub(crate) fn grid_ui_callbacks<'a>(
    on_select: &'a mut dyn FnMut(usize),
    on_sort: Option<&'a mut dyn FnMut(usize, SortOrder)>,
    on_freeze: Option<&'a mut dyn FnMut(usize)>,
    on_delete: Option<&'a mut dyn FnMut(usize)>,
    on_edit: Option<&'a mut dyn FnMut(EditIntent)>,
    on_filter_dialog: Option<&'a mut dyn FnMut(usize)>,
) -> GridCallbacks<'a> {
    GridCallbacks {
        on_select,
        on_sort,
        on_freeze,
        on_delete,
        on_edit,
        on_filter_dialog,
    }
}
