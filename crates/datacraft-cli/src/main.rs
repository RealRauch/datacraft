//! The `datacraft-cli` binary: composition root for the CLI client
//! (Core over the real Jet4 port), delegating to the library.

use datacraft_core::core::Core;
use datacraft_storage::port_adapter::Jet4Port;

fn main() {
    let mut core = Core::new(Box::new(Jet4Port::new()));
    let args: Vec<String> = std::env::args().skip(1).collect();
    let outcome = datacraft_cli::run(&mut core, &args, &mut std::io::stdout().lock());
    std::process::exit(outcome.code());
}
