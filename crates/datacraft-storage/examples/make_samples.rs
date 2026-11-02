//! Sample generator (phase 29, step 29.3, item H44).
//!
//! Usage: `cargo run -p datacraft-storage --example make_samples [--check]`
//!
//! Writes the deterministic sample set into `<workspace>/samples`;
//! `--check` regenerates into a temp directory and byte-compares against
//! the shipped files instead — the drift guard behind the committed
//! samples and the `samples/manifest` hashes.

fn main() {
    let check = std::env::args().any(|arg| arg == "--check");
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("samples");
    let result = if check {
        datacraft_storage::samples::check(&dir)
    } else {
        datacraft_storage::samples::generate_all(&dir)
    };
    match result {
        Ok(()) => println!(
            "samples {} OK: {}",
            if check { "check" } else { "generation" },
            dir.display()
        ),
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(1);
        }
    }
}
