//! The differential oracle: a compiler frontend grades the pass that runs without one.
//!
//! Resolution correctness is the one thing here that cannot be eyeballed. Same-name
//! methods, trait dispatch, re-export chains, aliased imports and macro-generated items all
//! *look* right in review and are wrong in the panel. Rather than hand-write assertions
//! about them, this runs the real `rust-analyzer scip` over generated fixtures, runs the
//! heuristic pass over the same trees, and diffs the edge sets.
//!
//! Both sides are read through [`IndexBackend`](shage_index::contract::IndexBackend), so
//! nothing here knows which tier it is talking to and a backend that lands later is graded
//! by code that already exists.

mod baseline;
mod edges;
mod scoring;

use std::path::Path;
use std::process::{Command, ExitCode};

use shage_index::heuristic::HeuristicBackend;
use shage_index::scip::ScipBackend;

use crate::fixtures::hazards;
use scoring::{Disagreement, Got, Score};

/// Runs every fixture, prints the table, and fails on a regression against the floor.
pub fn run(root: &Path, bless: bool) -> ExitCode {
    if let Err(message) = check_indexer() {
        eprintln!("{message}");
        return ExitCode::FAILURE;
    }
    let floor = match baseline::Baseline::load(root) {
        Ok(floor) => floor,
        Err(err) => {
            eprintln!("oracle: cannot read {}: {err}", baseline::PATH);
            return ExitCode::FAILURE;
        }
    };

    let mut measured: Vec<(&str, Score)> = Vec::new();
    let mut reports = Vec::new();
    for fixture in hazards::ALL {
        let dir = root.join(crate::fixtures::DIR).join(fixture.name);
        if !dir.is_dir() {
            eprintln!(
                "oracle: {} is missing — run `cargo xtask fixtures`",
                fixture.name
            );
            return ExitCode::FAILURE;
        }
        match measure(&dir) {
            Ok(report) => {
                measured.push((fixture.name, report.score));
                reports.push((fixture.name, report));
            }
            Err(message) => {
                eprintln!("oracle: {}: {message}", fixture.name);
                return ExitCode::FAILURE;
            }
        }
    }

    print_table(&measured);
    print_disagreements(&reports);

    if bless {
        return match baseline::bless(root, &measured) {
            Ok(()) => {
                println!("\nblessed {} rows into {}", measured.len(), baseline::PATH);
                ExitCode::SUCCESS
            }
            Err(err) => {
                eprintln!("oracle: cannot write {}: {err}", baseline::PATH);
                ExitCode::FAILURE
            }
        };
    }
    verdict(&floor, &measured)
}

/// Fails only on a drop below the floor. A fixture the floor has never seen is new work,
/// not a regression, and is named rather than failed on.
fn verdict(floor: &baseline::Baseline, measured: &[(&str, Score)]) -> ExitCode {
    let mut regressed = Vec::new();
    let mut unblessed = Vec::new();
    for (name, score) in measured {
        match floor.regression(name, score) {
            Some(why) => regressed.push(format!("  {name}: {why}")),
            None if !floor.covers(name) => unblessed.push(*name),
            None => {}
        }
    }
    if !unblessed.is_empty() {
        println!(
            "\nnot in {} yet: {} — run `cargo xtask oracle --bless` to record them",
            baseline::PATH,
            unblessed.join(", ")
        );
    }
    if regressed.is_empty() {
        println!("\nno regression against {}", baseline::PATH);
        return ExitCode::SUCCESS;
    }
    eprintln!("\nregression against {}:", baseline::PATH);
    for line in &regressed {
        eprintln!("{line}");
    }
    eprintln!("\nIf the drop is deliberate, say so by re-blessing: cargo xtask oracle --bless");
    ExitCode::FAILURE
}

/// One fixture's grade, with both edge sets kept so the report can name what it compared.
struct Report {
    score: Score,
    truth: edges::Edges,
    measured: edges::Edges,
    disagreements: Vec<Disagreement>,
}

/// Indexes one fixture with the real indexer and grades the heuristic pass against it.
fn measure(dir: &Path) -> Result<Report, String> {
    let index = dir.join("index.scip");
    let output = Command::new("rust-analyzer")
        .arg("scip")
        .arg(dir)
        .arg("--output")
        .arg(&index)
        .output()
        .map_err(|err| format!("running rust-analyzer: {err}"))?;
    if !output.status.success() {
        return Err(format!(
            "rust-analyzer scip failed:\n{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    // The indexer just ran against this tree, so HEAD is genuinely the commit it indexed.
    // Writing it down is the only reason the backend can stamp an answer honestly.
    let commit = head(dir);
    if let Some(commit) = &commit {
        let _ = std::fs::write(
            index.with_extension(shage_index::scip::COMMIT_SIDECAR),
            commit,
        );
    }

    let truth = ScipBackend::open(&index, dir, commit).map_err(|err| err.to_string())?;
    let under_test = HeuristicBackend::open(dir).map_err(|err| err.to_string())?;

    let files = edges::whole_tree(dir);
    let truth = edges::of(&truth, &files).map_err(|err| err.to_string())?;
    let measured = edges::of(&under_test, &files).map_err(|err| err.to_string())?;
    let (score, disagreements) = scoring::grade(&truth, &measured);
    Ok(Report {
        score,
        truth,
        measured,
        disagreements,
    })
}

fn print_table(measured: &[(&str, Score)]) {
    println!(
        "{:<22} {:>5} {:>7} {:>5} {:>5} {:>5} {:>5}",
        "fixture", "prec", "recall", "hits", "miss", "amb", "edges"
    );
    for (name, score) in measured {
        println!(
            "{name:<22} {:>5.2} {:>7.2} {:>5} {:>5} {:>5} {:>5}",
            score.precision(),
            score.recall(),
            score.hits,
            score.missed,
            score.ambiguous,
            score.truth
        );
    }
}

fn print_disagreements(reports: &[(&str, Report)]) {
    let total: usize = reports.iter().map(|(_, r)| r.disagreements.len()).sum();
    if total == 0 {
        return;
    }
    println!("\n{total} disagreements");
    for (name, report) in reports {
        for disagreement in &report.disagreements {
            println!("  {name}  {}", render(report, disagreement));
        }
    }
}

/// One disagreement as "expected A -> B, got A -> C".
///
/// `A` comes from whichever side saw the site, and both `B` and `C` are named by the side
/// that produced them: the two backends give the same definition different display names,
/// and rewriting one into the other's vocabulary would hide exactly the kind of mismatch
/// this report exists to surface.
fn render(report: &Report, disagreement: &Disagreement) -> String {
    let (path, line) = &disagreement.site;
    let caller = report
        .truth
        .callers
        .get(&disagreement.site)
        .or_else(|| report.measured.callers.get(&disagreement.site))
        .map(|from| report.truth.describe(from))
        .unwrap_or_else(|| "?".to_owned());
    let call = report.truth.names.get(&disagreement.site).cloned();
    let expected = match &disagreement.expected {
        Some(to) => format!("{caller} -> {}", report.truth.describe(to)),
        None => format!("{caller} -> no edge"),
    };
    let got = match &disagreement.got {
        Got::Wrong(to) => format!("{caller} -> {}", report.measured.describe(to)),
        Got::Ambiguous(candidates) => format!(
            "{caller} -> {} candidates [{}]",
            candidates.len(),
            candidates
                .iter()
                .map(|to| report.measured.describe(to))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Got::Nothing => format!("{caller} -> nothing"),
    };
    let written = call
        .filter(|c| !c.is_empty())
        .map(|c| format!(" `{c}`"))
        .unwrap_or_default();
    format!(
        "{}:{line}{written}\n      expected {expected}\n           got {got}",
        path.display()
    )
}

fn head(dir: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|commit| !commit.is_empty())
}

/// Fails with something actionable when the indexer is not usable.
///
/// It runs the binary rather than probing `PATH`, because on a rustup toolchain `PATH`
/// lies: `rust-analyzer` is a shim that exists whether or not the component is installed,
/// and only running it says which.
fn check_indexer() -> Result<(), String> {
    match Command::new("rust-analyzer").arg("--version").output() {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(format!(
            "oracle: `rust-analyzer --version` failed:\n  {}\n\
             It is on PATH but not usable — on a rustup toolchain that means the component \
             is missing.\n  Install it with: rustup component add rust-analyzer",
            String::from_utf8_lossy(&output.stderr).trim()
        )),
        Err(err) => Err(format!(
            "oracle: rust-analyzer is not on PATH ({err}).\n  \
             The oracle grades against a real compiler frontend, so there is no useful \
             answer without one.\n  Install it with: rustup component add rust-analyzer"
        )),
    }
}
