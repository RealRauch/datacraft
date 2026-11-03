//! Static analysis over the real Northwind VBA project (step 17.3, L5).

use std::path::{Path, PathBuf};

use datacraft_core::port::StoragePort;
use datacraft_core::vba_analysis::analyze;
use datacraft_jet::port_adapter::Jet4Port;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("fixtures")
        .join(name)
}

#[test]
fn the_northwind_project_analysis_is_complete_and_deterministic() {
    let mut port = Jet4Port::new();
    port.open(&fixture("northwind2007.accdb"), None)
        .expect("fixture opens");
    let project = port
        .vba_project()
        .expect("project extracts")
        .expect("project present");
    let tables: Vec<String> = port.tables().into_iter().map(|table| table.name).collect();
    // The fixture carries native queries outside the decoded MSysQueries
    // subset; analysis works without them (the core falls back the same way).
    let queries: Vec<String> = port
        .native_queries()
        .unwrap_or_default()
        .into_iter()
        .map(|query| query.name)
        .collect();

    let analysis = analyze(&project, &tables, &queries);
    assert_eq!(analysis.call_graph.len(), project.modules.len());
    assert!(
        analysis
            .sql_literals
            .iter()
            .any(|literal| literal.sql.contains("SELECT")),
        "SQL literals are found: {:#?}",
        analysis.sql_literals
    );
    assert!(
        analysis
            .table_references
            .iter()
            .any(|table| table == "Employee Privileges"),
        "string-literal table references are resolved: {:#?}",
        analysis.table_references
    );
    assert!(
        analysis
            .call_graph
            .iter()
            .any(|entry| entry.module == "Form_Order Details"
                && entry
                    .callees
                    .iter()
                    .any(|callee| callee == "CustomerOrders")),
        "qualified calls resolve: {:#?}",
        analysis.call_graph
    );
    assert!(
        analysis
            .unused_modules
            .iter()
            .all(|name| !name.starts_with("Form_") && !name.starts_with("Report_")),
        "host-bound modules are not reported unused: {:?}",
        analysis.unused_modules
    );
    assert_eq!(analysis, analyze(&project, &tables, &queries));
}
