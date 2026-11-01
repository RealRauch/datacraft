//! Generates the manual Access-repair-gate artifact (BACKLOG H9 acceptance).
//!
//! Copies the pristine pinned fixture and extends it with a fresh table of
//! every core type through the clean-room Jet4 writer. The result lands in
//! `access-gate/GateCheck.mdb` — open it in real Microsoft Access and follow
//! `access-gate/CHECKLIST.md`. Run from the workspace root:
//!
//! ```text
//! cargo run -p datacraft-storage --example access_gate
//! ```

use datacraft_storage::jet::{JetColumn, JetType, Jet4Writer};
use datacraft_storage::StorageValue;

fn main() {
    let source = std::path::Path::new("fixtures/testV2000.mdb");
    let out_dir = std::path::Path::new("access-gate");
    std::fs::create_dir_all(out_dir).expect("creates the output directory");
    let target = out_dir.join("GateCheck.mdb");
    std::fs::copy(source, &target).expect("copies the pinned fixture");

    let file = std::fs::read(&target).expect("reads the copy");
    let mut writer = Jet4Writer::new(file).expect("opens the pristine fixture");

    let tdef = writer
        .create_table(
            "GateCheck",
            &[
                JetColumn::new("Id", JetType::LongInt, 0x03, 0, 0, 4),
                JetColumn::new("Name", JetType::Text, 0x02, 1, 0, 60),
                JetColumn::new("Score", JetType::Double, 0x03, 2, 4, 8),
                JetColumn::new("Active", JetType::Bool, 0x01, 3, 12, 0),
                JetColumn::new("Note", JetType::Memo, 0x02, 4, 0, 0),
            ],
        )
        .expect("creates the GateCheck table");

    let rows: Vec<(i64, &str, f64, bool, Option<&str>)> = vec![
        (1, "alpha", 1.5, true, Some("first row")),
        (2, "beta", -0.25, false, None),
        (3, "gamma", 42.0, true, Some("mit Umlauten: äöüß")),
    ];
    for (id, name, score, active, note) in rows {
        writer
            .insert_row(
                tdef,
                &[
                    StorageValue::Int(id),
                    StorageValue::Text(name.to_owned()),
                    StorageValue::Float(score),
                    StorageValue::Bit(active),
                    match note {
                        Some(text) => StorageValue::Text(text.to_owned()),
                        None => StorageValue::Null,
                    },
                ],
            )
            .expect("inserts");
    }
    std::fs::write(&target, writer.into_file()).expect("writes the artifact");
    println!("wrote {} — open it in Access and follow access-gate/CHECKLIST.md", target.display());
}
