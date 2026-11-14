//! `datacraft-import` — native Access forms/reports into a `.datacraft`
//! sidecar (Route B, ADR-9).

fn main() {
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let code = datacraft_import::cli::run(std::env::args().skip(1), &mut stdout, &mut stderr);
    std::process::exit(code);
}
