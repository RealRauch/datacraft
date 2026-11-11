//! Editor buffer accessors (step 68.3, item M76; split from `app.rs` to
//! keep the file below the size gate — L30).
//!
//! The SQL and expression windows keep their text and last outcome as
//! presentation state on `App`; these accessors are the crate-facing seam.

use crate::app::App;

impl App {
    /// SQL window editor text.
    pub fn sql_text(&self) -> &str {
        &self.sql_text
    }

    /// Replaces the SQL editor text.
    pub fn set_sql_text(&mut self, text: impl Into<String>) {
        self.sql_text = text.into();
    }

    /// Expression window editor text.
    pub fn expression_text(&self) -> &str {
        &self.expression_text
    }

    /// Replaces the expression editor text.
    pub fn set_expression_text(&mut self, text: impl Into<String>) {
        self.expression_text = text.into();
    }

    /// Display form of the last successful expression run.
    pub fn expression_output(&self) -> Option<&str> {
        self.expression_output.as_deref()
    }

    /// Precise reason of the last failed expression run.
    pub fn expression_error(&self) -> Option<&str> {
        self.expression_error.as_deref()
    }

    /// Precise reason of the last failed SQL run.
    pub fn sql_error(&self) -> Option<&str> {
        self.sql_error.as_deref()
    }
}
