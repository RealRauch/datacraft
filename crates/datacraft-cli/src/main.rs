//! The `datacraft-cli` binary: composition root for the CLI client
//! (Core over the format-detecting port), delegating to the library.
//!
//! Successes go to stdout, failures and usage errors to stderr (step 23.1,
//! item T4); the exit code carries the outcome for machines.

use datacraft_core::core::Core;
use datacraft_core::port::StoragePort;
use datacraft_storage::detecting::DetectingPort;

/// The composed driver set (ADR-7): built-in probes plus the out-of-tree
/// CSV directory driver.
fn csv_driver() -> Box<dyn StoragePort> {
    Box::new(datacraft_csv::CsvPort::new())
}

/// The Excel read driver (phase 96, M127): an out-of-tree driver like CSV,
/// composed at the roots only.
fn xlsx_driver() -> Box<dyn StoragePort> {
    Box::new(datacraft_xlsx::ExcelPort::new())
}

fn main() {
    let mut core = Core::new(Box::new(
        DetectingPort::new()
            .with_fallback(csv_driver)
            .with_fallback(xlsx_driver),
    ));
    core.register_import_reader(Box::new(datacraft_xlsx::ExcelImport));
    let args: Vec<String> = std::env::args().skip(1).collect();
    let stdout = std::io::stdout();
    let stderr = std::io::stderr();
    let stdin = std::io::stdin();
    let outcome = datacraft_cli::run_split(
        &mut core,
        &args,
        &mut stdout.lock(),
        &mut stderr.lock(),
        &mut stdin.lock(),
    );
    std::process::exit(outcome.code());
}
