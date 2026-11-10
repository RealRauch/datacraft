//! Source specifications for the open surface (step 65.1, item L8; ADR-7).
//!
//! The driver model is ready for server drivers, but the engine serves
//! local files today: [`SourceSpec`] carries both worlds. Resolution is
//! precise and locatable — a `file://` URI opens exactly like its path
//! equivalent, and every other scheme answers the documented
//! no-server-driver boundary instead of failing mysteriously at the port.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::failure::FailureCode;

/// Where a database session comes from (step 65.1, item L8; ADR-7).
///
/// The open command's plain `path` field keeps working — the spec is
/// authoritative only when present ([`effective_path`]). The first actual
/// server driver decides credentials/SSL and stays synchronous behind the
/// no-async rule (ADR-7); nothing here pre-decides that.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SourceSpec {
    /// A local filesystem path — the established open surface.
    Path(PathBuf),
    /// A URI. `file://` URIs (authority empty or `localhost`, RFC 8089
    /// drive forms) resolve to the local path and open exactly like it;
    /// every other scheme answers the no-server-driver boundary.
    Uri(String),
}

/// Precise, locatable source-resolution failures (step 65.1, item L8).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SourceError {
    /// The spec carries a path that disagrees with the command's `path`.
    PathMismatch {
        /// The path the command carried.
        command: PathBuf,
        /// The path the source spec carried.
        source: PathBuf,
    },
    /// The URI carries no `scheme:` at all.
    MissingScheme(String),
    /// The `file://` URI is malformed — the `reason` names the exact
    /// defect (authority form, remote host, query/fragment parts, broken
    /// percent escapes, empty path).
    InvalidFileUri {
        /// The rejected URI.
        uri: String,
        /// The precise defect.
        reason: &'static str,
    },
    /// A non-file scheme: no server driver exists for it — server
    /// sources are a documented future path (ADR-7).
    NoServerDriver {
        /// The scheme that has no driver.
        scheme: String,
    },
}

impl SourceError {
    /// The failure class the refusal rides (step 62.1, I6): malformed
    /// request payloads are `InvalidInput`; a well-formed scheme the
    /// engine cannot serve is `Unsupported`.
    pub fn code(&self) -> FailureCode {
        match self {
            Self::NoServerDriver { .. } => FailureCode::Unsupported,
            _ => FailureCode::InvalidInput,
        }
    }
}

impl std::fmt::Display for SourceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PathMismatch { command, source } => write!(
                f,
                "source path {:?} disagrees with the open path {:?} — the source spec is authoritative; make them agree or drop the path field",
                source, command
            ),
            Self::MissingScheme(uri) => write!(
                f,
                "uri {uri:?} carries no scheme — file and server sources need one (file://, postgres://, …)"
            ),
            Self::InvalidFileUri { uri, reason } => {
                write!(f, "invalid file uri {uri:?}: {reason}")
            }
            Self::NoServerDriver { scheme } => write!(
                f,
                "uri scheme {scheme:?} has no server driver — server sources are a documented future path (ADR-7); open a local file instead"
            ),
        }
    }
}

impl std::fmt::Display for SourceSpec {
    /// The source display every client renders (step 65.2/65.3, item L8):
    /// a `Uri` shows as entered, a `Path` shows its filesystem form.
    /// Future source variants decide their own display here — the single
    /// render point (the `DatabaseFormat` pattern, M18).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Path(path) => write!(f, "{}", path.display()),
            Self::Uri(uri) => write!(f, "{uri}"),
        }
    }
}

impl SourceSpec {
    /// Resolves the open request into the effective local path.
    ///
    /// `Path` must agree with the command's `path` (both present is only
    /// meaningful when identical); `Uri` resolves the `file://` forms of
    /// [`Self::Uri`] and refuses everything else.
    pub fn resolve(&self, command_path: &Path) -> Result<PathBuf, SourceError> {
        match self {
            Self::Path(path) => {
                if command_path.as_os_str().is_empty() || command_path == path {
                    Ok(path.clone())
                } else {
                    Err(SourceError::PathMismatch {
                        command: command_path.to_path_buf(),
                        source: path.clone(),
                    })
                }
            }
            Self::Uri(uri) => resolve_uri(uri),
        }
    }
}

/// The effective local path of an open request (step 65.1, item L8): a
/// present spec is authoritative, an absent one keeps the plain path —
/// the historical behavior, byte-identical.
pub fn effective_path(
    command_path: &Path,
    source: Option<&SourceSpec>,
) -> Result<PathBuf, SourceError> {
    match source {
        None => Ok(command_path.to_path_buf()),
        Some(spec) => spec.resolve(command_path),
    }
}

/// Parses a `file://` URI into its local path (RFC 8089 subset): the
/// authority must be empty or `localhost`, the path portion uses `/`,
/// rejects query/fragment parts, decodes `%HH` escapes and maps the
/// Windows drive form (`file:///C:/dir`) to the plain path.
fn resolve_uri(uri: &str) -> Result<PathBuf, SourceError> {
    let Some((raw_scheme, rest)) = uri.split_once(':') else {
        return Err(SourceError::MissingScheme(uri.to_owned()));
    };
    // URI schemes are case-insensitive (RFC 3986 §3.1).
    let scheme = raw_scheme.to_ascii_lowercase();
    if !valid_scheme(&scheme) || scheme != "file" {
        return Err(SourceError::NoServerDriver { scheme });
    }
    let Some(after_authority) = rest.strip_prefix("//") else {
        return Err(SourceError::InvalidFileUri {
            uri: uri.to_owned(),
            reason: "the file scheme requires the authority form file://…",
        });
    };
    let (authority, raw_path) = match after_authority.split_once('/') {
        Some((authority, path)) => (authority, path),
        // `file://localhost` with no path at all.
        None => (after_authority, ""),
    };
    if !authority.is_empty() && !authority.eq_ignore_ascii_case("localhost") {
        return Err(SourceError::InvalidFileUri {
            uri: uri.to_owned(),
            reason: "remote file hosts are not supported — only local files open today",
        });
    }
    if raw_path.contains(['?', '#']) {
        return Err(SourceError::InvalidFileUri {
            uri: uri.to_owned(),
            reason: "query and fragment parts are not database paths",
        });
    }
    if raw_path.is_empty() {
        return Err(SourceError::InvalidFileUri {
            uri: uri.to_owned(),
            reason: "the file uri carries no path",
        });
    }
    let decoded = percent_decode(raw_path).map_err(|reason| SourceError::InvalidFileUri {
        uri: uri.to_owned(),
        reason,
    })?;
    // Reconstruct the absolute path the authority split consumed
    // (`file:///dir` and `file://localhost/dir` are both `/dir`), then
    // map the RFC 8089 Windows drive form (`/C:/dir` → `C:/dir`).
    let mut candidate = format!("/{decoded}");
    let bytes = candidate.as_bytes();
    if bytes.len() >= 3 && bytes[0] == b'/' && bytes[1].is_ascii_alphabetic() && bytes[2] == b':' {
        candidate.remove(0);
    }
    Ok(PathBuf::from(candidate))
}

/// A URI scheme is `alpha *( alnum / "+" / "-" / "." )` (RFC 3986 §3.1).
fn valid_scheme(scheme: &str) -> bool {
    let mut chars = scheme.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'))
}

/// Decodes `%HH` escapes; anything malformed is a precise refusal.
fn percent_decode(raw: &str) -> Result<String, &'static str> {
    let bytes = raw.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len()
                || !bytes[i + 1].is_ascii_hexdigit()
                || !bytes[i + 2].is_ascii_hexdigit()
            {
                return Err("invalid percent escape (expected %HH)");
            }
            let hex = &raw[i + 1..i + 3];
            match u8::from_str_radix(hex, 16) {
                Ok(byte) => out.push(byte),
                Err(_) => return Err("invalid percent escape (expected %HH)"),
            }
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).map_err(|_| "the percent escape decodes to invalid utf-8")
}
