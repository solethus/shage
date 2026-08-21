//! Repo automation, invoked as `cargo xtask <cmd>` (alias in .cargo/config.toml).
use std::process::ExitCode;

fn main() -> ExitCode {
    match std::env::args().nth(1).as_deref() {
        // ponytail: stub. The real check diffs crates/shage-core against the vendor
        // tag named in docs/SEAMS.md and fails on undeclared drift.
        Some("seams") => {
            eprintln!("xtask seams: stub, no drift check yet (see docs/SEAMS.md)");
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("usage: cargo xtask seams [--check]");
            ExitCode::from(2)
        }
    }
}
