//! The command language (step 4.1, item H13; ARCHITECTURE §5).
//!
//! Every user-visible action — GUI or agent — is exactly one command. The
//! set grows with the phases that introduce the features (QBE commands in
//! Phase 7, form commands in Phase 8); the dispatch mechanism is fixed
//! here.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ObjectId;
use crate::ids::TabId;
use crate::qbe::{QbeRow, QbeValue};
use crate::schema::TableDef;

/// One user-visible action.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Command {
    /// Opens the database file at `path` (closes nothing implicitly —
    /// rejected while another database is open).
    OpenDatabase {
        /// Filesystem path of the `.mdb`/`.accdb` file.
        path: PathBuf,
    },
    /// Closes the open database and all of its tabs.
    CloseDatabase,
    /// Creates a table from a validated definition.
    CreateTable {
        /// Full field-level definition.
        definition: TableDef,
    },
    /// Drops a table (name matched case-insensitively); dependent tabs
    /// close first.
    DropTable {
        /// Table name as shown in the navigation pane.
        table: String,
    },
    /// Opens the object in a tab (re-activates an existing tab instead of
    /// opening a second one for the same object).
    OpenObject(ObjectId),
    /// Closes one document tab; closing the active tab activates its
    /// next-lower neighbor.
    CloseTab {
        /// The tab to close.
        tab_id: TabId,
    },
    /// Makes an existing tab the active one.
    SetActiveTab {
        /// The tab to activate.
        tab_id: TabId,
    },
    /// Shows or hides the navigation pane.
    ToggleNavPane,
    /// Shows or hides the relationship mapper (Access opens it as a
    /// document; the headless core models it as a pane flag).
    ToggleRelationshipMapper,
    /// Creates an empty query (name unique, case-insensitive).
    CreateQuery {
        /// Query name; rejected on duplicates.
        name: String,
    },
    /// Edits one QBE grid cell; identifiers are whitelisted against the
    /// schema (LESSONS #5).
    SetQbeCell {
        /// Target query name (case-insensitive).
        query: String,
        /// Grid row.
        row: QbeRow,
        /// Grid column index.
        col: usize,
        /// New cell value.
        value: QbeValue,
    },
    /// Deletes a query (dependent tabs close first).
    DeleteQuery {
        /// Query name (case-insensitive).
        name: String,
    },
    /// Executes a query; the row count is observable, the data itself is
    /// fetched via `Core::run_query`.
    RunQuery {
        /// Query name (case-insensitive).
        query: String,
    },
    /// Creates an empty form (name unique, case-insensitive).
    CreateForm {
        /// Form name; rejected on duplicates.
        name: String,
    },
    /// Deletes a form.
    DeleteForm {
        /// Form name (case-insensitive).
        name: String,
    },
}
