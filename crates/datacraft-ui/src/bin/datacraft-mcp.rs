//! The `datacraft-mcp` binary (step 9.1, item H32): the headless MCP
//! endpoint. Composition root — it owns the real stdio handles and the
//! format-detecting port, then runs the core's line-protocol serve loop (decision
//! default 8: stdio JSON-RPC 2.0). Agents connect by spawning the process.

use datacraft_core::core::Core;
use datacraft_core::transport;
use datacraft_storage::detecting::DetectingPort;

fn main() {
    let mut core = Core::new(Box::new(DetectingPort::new()));
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut reader = std::io::BufReader::new(stdin.lock());
    let mut writer = stdout.lock();
    transport::serve(&mut reader, &mut writer, &mut core);
}
