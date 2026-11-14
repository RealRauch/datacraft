//! Command-line front end of the import tool.
//!
//! Contract: `docs/M88-IMPORT.md` §7. Exit codes follow the DataCraft CLI
//! convention: 0 success, 1 failure or `--strict` with unsupported entries,
//! 2 usage error.

use std::io::Write;
use std::path::PathBuf;

use datacraft_core::schema::SchemaDocument;

use crate::{ImportError, ImportReport, has_unsupported, import_database, merge_documents};

/// Usage line printed on `--help` and usage errors.
pub const USAGE: &str = "usage: datacraft-import <database> [--sidecar <path>] [--strict] [--json]";

#[derive(Debug, PartialEq, Eq)]
struct Options {
    database: PathBuf,
    sidecar: Option<PathBuf>,
    strict: bool,
    json: bool,
}

/// Runs the command line; returns the process exit code.
pub fn run(
    args: impl IntoIterator<Item = String>,
    out: &mut dyn Write,
    err: &mut dyn Write,
) -> i32 {
    let args: Vec<String> = args.into_iter().collect();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        let _ = writeln!(out, "{USAGE}");
        return 0;
    }
    if args.iter().any(|arg| arg == "-V" || arg == "--version") {
        let _ = writeln!(out, "datacraft-import {}", env!("CARGO_PKG_VERSION"));
        return 0;
    }
    let options = match parse(&args) {
        Ok(options) => options,
        Err(message) => {
            let _ = writeln!(err, "error: {message}");
            let _ = writeln!(err, "{USAGE}");
            return 2;
        }
    };
    match execute(&options, out) {
        Ok(0) => 0,
        Ok(code) => {
            if options.strict {
                let _ = writeln!(err, "strict: unsupported entries present");
            }
            code
        }
        Err(error) => {
            let _ = writeln!(err, "error: {error}");
            1
        }
    }
}

fn parse(args: &[String]) -> Result<Options, String> {
    let mut database: Option<PathBuf> = None;
    let mut sidecar: Option<PathBuf> = None;
    let mut strict = false;
    let mut json = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--sidecar" => {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--sidecar needs a path".to_owned())?;
                sidecar = Some(PathBuf::from(value));
                index += 2;
            }
            "--strict" => {
                strict = true;
                index += 1;
            }
            "--json" => {
                json = true;
                index += 1;
            }
            flag if flag.starts_with('-') => {
                return Err(format!("unknown option {flag:?}"));
            }
            path => {
                if database.is_some() {
                    return Err(format!("unexpected argument {path:?}"));
                }
                database = Some(PathBuf::from(path));
                index += 1;
            }
        }
    }
    let database = database.ok_or_else(|| "missing the database path".to_owned())?;
    Ok(Options {
        database,
        sidecar,
        strict,
        json,
    })
}

#[derive(serde::Serialize)]
struct JsonOutput<'a> {
    sidecar: String,
    report: &'a ImportReport,
}

fn execute(options: &Options, out: &mut dyn Write) -> Result<i32, ImportError> {
    let import = import_database(&options.database)?;
    let sidecar = options
        .sidecar
        .clone()
        .unwrap_or_else(|| SchemaDocument::sidecar_path(&options.database));
    let document = if sidecar.exists() {
        let mut existing = SchemaDocument::load(&sidecar)?;
        merge_documents(&mut existing, import.document);
        existing
    } else {
        import.document
    };
    document.save(&sidecar)?;

    if options.json {
        let output = JsonOutput {
            sidecar: sidecar.display().to_string(),
            report: &import.report,
        };
        let _ = writeln!(
            out,
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    } else {
        report_summary(&import.report, &sidecar, out);
    }
    if options.strict && has_unsupported(&import.report) {
        return Ok(1);
    }
    Ok(0)
}

fn report_summary(report: &ImportReport, sidecar: &std::path::Path, out: &mut dyn Write) {
    let forms = report
        .objects
        .iter()
        .filter(|object| object.kind == "form")
        .count();
    let reports = report.objects.len() - forms;
    let controls: usize = report
        .objects
        .iter()
        .map(|object| object.imported_controls)
        .sum();
    let _ = writeln!(
        out,
        "imported {} objects ({forms} forms, {reports} reports), {controls} controls",
        report.objects.len()
    );
    for object in &report.objects {
        let skipped = object.unsupported_controls.len() + object.unsupported_properties.len();
        if skipped > 0 {
            let _ = writeln!(
                out,
                "warning: {}: {} unsupported controls, {} unsupported properties",
                object.object,
                object.unsupported_controls.len(),
                object.unsupported_properties.len()
            );
        }
    }
    let _ = writeln!(out, "wrote {}", sidecar.display());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    #[test]
    fn parse_reads_the_documented_flags() {
        let options = parse(&args(&[
            "db.accdb",
            "--sidecar",
            "out.datacraft",
            "--strict",
            "--json",
        ]))
        .expect("parses");
        assert_eq!(
            options,
            Options {
                database: PathBuf::from("db.accdb"),
                sidecar: Some(PathBuf::from("out.datacraft")),
                strict: true,
                json: true,
            }
        );
    }

    #[test]
    fn parse_rejects_usage_errors() {
        assert!(parse(&[]).is_err(), "missing database");
        assert!(
            parse(&args(&["a", "b"])).is_err(),
            "two positional arguments"
        );
        assert!(
            parse(&args(&["a", "--sidecar"])).is_err(),
            "missing sidecar value"
        );
        assert!(parse(&args(&["a", "--unknown"])).is_err(), "unknown option");
    }

    #[test]
    fn help_and_version_exit_zero() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(run(args(&["--help"]), &mut out, &mut err), 0);
        assert!(String::from_utf8_lossy(&out).contains("usage: datacraft-import"));
        let mut out = Vec::new();
        assert_eq!(run(args(&["--version"]), &mut out, &mut err), 0);
        assert!(String::from_utf8_lossy(&out).contains("datacraft-import"));
    }

    #[test]
    fn usage_errors_exit_two_with_the_usage_line() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        assert_eq!(run(Vec::new(), &mut out, &mut err), 2);
        let stderr = String::from_utf8_lossy(&err);
        assert!(stderr.contains("usage: datacraft-import"), "{stderr}");
    }

    #[test]
    fn a_missing_database_exits_one_with_the_cause() {
        let mut out = Vec::new();
        let mut err = Vec::new();
        let missing = std::env::temp_dir().join("datacraft-import-does-not-exist.accdb");
        let code = run(
            args(&[missing.to_str().expect("utf-8 temp path")]),
            &mut out,
            &mut err,
        );
        assert_eq!(code, 1);
        assert!(
            String::from_utf8_lossy(&err).contains("reading the Access database failed"),
            "{}",
            String::from_utf8_lossy(&err)
        );
    }

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../fixtures")
            .join(name)
            .canonicalize()
            .expect("the fixture is present")
    }

    fn scratch(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock after the epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "datacraft-import-unit-{tag}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).expect("create the scratch directory");
        path
    }

    #[test]
    fn execute_writes_the_sidecar_and_the_human_summary() {
        let scratch = scratch("execute");
        let sidecar = scratch.join("programmability.datacraft");
        let options = Options {
            database: fixture("programmability.accdb"),
            sidecar: Some(sidecar.clone()),
            strict: false,
            json: false,
        };
        let mut out = Vec::new();
        assert_eq!(execute(&options, &mut out).expect("imports"), 0);
        let stdout = String::from_utf8_lossy(&out);
        assert!(stdout.contains("imported 4 objects"), "{stdout}");
        assert!(stdout.contains("wrote "), "{stdout}");
        assert!(sidecar.exists());
    }

    #[test]
    fn execute_merges_an_existing_sidecar() {
        use datacraft_core::form::FormDef;
        use datacraft_core::schema::{FieldDef, FieldKind, NumericKind, SchemaDocument, TableDef};
        let scratch = scratch("merge");
        let sidecar = scratch.join("programmability.datacraft");
        let mut existing = SchemaDocument::new("programmability");
        existing.tables = vec![TableDef::new(
            "Customers",
            vec![FieldDef::new(
                "ID",
                FieldKind::Number(NumericKind::LongInteger),
            )],
        )];
        existing.forms = vec![FormDef::new("Keep Me"), FormDef::new("Items Browser")];
        existing.save(&sidecar).expect("write the existing sidecar");
        let options = Options {
            database: fixture("programmability.accdb"),
            sidecar: Some(sidecar.clone()),
            strict: false,
            json: false,
        };
        let mut out = Vec::new();
        assert_eq!(execute(&options, &mut out).expect("imports"), 0);
        let merged = SchemaDocument::load(&sidecar).expect("loads");
        assert_eq!(merged.tables.len(), 1);
        assert!(merged.forms.iter().any(|form| form.name == "Keep Me"));
        assert_eq!(merged.forms.len(), 4, "three imported + one kept");
    }

    #[test]
    fn execute_emits_json_and_strict_exits_one() {
        let scratch = scratch("json-strict");
        let sidecar = scratch.join("formDesignsV2016.datacraft");
        let options = Options {
            database: fixture("formDesignsV2016.accdb"),
            sidecar: Some(sidecar.clone()),
            strict: true,
            json: true,
        };
        let mut out = Vec::new();
        assert_eq!(execute(&options, &mut out).expect("imports"), 1);
        let value: serde_json::Value =
            serde_json::from_slice(&out).expect("standard output is JSON");
        assert_eq!(value["report"]["objects"].as_array().map(Vec::len), Some(3));
        assert!(sidecar.exists());
    }

    #[test]
    fn execute_reports_a_malformed_sidecar() {
        let scratch = scratch("bad-sidecar");
        let sidecar = scratch.join("broken.datacraft");
        std::fs::write(&sidecar, "{not json").expect("write the broken sidecar");
        let options = Options {
            database: fixture("programmability.accdb"),
            sidecar: Some(sidecar),
            strict: false,
            json: false,
        };
        let mut out = Vec::new();
        let error = execute(&options, &mut out).expect_err("sidecar error");
        assert!(error.to_string().contains("sidecar"), "{error}");
    }

    #[test]
    fn error_variants_render_and_chain_their_sources() {
        let io = ImportError::Io(std::io::Error::other("disk full"));
        assert!(io.to_string().contains("disk full"));
        assert!(std::error::Error::source(&io).is_some());
        let sidecar = ImportError::Sidecar(datacraft_core::schema::SchemaError::Parse {
            detail: "bad".to_owned(),
        });
        assert!(sidecar.to_string().contains("bad"));
        assert!(std::error::Error::source(&sidecar).is_some());
    }
}
