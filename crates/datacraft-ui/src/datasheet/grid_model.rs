//! The shared grid's input types (step 69.8, L30 split from `grid.rs`).
//!
//! Paint-free descriptors: one grid pass consumes [`Grid`] and
//! [`GridCallbacks`]; the in-place editor slot and its intents travel with
//! them. `grid_ui` in [`super::grid`] renders from these.

use datacraft_core::availability::DisabledReason;
use datacraft_core::edit_buffer::RowState;
use datacraft_core::grid::GridMetrics;
use datacraft_core::qbe::SortOrder;
use datacraft_core::selection::Selection;

/// Pending state of one displayed row (step 85.2, item H61): the row's
/// buffer state and the column indices carrying staged changes — the
/// amber-marker positions. Parallel to the grid's body rows; rows absent
/// from the slice render unchanged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PendingRow {
    /// The row's edit-buffer state.
    pub state: RowState,
    /// Column indices with staged changes.
    pub changed: Vec<usize>,
}

/// Everything one grid pass needs; the widths are mutated in place by the
/// header's drag handles and written back by the caller. `id_salt` names
/// the grid's focus anchor (keyboard navigation, step 40.1, item M38) and
/// must be unique per rendered grid; `frozen` is the effective count of
/// frozen leading columns (step 40.2, item M39); `delete_reason` restricts
/// the record-deletion affordance for the open source (step 40.3, M44);
/// `edit` is the armed in-place cell editor (step 40.4, L23) and
/// `edit_restricted` keeps the cells inert on sources that cannot write;
/// `pending` carries the edit-buffer markers (step 85.2, H61) and `locked`
/// refuses every grid input while a submission is in flight. `selection`
/// is the tab's current multi-cell selection (step 86.1, M101 — display
/// positions over `body`, already clamped by the caller).
pub(crate) struct Grid<'a> {
    pub metrics: GridMetrics,
    pub columns: &'a [String],
    pub body: &'a [Vec<String>],
    pub cursor: usize,
    pub selection: &'a Selection,
    pub widths: &'a mut [f32],
    pub sort_orders: &'a [SortOrder],
    /// Active column filters of the tab (empty for query grids); the
    /// header paints their indicator and names the criteria on hover
    /// (step 69.6, item M84).
    pub filters: &'a std::collections::BTreeMap<String, String>,
    /// Per-column follow-target labels (step 87.1, item M103): the cell
    /// context menu lists one entry per target; an empty list hides the
    /// follow affordance for the column (query grids: empty).
    pub follow_targets: &'a [Vec<String>],
    /// The count suffix of one menu entry (step 87.2, item M103): the
    /// (row, col, index)-resolved " (n)"/" (≥ n)" of a reverse target,
    /// `None` — pending or forward targets render bare.
    pub follow_counts: &'a dyn Fn(usize, usize, usize) -> Option<String>,
    pub id_salt: String,
    pub frozen: usize,
    pub delete_reason: Option<DisabledReason>,
    pub edit: Option<CellEditSlot<'a>>,
    pub edit_restricted: bool,
    pub pending: &'a [PendingRow],
    pub locked: bool,
    /// Height policy of the body scroll area (step 91.2, item M110):
    /// `None` fills the available height (the datasheet and the tab-hosted
    /// query/view grids); `Some(bound)` shrinks the body to its content
    /// and caps it at `bound` — floating-window embedders pass a bound so
    /// a small result never grows the window and a large one scrolls
    /// inside it instead of feeding the window's size loop.
    pub max_height: Option<f32>,
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
/// selection, the multi-cell selection changes (step 86.1, M101), QBE
/// sort writes (query grids), freeze, record deletion, in-place cell
/// editing and the table column filters.
pub(crate) struct GridCallbacks<'a> {
    /// Row selection (display index).
    pub on_select: &'a mut dyn FnMut(usize),
    /// The new selection after a keyboard or mouse intent (step 86.1,
    /// M101); the caller stores it as the tab's presentation state.
    pub on_selection: Option<&'a mut dyn FnMut(Selection)>,
    /// Column selection through the header (the column and whether the
    /// header was shift-clicked, extending the anchor).
    pub on_select_column: Option<&'a mut dyn FnMut(usize, bool)>,
    /// Ctrl+C over a non-empty grid (step 86.2, M101): the caller builds
    /// the TSV from the body it passed in.
    pub on_copy: Option<&'a mut dyn FnMut()>,
    /// Ctrl+V on a writable, unlocked grid (step 86.2, M101): the caller
    /// reads the clipboard, validates and writes.
    pub on_paste: Option<&'a mut dyn FnMut()>,
    /// Ctrl+D on a writable, unlocked grid with an extended selection
    /// (step 86.3, M101): fills the selection from its top row.
    pub on_fill: Option<&'a mut dyn FnMut()>,
    /// Del over an extended selection (step 86.3, M101): clears the
    /// cells to NULL; a collapsed cell stays the record deletion (M44).
    pub on_clear: Option<&'a mut dyn FnMut()>,
    /// Zoom on one cell (step 86.4, M102): Shift+F2 over the focus cell,
    /// Alt+click, a double click on a read-only cell or the cell context
    /// menu's "Zoom Value" report the displayed (row, col).
    pub on_zoom: Option<&'a mut dyn FnMut(usize, usize)>,
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
    /// Opens the column's value list (phase 101, step 101.2; item M132).
    pub on_value_list: Option<&'a mut dyn FnMut(usize)>,
    /// Moves a column one display position (phase 101, step 101.4; M132).
    pub on_move_column: Option<&'a mut dyn FnMut(usize, bool)>,
    /// The visible row range of this frame (phase 101, step 101.5; M132).
    pub on_visible_rows: Option<&'a mut dyn FnMut(usize, usize)>,
    /// The follow intent (step 87.1, item M103): the displayed (row,
    /// col) and the context menu's target index — F4 reports index 0.
    pub on_follow: Option<&'a mut dyn FnMut(usize, usize, usize)>,
    /// The cell context menu shows for (row, col) (step 87.2, item
    /// M103) — fired every frame the menu stays open; the caller
    /// deduplicates its count reads against the answered map.
    pub on_follow_menu: Option<&'a mut dyn FnMut(usize, usize)>,
    /// Opens the tab-wide filter builder window (step 87.4, item M104).
    pub filter_builder: Option<&'a mut dyn FnMut()>,
}

/// Bundles the callbacks for [`super::grid::grid_ui`] (keeps the call sites
/// tidy).
#[allow(clippy::too_many_arguments)]
pub(crate) fn grid_ui_callbacks<'a>(
    on_select: &'a mut dyn FnMut(usize),
    on_selection: Option<&'a mut dyn FnMut(Selection)>,
    on_select_column: Option<&'a mut dyn FnMut(usize, bool)>,
    on_copy: Option<&'a mut dyn FnMut()>,
    on_paste: Option<&'a mut dyn FnMut()>,
    on_fill: Option<&'a mut dyn FnMut()>,
    on_clear: Option<&'a mut dyn FnMut()>,
    on_zoom: Option<&'a mut dyn FnMut(usize, usize)>,
    on_sort: Option<&'a mut dyn FnMut(usize, SortOrder)>,
    on_freeze: Option<&'a mut dyn FnMut(usize)>,
    on_delete: Option<&'a mut dyn FnMut(usize)>,
    on_edit: Option<&'a mut dyn FnMut(EditIntent)>,
    on_filter_dialog: Option<&'a mut dyn FnMut(usize)>,
    on_value_list: Option<&'a mut dyn FnMut(usize)>,
    on_move_column: Option<&'a mut dyn FnMut(usize, bool)>,
    on_visible_rows: Option<&'a mut dyn FnMut(usize, usize)>,
    on_follow: Option<&'a mut dyn FnMut(usize, usize, usize)>,
    on_follow_menu: Option<&'a mut dyn FnMut(usize, usize)>,
    filter_builder: Option<&'a mut dyn FnMut()>,
) -> GridCallbacks<'a> {
    GridCallbacks {
        on_select,
        on_selection,
        on_select_column,
        on_copy,
        on_paste,
        on_fill,
        on_clear,
        on_zoom,
        on_sort,
        on_freeze,
        on_delete,
        on_edit,
        on_filter_dialog,
        on_value_list,
        on_move_column,
        on_visible_rows,
        on_follow,
        on_follow_menu,
        filter_builder,
    }
}
