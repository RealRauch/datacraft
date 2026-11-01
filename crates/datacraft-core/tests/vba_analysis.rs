//! Static VBA analysis contract (step 17.3, item L5).
//!
//! Headless, deterministic and MCP-friendly: call graph, embedded SQL
//! literals, table/query references and unused modules. No execution.

use datacraft_core::vba::{VbaModule, VbaModuleKind, VbaProject};
use datacraft_core::vba_analysis::analyze;

fn module(name: &str, kind: VbaModuleKind, source: &str) -> VbaModule {
    VbaModule {
        name: name.to_owned(),
        kind,
        source: source.to_owned(),
    }
}

fn synthetic_project() -> VbaProject {
    VbaProject {
        modules: vec![
            module(
                "Orders",
                VbaModuleKind::Standard,
                r#"Option Compare Database
Public Sub Main()
    Helper()
    Inventory.RestockProduct 5
    Dim worker As New Worker
    Set rs = db.OpenRecordset("SELECT * FROM [Customers] WHERE [ID]=1")
    db.Execute "DELETE FROM Orders"
End Sub

Public Function Helper() As Long
    Helper = 1
End Function
"#,
            ),
            module(
                "Inventory",
                VbaModuleKind::Standard,
                r#"Public Sub RestockProduct(ByVal id As Long)
    ' Orders.Main is a comment, not a call
    Debug.Print "Orders.Main is a string, not a call"
End Sub
"#,
            ),
            module(
                "Unused",
                VbaModuleKind::Standard,
                "Public Sub NeverCalled()\nEnd Sub\n",
            ),
            module(
                "Worker",
                VbaModuleKind::Class,
                "Public Sub DoIt()\nEnd Sub\n",
            ),
            module(
                "Form_Orders",
                VbaModuleKind::Form,
                "Private Sub Form_Load()\n    Helper()\nEnd Sub\n",
            ),
        ],
    }
}

#[test]
fn the_call_graph_resolves_qualified_calls_new_and_bare_procedures() {
    let analysis = analyze(
        &synthetic_project(),
        &["Customers".to_owned(), "Orders".to_owned()],
        &["Q1".to_owned()],
    );
    let edge = |from: &str, to: &str| {
        analysis
            .call_graph
            .iter()
            .find(|entry| entry.module == from)
            .is_some_and(|entry| entry.callees.iter().any(|callee| callee == to))
    };
    assert!(edge("Orders", "Orders"), "bare Helper() resolves");
    assert!(edge("Orders", "Inventory"), "qualified call resolves");
    assert!(edge("Orders", "Worker"), "New Worker resolves");
    assert!(edge("Form_Orders", "Orders"), "event code calls helper");
    assert!(!edge("Inventory", "Orders"), "comments/strings do not count");
    assert_eq!(
        analysis
            .call_graph
            .iter()
            .find(|entry| entry.module == "Inventory")
            .expect("entry")
            .callees,
        Vec::<String>::new()
    );
}

#[test]
fn sql_literals_and_table_references_are_extracted() {
    let analysis = analyze(
        &synthetic_project(),
        &["Customers".to_owned(), "Orders".to_owned()],
        &["Q1".to_owned()],
    );
    let sql: Vec<&str> = analysis
        .sql_literals
        .iter()
        .map(|literal| literal.sql.as_str())
        .collect();
    assert!(
        sql.iter()
            .any(|literal| literal.contains("SELECT * FROM [Customers]")),
        "{sql:?}"
    );
    assert!(
        sql.iter()
            .any(|literal| literal.contains("DELETE FROM Orders")),
        "{sql:?}"
    );
    assert_eq!(analysis.table_references, vec!["Customers", "Orders"]);
    assert!(analysis.query_references.is_empty());
}

#[test]
fn unused_modules_are_standard_modules_without_incoming_edges() {
    let analysis = analyze(&synthetic_project(), &[], &[]);
    assert_eq!(analysis.unused_modules, vec!["Unused"]);
    let again = analyze(&synthetic_project(), &[], &[]);
    assert_eq!(analysis, again, "analysis is deterministic");
}

#[test]
fn an_empty_project_has_empty_results() {
    let analysis = analyze(&VbaProject { modules: vec![] }, &[], &[]);
    assert!(analysis.call_graph.is_empty());
    assert!(analysis.sql_literals.is_empty());
    assert!(analysis.table_references.is_empty());
    assert!(analysis.query_references.is_empty());
    assert!(analysis.unused_modules.is_empty());
}
