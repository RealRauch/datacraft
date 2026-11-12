//! Views that live in the opened file itself (phase 73, step 73.1; item
//! M90).
//!
//! A foreign view is not a DataCraft design: the file owns its SQL, the
//! port only lists it, reads its rows and feeds it to the SQL path.
//! Writes answer a precise `Unsupported`; the sidecar view designs stay
//! the writable authority (ADR-1, M93).

use serde::{Deserialize, Serialize};

use crate::schema::FieldDef;

/// One native (file-owned) view of a data source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NativeView {
    /// View name as the file reports it.
    pub name: String,
    /// The source statement verbatim (`CREATE VIEW …` / the stored SQL).
    pub sql: String,
    /// Columns in select order (from the file's column metadata).
    pub fields: Vec<FieldDef>,
}
