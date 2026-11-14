//! Import failures with locatable causes.

use std::fmt;

/// Why an import failed.
#[derive(Debug)]
pub enum ImportError {
    /// The database could not be opened or decoded by jetdb.
    Jetdb(jetdb::FileError),
    /// Reading the sidecar document failed.
    Sidecar(datacraft_core::schema::SchemaError),
    /// Reading or writing a file failed.
    Io(std::io::Error),
}

impl fmt::Display for ImportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Jetdb(error) => write!(f, "reading the Access database failed: {error}"),
            Self::Sidecar(error) => write!(f, "reading the sidecar failed: {error}"),
            Self::Io(error) => write!(f, "file operation failed: {error}"),
        }
    }
}

impl std::error::Error for ImportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Jetdb(error) => Some(error),
            Self::Sidecar(error) => Some(error),
            Self::Io(error) => Some(error),
        }
    }
}

impl From<jetdb::FileError> for ImportError {
    fn from(error: jetdb::FileError) -> Self {
        Self::Jetdb(error)
    }
}

impl From<datacraft_core::schema::SchemaError> for ImportError {
    fn from(error: datacraft_core::schema::SchemaError) -> Self {
        Self::Sidecar(error)
    }
}

impl From<std::io::Error> for ImportError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}
