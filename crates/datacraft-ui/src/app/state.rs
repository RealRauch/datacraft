//! The `App` struct's fields (split from `app.rs` for the file-size
//! gate, step 86.2): the shell's presentation and cache state. The
//! accessors live in the sibling modules; `new` stays with the module
//! that owns the composition root.

use std::collections::HashMap;

use datacraft_core::core::Core;
use datacraft_core::ids::TabId;
use datacraft_core::query_engine::QueryResult;
use datacraft_core::schema::FieldValue;
use datacraft_core::snapshot::UiStateSnapshot;

/// Root application state.
pub struct App {
    pub(crate) core: Core,
    pub(super) snapshot: UiStateSnapshot,
    /// Path input buffer of the File > Open Database ribbon action.
    pub open_path: String,
    /// Selected ribbon tab id (presentation state, `&'static` from the spec).
    pub active_ribbon_tab: &'static str,
    /// Object kind of the active document as last seen by the ribbon
    /// (step 47.1, M58): the selection follows *kind changes* only.
    pub(crate) active_object_kind: Option<String>,
    /// Selected backstage page while the File tab is active (step 21.3).
    pub backstage_page: crate::backstage::BackstagePage,
    /// Navigation-pane group sort (step 21.5; ordering lives in core).
    pub nav_sort: datacraft_core::nav::NavSortOrder,
    /// Navigation-pane search query (step 92.3, item L48) — session-only
    /// presentation state, never persisted.
    pub nav_search: String,
    /// Navigation-pane selection (step 92.4, item L48) — the rows picked
    /// with Ctrl/Shift clicks, in click order; session-only state.
    pub nav_selection: Vec<datacraft_core::ObjectId>,
    /// The range anchor of the selection (the last plain-clicked row).
    pub nav_anchor: Option<datacraft_core::ObjectId>,
    /// The pending "new group" dialog draft (step 92.5, item L48).
    pub group_draft: Option<crate::nav_groups::GroupDraft>,
    /// The resolved rendering mode (step 92.6, item L51) — the
    /// composition root resolves it once at startup.
    pub render_mode: crate::render_mode::RenderMode,
    /// The previous frame's start (the overlay's frame time).
    pub(crate) last_frame: Option<std::time::Instant>,
    /// Last viewport title sent to the window system (step 21.7, U11).
    pub(crate) window_title: String,
    /// Current ribbon key-tip phase (step 21.8, U12).
    pub keytips: Option<crate::keys::KeyTipsPhase>,
    /// Alt state of the previous frame (edge detection for key tips).
    pub(crate) alt_down: bool,
    /// Row data per table tab, loaded once on tab appearance.
    pub(crate) rows: HashMap<TabId, Vec<Vec<FieldValue>>>,
    /// Bounded row windows of the large table tabs (phase 101, step
    /// 101.5; M132): a tab in this map is windowed and has no full cache.
    pub(crate) row_windows: crate::row_window::RowWindows,
    /// Query results per query tab, refreshed when the QBE model changes.
    pub(super) results: HashMap<TabId, QueryResult>,
    /// Cached SQL results of open view tabs (step 71.2, item M93).
    pub(crate) view_results: HashMap<TabId, datacraft_core::sql_types::SqlResult>,
    /// The open view editor draft (step 71.2, item M93).
    pub(crate) view_draft: Option<crate::views_window::ViewDraft>,
    /// The open object-rename dialog (phase 72, step 72.4; item M81).
    pub(crate) rename_draft: Option<crate::rename_dialog::RenameDraft>,
    /// Selected control per form-design tab.
    pub(super) form_selection: HashMap<TabId, String>,
    /// Content drafts of the designer's control panel, keyed by tab and
    /// control (step 63.2, item H56); cleared on apply and selection.
    pub(crate) control_content_drafts: HashMap<(TabId, String), String>,
    /// Buffered control name of the property sheet per design tab
    /// (phase 102, item M133); re-seeds when the selection changes.
    pub(crate) property_drafts: HashMap<TabId, crate::control_properties::PropertyDraft>,
    /// The palette field being dragged over a designer canvas (phase 102,
    /// step 102.4): the canvas places it on release inside its bounds.
    pub(crate) field_drag: Option<crate::designer_palette::FieldDrag>,
    /// Drag accumulation per (tab, control): (dx, dy), present while dragging.
    pub(super) drags: HashMap<(TabId, String), (f32, f32)>,
    /// Current runtime record index per form tab.
    pub(super) form_record: HashMap<TabId, usize>,
    /// Per-record edit buffer of every form tab (phase 64, M68; ADR-8).
    pub(super) form_edit: super::form_edit::FormEditState,
    /// Selected design-grid field index per tab.
    pub(super) design_selection: HashMap<TabId, usize>,
    /// Buffered new-field name per design tab (step 24.3, M9).
    pub(super) design_new_field: HashMap<TabId, String>,
    pub(super) filter_drafts: HashMap<(TabId, usize), String>,
    pub(super) filter_dialog: HashMap<TabId, usize>,
    /// The open value list of the column header (phase 101, step 101.2;
    /// item M132) — filled by the `ColumnValuesListed` event.
    pub(crate) value_filter: Option<crate::datasheet::filter_value::ValueList>,
    /// The find/replace window's draft (phase 101, step 101.3; M132) —
    /// an open window when `Some`.
    pub find_replace: Option<crate::find_replace::FindReplaceDraft>,
    /// Display order of the datasheet columns per table tab (phase 101,
    /// step 101.4; M132): display position → storage column index.
    pub(crate) column_order: HashMap<TabId, Vec<usize>>,
    /// Buffered rename text per design tab (step 24.3, M9).
    pub(super) design_rename: HashMap<TabId, String>,
    /// Current record cursor per table/query tab (the status bar's
    /// record-navigation box and the datasheet row selector).
    pub(super) record_cursor: HashMap<TabId, usize>,
    /// Per-tab datasheet selection (step 86.1, item M101): the core
    /// model's display positions over the loaded rows; presentation
    /// state, clamped against the current bounds on every read.
    pub(super) selections: HashMap<TabId, datacraft_core::selection::Selection>,
    /// Current preview page of a report tab (defaults to the first).
    pub(super) preview_page: HashMap<TabId, usize>,
    /// Cached preview pages per report tab (refreshed on dispatch).
    pub(super) previews: HashMap<TabId, Vec<datacraft_core::report_preview::PreviewPage>>,
    /// SQL window editor text (presentation state; the statement runs
    /// through the command bus).
    pub(super) sql_text: String,
    /// The SQL window's query-plan toggle (native sources only).
    pub(super) sql_explain: bool,
    /// Reason of the last failed SQL run (the window renders it).
    pub(super) sql_error: Option<String>,
    /// Import File window buffers (presentation state; the source and the
    /// target table run through the command bus, step 58.1, item M50).
    /// Crate-visible: the accessors live in `flows.rs` (size gate).
    pub(crate) import_source: String,
    pub(crate) import_table: String,
    /// Expression window editor text (presentation state; the expression
    /// runs through the command bus, step 41.2).
    pub(crate) expression_text: String,
    /// The expression window's picked row context (presentation state;
    /// the table plus the zero-based row as typed text — step 60.1,
    /// item L27). `None` evaluates over the empty scope. Crate-visible:
    /// the accessors live in `flows.rs` (size gate).
    pub(crate) expression_context: Option<(String, String)>,
    /// Display form of the last successful expression run (the window
    /// renders it, step 41.2).
    pub(crate) expression_output: Option<String>,
    /// Reason of the last failed expression run (the window renders it).
    pub(crate) expression_error: Option<String>,
    /// Bounded notice/error history (step 68.3, item M76): the status bar
    /// shows the newest entry, the messages dialog lists the rest.
    pub(super) notices: super::notices::NoticeLog,
    /// Whether the messages dialog is open.
    pub(crate) notices_open: bool,
    /// Whether the shortcuts help page is open (step 68.4, item M77).
    pub(crate) shortcuts_help_open: bool,
    /// Deferred busy command and its overlay state (step 68.5, item M75).
    pub(crate) busy: crate::busy::BusyState,
    /// Path buffer of the sidecar save/load ribbon actions (step 11.8).
    pub sidecar_path: String,
    /// Draft of the two-step New Database page (step 75.2, item L42).
    pub new_database: crate::backstage::NewDatabaseDraft,
    /// Path buffer of the backstage Save-a-copy flow (step 27.2, M14).
    pub save_as_path: String,
    /// Whether the copy is protected (the checkbox next to the path).
    pub save_as_encrypted: bool,
    /// Password buffer of the Save-a-copy flow.
    pub save_as_password: String,
    /// New-password buffer of the change-password flow.
    pub change_password: String,
    /// Buffered cells of the datasheet's new-record row per table tab.
    pub(super) new_row: HashMap<TabId, Vec<String>>,
    /// The armed in-place cell editor per table tab (step 40.4, L23):
    /// (row, col) plus the editable buffer; absent while no cell is
    /// being edited.
    pub(super) cell_edit: HashMap<TabId, (usize, usize, String)>,
    /// Per-tab column widths of the datasheet (presentation state; the
    /// resize math lives in core `grid`, step 21.4).
    pub(super) column_widths: HashMap<TabId, Vec<f32>>,
    /// Per-tab count of frozen leading datasheet columns (presentation
    /// state; the freeze placement math lives in core `grid`, step 40.2).
    pub(super) frozen_columns: HashMap<TabId, usize>,
    /// Column widths of the SQL window's result grid (step 21.4).
    pub(super) sql_column_widths: Vec<f32>,
    /// Selection of the SQL window's result grid (step 86.1, item M101;
    /// presentation state like the widths above).
    pub(super) sql_selection: datacraft_core::selection::Selection,
    /// The open cell value viewer per table tab (step 86.4, item M102):
    /// the displayed (row, col) the zoom window addresses.
    pub(super) value_viewer: HashMap<TabId, (usize, usize)>,
    /// The value viewer's multiline editor draft per tab (step 86.4,
    /// item M102); absent while the window shows read-only content.
    pub(super) value_editor: HashMap<TabId, String>,
    /// Open database-password prompt (step 16.2, M5; the state and paint
    /// live in `password_prompt.rs`).
    pub(crate) password_prompt: Option<crate::password_prompt::PasswordPrompt>,
    /// VBA viewer search text (module list + source full-text filter,
    /// step 17.2).
    pub vba_search: String,
    /// Draft of the relationship-create form while the mapper pane's
    /// form is open (`None` hides it, step 41.1, item M45).
    pub(crate) relationship_draft: Option<crate::relationships::RelationshipDraft>,
    /// Cached static VBA analysis (the viewer's right pane renders it,
    /// step 41.3, item M47).
    pub(crate) vba_analysis: Option<datacraft_core::vba_analysis::VbaAnalysis>,
    /// Reason of the last failed analysis run (the viewer renders it).
    pub(crate) vba_analysis_error: Option<String>,
    /// Cached readiness report (the readiness window renders it, phase
    /// 103, step 103.2; item M134).
    pub(crate) readiness_report: Option<datacraft_core::readiness::AppReadiness>,
    /// Reason of the last failed readiness run (the window renders it).
    pub(crate) readiness_error: Option<String>,
    /// UI-owned settings (ADR-5); persisted only when a path is attached.
    pub(crate) settings: crate::settings::Settings,
    /// Path of `ui.json`; `None` keeps settings session-only (tests).
    pub(crate) settings_path: Option<std::path::PathBuf>,
    /// Whether the restored window position was already checked (step 22.2).
    pub(crate) position_rescued: bool,
    /// Localized load failures per tab (step 39.4, item M31): rendered in
    /// place of the endless loading state, cleared on the next successful
    /// load pass.
    pub(crate) load_errors: HashMap<TabId, String>,
    /// The dialog seam: platform dialogs in the binary, scripted in the
    /// headless suite (step 38.3, item M43); the queued model answers on
    /// a later frame (step 81.1, item M97).
    pub(crate) dialogs: Box<dyn crate::dialogs::DialogQueue>,
    /// The clipboard seam (step 86.2, item M101): the system clipboard
    /// in the binary, a scripted one in the headless suite.
    pub(super) clipboard: Box<dyn crate::clipboard::ClipboardPort>,
    /// The parked follow-up of the outstanding dialog request (step
    /// 81.2, item M97); `None` while no dialog is open.
    pub(super) pending_dialog: Option<super::dialog_flow::DialogIntent>,
    /// Per-tab datasheet edit buffer (step 85.2, item H61): staged
    /// updates, inserts and deletes applied as one journal batch. The
    /// vocabulary and its accessors live in `app/edit_batch.rs`.
    pub(super) edit_buffers: HashMap<TabId, datacraft_core::edit_buffer::EditBuffer>,
    /// Buffer row (slot) the tab's record cursor addresses — sort and
    /// filter make display indices differ from slots, so the datasheet
    /// stores the resolved slot every frame (the Escape path and the
    /// staged intents address it).
    pub(super) cursor_slots: HashMap<TabId, usize>,
    /// The open submission of a tab: the journal's own changes between
    /// `PreviewWrite` and its apply/discard (step 85.2, item H61).
    pub(super) submission: Option<super::edit_batch::Submission>,
    /// Per-tab foreign-key navigation history (step 87.2, item M103):
    /// the follow/back/forward chain, carried by the tab it navigates
    /// from; vocabulary and accessors live in `app/nav_history.rs`.
    pub(super) fk_histories: HashMap<TabId, super::nav_history::NavHistory>,
    /// Answered referenced-by counts keyed by child table and criteria
    /// (step 87.2, item M103); cleared with the tab.
    pub(super) count_results: HashMap<String, super::nav_history::ReferencedCount>,
    /// Bounded reads waiting for their frame slot (step 87.2, M103 —
    /// at most one processes per frame).
    pub(super) count_pending: Vec<super::nav_history::CountRequest>,
    /// The tab whose follow context menu is open (step 87.2, item M103):
    /// the frame-local count context clones only while this holds.
    pub(super) count_menu_tab: Option<TabId>,
    /// The filter builder's per-tab draft (step 87.4, item M104):
    /// present while the window is open; vocabulary lives in
    /// `app/filter_builder.rs`.
    pub(super) filter_builder_drafts: HashMap<TabId, super::filter_builder::FilterBuilderDraft>,
}
