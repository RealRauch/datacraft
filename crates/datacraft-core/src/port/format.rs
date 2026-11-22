//! Source family and SQL dialect vocabulary (ADR-7; phase 98 / M129).
//!
//! Split from `port.rs` for the file-size gate: both enums are wire
//! vocabulary, rendered by one Display point in `present.rs`.

use serde::{Deserialize, Serialize};

/// Which SQL surface `RunSql` uses for the open source (phase 98, item
/// M129). The snapshot carries it so every client names the dialect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SqlDialect {
    /// The driver's own engine (SQLite): full dialect, real indexes, native
    /// views and `EXPLAIN QUERY PLAN`.
    Native,
    /// The materialized shadow engine (Jet/ACE/CSV): SQLite semantics over
    /// the rows read through the port.
    Shadow,
}

/// Binary format family of an opened database (domain vocabulary; the
/// backends map their physical version detection onto it).
///
/// `#[non_exhaustive]` (M32): ADR-7's server path adds format families;
/// clients match with a wildcard arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DatabaseFormat {
    /// Jet 3.x (Access 97 and earlier).
    Jet3,
    /// Jet 4.x (Access 2000–2003).
    Jet4,
    /// ACE (Access 2007 and later, `.accdb`).
    Ace,
    /// SQLite 3 (`.sqlite`, `.db`) — the non-Jet source family (H35).
    Sqlite,
    /// A directory of `*.csv` files (phase 30, M18; ADR-7 driver proof).
    Csv,
    /// An Excel workbook (`.xlsx`, `.xlsm`, `.xls`, `.xlsb`) read as a
    /// database of sheets (phase 96, M127; roadmap §7). Read-only.
    Excel,
}

impl DatabaseFormat {
    /// The canonical extension of a family this build can create — without
    /// the dot (step 75.3, item L43). The creation pickers suggest it and
    /// the create commands compare against it.
    pub fn create_extension(self) -> Option<&'static str> {
        match self {
            Self::Jet4 => Some("mdb"),
            Self::Sqlite => Some("sqlite"),
            Self::Jet3 | Self::Ace | Self::Csv | Self::Excel => None,
        }
    }

    /// The suffix conflict of a creation path (step 75.3, item L43): the
    /// extension of the *other* creatable family, compared
    /// case-insensitively. `None` accepts matching, unknown and missing
    /// extensions — the create command carries the format explicitly, so
    /// only a known extension of the other family is a contradiction.
    pub fn creation_suffix_conflict(self, path: &std::path::Path) -> Option<&'static str> {
        let extension = path.extension()?.to_str()?;
        [Self::Jet4, Self::Sqlite]
            .into_iter()
            .filter(|family| *family != self)
            .filter_map(Self::create_extension)
            .find(|candidate| extension.eq_ignore_ascii_case(candidate))
    }
}
