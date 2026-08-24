//! Repo automation, invoked as `cargo xtask <cmd>` (alias in .cargo/config.toml).

mod fixtures;
mod oracle;

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

/// The upstream tag `crates/shage-core/` is vendored from. Bump it on an upstream sync.
const VENDOR_TAG: &str = "vendor/tuicr-v0.23.1";

/// The only files under `crates/shage-core/` allowed to differ from [`VENDOR_TAG`].
///
/// The same list as the table in `docs/SEAMS.md`; when the two disagree, one of them is
/// wrong and the failure message says where to look.
const DECLARED_SEAMS: &[&str] = &[
    "crates/shage-core/AGENTS.md",
    "crates/shage-core/Cargo.toml",
    "crates/shage-core/src/update/install/tests.rs",
];

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let root = repo_root();
    match args.first().map(String::as_str) {
        Some("seams") => seams(&root),
        Some("fixtures") => match fixtures::run(&root) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("xtask fixtures: {err}");
                ExitCode::FAILURE
            }
        },
        Some("oracle") => oracle::run(&root, args.iter().any(|arg| arg == "--bless")),
        _ => {
            eprintln!(
                "usage: cargo xtask <seams [--check] | fixtures | oracle [--bless]>\n\
                 \n  seams     check crates/shage-core against the vendor tag\n  \
                 fixtures  write the hazard projects into {}/\n  \
                 oracle    grade the heuristic backend against rust-analyzer",
                fixtures::DIR
            );
            ExitCode::from(2)
        }
    }
}

/// Fails when anything under `crates/shage-core/` differs from the vendor tag without a row
/// in `docs/SEAMS.md`.
///
/// This is `docs/SEAMS.md`'s own check, run rather than described. No pathspec is passed to
/// `git diff` — one would disable rename detection, and every file in the slice is a rename
/// from upstream's repo root, so without `-M` over the whole tree the check reports the
/// entire vendored source as drift.
fn seams(root: &Path) -> ExitCode {
    let output = match Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["diff", "-M", "--name-status", VENDOR_TAG, "HEAD"])
        .output()
    {
        Ok(output) => output,
        Err(err) => {
            eprintln!("xtask seams: running git: {err}");
            return ExitCode::FAILURE;
        }
    };
    if !output.status.success() {
        eprintln!(
            "xtask seams: git diff against {VENDOR_TAG} failed:\n  {}\n  \
             The tag is the vendor base; fetch it with: git fetch --tags",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        return ExitCode::FAILURE;
    }

    let listing = String::from_utf8_lossy(&output.stdout);
    let mut undeclared = Vec::new();
    for line in listing.lines() {
        let Some((status, paths)) = line.split_once('\t') else {
            continue;
        };
        // `R100` is a pure rename: upstream's repo root moved under crates/shage-core/ with
        // the bytes unchanged, which is the layout move rather than drift.
        if status == "R100" {
            continue;
        }
        // A rename status carries `old\tnew`; the new path is the one the seam table names.
        let path = paths.rsplit('\t').next().unwrap_or(paths);
        if path.starts_with("crates/shage-core/") && !DECLARED_SEAMS.contains(&path) {
            undeclared.push(format!("  {status}\t{path}"));
        }
    }

    if undeclared.is_empty() {
        println!(
            "xtask seams: crates/shage-core/ differs from {VENDOR_TAG} only at declared seams"
        );
        return ExitCode::SUCCESS;
    }
    eprintln!("xtask seams: undeclared drift from {VENDOR_TAG}:");
    for line in &undeclared {
        eprintln!("{line}");
    }
    eprintln!(
        "\nEvery edit under crates/shage-core/ needs a row in docs/SEAMS.md. Add one there\n\
         and to DECLARED_SEAMS in xtask/src/main.rs, or revert the edit."
    );
    ExitCode::FAILURE
}

/// The repository root, from this crate's manifest rather than the working directory.
///
/// `cargo xtask` can be run from any subdirectory, and every path in here is
/// repository-relative because that is what `SymbolRef::path` promises.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask/ always has a parent")
        .to_path_buf()
}
