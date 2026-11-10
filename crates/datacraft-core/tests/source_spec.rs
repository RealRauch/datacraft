//! Source-spec resolution contract (step 65.1, item L8): `SourceSpec` on
//! the open command — `file://` URIs resolve to the plain file path with
//! identical behavior, every other scheme answers the precise
//! no-server-driver boundary (ADR-7), and the historical wire shape keeps
//! working. MockPort seam; the real-stack pins ride the wire suites.

use std::path::PathBuf;

use datacraft_core::command::{Command, SourceSpec};
use datacraft_core::core::Core;
use datacraft_core::event::Event;
use datacraft_core::failure::FailureCode;
use datacraft_core::port::{DatabaseFormat, MockPort};

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn core() -> Core {
    Core::new(Box::new(MockPort::new()))
}

fn open(core: &mut Core, path: &str, source: Option<SourceSpec>) -> Vec<Event> {
    core.dispatch(Command::OpenDatabase {
        path: PathBuf::from(path),
        password: None,
        source,
    })
}

fn failure_code(events: &[Event]) -> FailureCode {
    match events {
        [Event::Failed { code, .. }] => *code,
        other => panic!("expected a single Failed event, got {other:?}"),
    }
}

fn failed_reason(events: &[Event]) -> String {
    match events {
        [Event::Failed { reason, .. }] => reason.clone(),
        other => panic!("expected a single Failed event, got {other:?}"),
    }
}

/// Absolute plain path and its `file://` URI for the host platform (the
/// Windows drive form `file:///C:/dir` strips its leading slash).
fn fixture_paths(dir: &str, file: &str) -> (String, String) {
    if cfg!(windows) {
        (
            format!("C:/{dir}/{file}"),
            format!("file:///C:/{dir}/{file}"),
        )
    } else {
        (format!("/{dir}/{file}"), format!("file:///{dir}/{file}"))
    }
}

fn opened_path(core: &Core) -> PathBuf {
    core.inspect().database.expect("database must be open").path
}

// ---------------------------------------------------------------------------
// opens
// ---------------------------------------------------------------------------

#[test]
fn plain_path_open_is_unchanged_without_a_source_spec() {
    let mut core = core();
    let events = open(&mut core, "shop.mdb", None);
    assert!(
        matches!(
            events.as_slice(),
            [Event::DatabaseOpened {
                name,
                format: DatabaseFormat::Jet4
            }] if name == "shop"
        ),
        "{events:?}"
    );
    assert_eq!(opened_path(&core), PathBuf::from("shop.mdb"));
}

#[test]
fn file_uri_open_resolves_to_the_plain_path_behavior() {
    let (path, uri) = fixture_paths("dir", "shop.mdb");
    let plain = {
        let mut core = core();
        let events = open(&mut core, &path, None);
        assert!(
            matches!(events.as_slice(), [Event::DatabaseOpened { .. }]),
            "{events:?}"
        );
        opened_path(&core)
    };
    let mut core = core();
    let events = open(&mut core, &path, Some(SourceSpec::Uri(uri)));
    assert!(
        matches!(events.as_slice(), [Event::DatabaseOpened { .. }]),
        "the uri open must behave like the plain path open: {events:?}"
    );
    assert_eq!(opened_path(&core), plain);
}

#[test]
fn localhost_file_uri_resolves_like_the_authority_free_form() {
    let (path, uri) = fixture_paths("dir", "shop.mdb");
    let localhost_uri = uri.replacen("file:///", "file://localhost/", 1);
    let mut core = core();
    let events = open(&mut core, &path, Some(SourceSpec::Uri(localhost_uri)));
    assert!(
        matches!(events.as_slice(), [Event::DatabaseOpened { .. }]),
        "{events:?}"
    );
    assert_eq!(opened_path(&core), PathBuf::from(&path));
}

#[test]
fn percent_encoded_spaces_resolve_to_the_literal_path() {
    let (path, uri) = fixture_paths("my dir", "shop.mdb");
    let encoded_uri = uri.replace(' ', "%20");
    let mut core = core();
    let events = open(&mut core, &path, Some(SourceSpec::Uri(encoded_uri)));
    assert!(
        matches!(events.as_slice(), [Event::DatabaseOpened { .. }]),
        "{events:?}"
    );
    assert_eq!(opened_path(&core), PathBuf::from(&path));
}

#[test]
fn a_path_spec_matching_the_command_path_opens() {
    let mut core = core();
    let events = open(
        &mut core,
        "shop.mdb",
        Some(SourceSpec::Path(PathBuf::from("shop.mdb"))),
    );
    assert!(
        matches!(events.as_slice(), [Event::DatabaseOpened { .. }]),
        "{events:?}"
    );
}

// ---------------------------------------------------------------------------
// precise refusals
// ---------------------------------------------------------------------------

#[test]
fn a_path_spec_disagreeing_with_the_command_path_is_refused_precisely() {
    let mut core = core();
    let events = open(
        &mut core,
        "shop.mdb",
        Some(SourceSpec::Path(PathBuf::from("other.mdb"))),
    );
    assert_eq!(failure_code(&events), FailureCode::InvalidInput);
    let reason = failed_reason(&events);
    assert!(
        reason.contains("shop.mdb") && reason.contains("other.mdb"),
        "the reason must name both paths: {reason}"
    );
    assert!(core.inspect().database.is_none());
}

#[test]
fn a_uri_without_a_scheme_is_refused_precisely() {
    let mut core = core();
    let events = open(
        &mut core,
        "shop.mdb",
        Some(SourceSpec::Uri("shop.mdb".into())),
    );
    assert_eq!(failure_code(&events), FailureCode::InvalidInput);
    assert!(
        failed_reason(&events).contains("no scheme"),
        "the reason must name the missing scheme: {events:?}"
    );
    assert!(core.inspect().database.is_none());
}

#[test]
fn a_postgres_uri_answers_the_no_server_driver_boundary() {
    let mut core = core();
    let events = open(
        &mut core,
        "shop.mdb",
        Some(SourceSpec::Uri("postgres://db.example.com/shop".into())),
    );
    assert_eq!(failure_code(&events), FailureCode::Unsupported);
    let reason = failed_reason(&events);
    assert!(
        reason.contains("postgres") && reason.contains("ADR-7"),
        "the reason must name the scheme and the ADR-7 boundary: {reason}"
    );
    assert!(core.inspect().database.is_none());
}

#[test]
fn a_sqlserver_uri_answers_the_same_boundary() {
    let mut core = core();
    let events = open(
        &mut core,
        "shop.mdb",
        Some(SourceSpec::Uri("sqlserver://localhost/shop".into())),
    );
    assert_eq!(failure_code(&events), FailureCode::Unsupported);
    assert!(
        failed_reason(&events).contains("sqlserver"),
        "the reason must name the scheme: {events:?}"
    );
}

#[test]
fn a_windows_drive_in_the_uri_field_answers_the_driver_boundary() {
    // A bare Windows path carries `C:` — syntactically the scheme `c` —
    // and must answer the driver boundary instead of being silently
    // reinterpreted as a path.
    let mut core = core();
    let events = open(
        &mut core,
        "shop.mdb",
        Some(SourceSpec::Uri(r"C:\dir\shop.mdb".into())),
    );
    assert_eq!(failure_code(&events), FailureCode::Unsupported);
    assert!(
        failed_reason(&events).contains("no server driver"),
        "the reason must carry the boundary: {events:?}"
    );
}

#[test]
fn a_remote_file_host_uri_is_refused() {
    let mut core = core();
    let events = open(
        &mut core,
        "shop.mdb",
        Some(SourceSpec::Uri("file://fileserver/share/shop.mdb".into())),
    );
    assert_eq!(failure_code(&events), FailureCode::InvalidInput);
    assert!(
        failed_reason(&events).contains("remote file host"),
        "the reason must name the remote host: {events:?}"
    );
}

#[test]
fn a_file_uri_with_a_query_part_is_refused() {
    let (_, uri) = fixture_paths("dir", "shop.mdb");
    let mut core = core();
    let events = open(
        &mut core,
        "shop.mdb",
        Some(SourceSpec::Uri(format!("{uri}?raw=1"))),
    );
    assert_eq!(failure_code(&events), FailureCode::InvalidInput);
    assert!(
        failed_reason(&events).contains("query"),
        "the reason must name the query part: {events:?}"
    );
}

#[test]
fn a_file_uri_with_a_fragment_is_refused() {
    let (_, uri) = fixture_paths("dir", "shop.mdb");
    let mut core = core();
    let events = open(
        &mut core,
        "shop.mdb",
        Some(SourceSpec::Uri(format!("{uri}#anchor"))),
    );
    assert_eq!(failure_code(&events), FailureCode::InvalidInput);
    assert!(
        failed_reason(&events).contains("fragment"),
        "the reason must name the fragment: {events:?}"
    );
}

#[test]
fn a_file_uri_with_a_truncated_percent_escape_is_refused() {
    let (_, uri) = fixture_paths("dir", "shop.mdb");
    let mut core = core();
    let events = open(
        &mut core,
        "shop.mdb",
        Some(SourceSpec::Uri(format!("{uri}%2"))),
    );
    assert_eq!(failure_code(&events), FailureCode::InvalidInput);
    assert!(
        failed_reason(&events).contains("percent"),
        "the reason must name the escape: {events:?}"
    );
}

#[test]
fn a_file_uri_with_an_invalid_hex_escape_is_refused() {
    let (_, uri) = fixture_paths("dir", "shop.mdb");
    let mut core = core();
    let events = open(
        &mut core,
        "shop.mdb",
        Some(SourceSpec::Uri(format!("{uri}%ZZ"))),
    );
    assert_eq!(failure_code(&events), FailureCode::InvalidInput);
    assert!(
        failed_reason(&events).contains("percent"),
        "the reason must name the escape: {events:?}"
    );
}

#[test]
fn a_file_uri_without_the_authority_form_is_refused() {
    let mut core = core();
    let events = open(
        &mut core,
        "shop.mdb",
        Some(SourceSpec::Uri("file:C:/dir/shop.mdb".into())),
    );
    assert_eq!(failure_code(&events), FailureCode::InvalidInput);
    assert!(
        failed_reason(&events).contains("file://"),
        "the reason must name the required authority form: {events:?}"
    );
}

#[test]
fn a_uri_with_an_invalid_scheme_answers_the_driver_boundary() {
    // `1http://…` cannot be a scheme (RFC 3986 §3.1: alphabetic first
    // byte) — still answered at the boundary, never a panic.
    let mut core = core();
    let events = open(
        &mut core,
        "shop.mdb",
        Some(SourceSpec::Uri("1http://example.com/x".into())),
    );
    assert_eq!(failure_code(&events), FailureCode::Unsupported);
    assert!(
        failed_reason(&events).contains("no server driver"),
        "the reason must carry the boundary: {events:?}"
    );
}

#[test]
fn a_percent_escape_decoding_to_invalid_utf8_is_refused() {
    let (_, uri) = fixture_paths("dir", "shop.mdb");
    let mut core = core();
    let events = open(
        &mut core,
        "shop.mdb",
        Some(SourceSpec::Uri(format!("{uri}%FF"))),
    );
    assert_eq!(failure_code(&events), FailureCode::InvalidInput);
    assert!(
        failed_reason(&events).contains("utf-8"),
        "the reason must name the decoding failure: {events:?}"
    );
}

#[test]
fn a_file_uri_with_an_authority_but_no_path_is_refused() {
    let mut core = core();
    let events = open(
        &mut core,
        "shop.mdb",
        Some(SourceSpec::Uri("file://localhost".into())),
    );
    assert_eq!(failure_code(&events), FailureCode::InvalidInput);
    assert!(
        failed_reason(&events).contains("no path"),
        "the reason must name the empty path: {events:?}"
    );
}

#[test]
fn the_lifecycle_gate_precedes_source_resolution() {
    let mut core = core();
    assert!(matches!(
        open(&mut core, "shop.mdb", None).as_slice(),
        [Event::DatabaseOpened { .. }]
    ));
    let events = open(
        &mut core,
        "other.mdb",
        Some(SourceSpec::Uri("postgres://db.example.com/shop".into())),
    );
    assert_eq!(failure_code(&events), FailureCode::Lifecycle);
}

// ---------------------------------------------------------------------------
// wire compatibility
// ---------------------------------------------------------------------------

#[test]
fn the_historical_wire_shape_without_source_still_deserializes() {
    let command: Command =
        serde_json::from_str(r#"{"OpenDatabase":{"path":"shop.mdb","password":null}}"#)
            .expect("old clients keep working");
    match command {
        Command::OpenDatabase {
            path,
            password,
            source,
        } => {
            assert_eq!(path, PathBuf::from("shop.mdb"));
            assert_eq!(password, None);
            assert_eq!(source, None);
        }
        other => panic!("unexpected command {other:?}"),
    }
}

#[test]
fn a_uri_source_round_trips_through_the_wire() {
    let command = Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: Some(SourceSpec::Uri("file:///C:/dir/shop.mdb".into())),
    };
    let raw = serde_json::to_string(&command).expect("serialize");
    let back: Command = serde_json::from_str(&raw).expect("deserialize");
    assert_eq!(back, command);
}

#[test]
fn a_path_source_round_trips_through_the_wire() {
    let command = Command::OpenDatabase {
        path: PathBuf::from("shop.mdb"),
        password: None,
        source: Some(SourceSpec::Path(PathBuf::from("shop.mdb"))),
    };
    let raw = serde_json::to_string(&command).expect("serialize");
    let back: Command = serde_json::from_str(&raw).expect("deserialize");
    assert_eq!(back, command);
}
