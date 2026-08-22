//! The [`IndexBackend`](crate::contract::IndexBackend) implementations, and the detection
//! that picks one.

pub mod null;

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::contract::IndexBackend;
use crate::heuristic::HeuristicBackend;
use crate::scip::ScipBackend;

pub use null::NullBackend;

/// Where a SCIP index sits by convention, relative to a repository root.
pub fn scip_index_path(root: &Path) -> PathBuf {
    root.join("index.scip")
}

/// Picks the most precise backend that can actually answer for `root`.
///
/// **Detection never errors and never blocks startup.** A missing indexer is silence, not a
/// warning dialog: every failure falls through to the next tier and the last tier is
/// [`NullBackend`], which behaves exactly like having no index at all. The order is
/// precision-first — a compiler-backed index beats a parse of the source, and a parse of
/// the source beats nothing — and it is deliberately not configurable here. Overriding it
/// needs a config key, which needs a seam in the TUI that does not exist yet.
///
/// The cost is bounded by the tier that wins: reading a `.scip` is one file, and the
/// fallback parse walks the tree once. Neither reaches the network, and neither runs an
/// indexer — installing and running one is a decision a user makes, not a side effect of
/// opening a review.
pub fn detect(root: &Path) -> Box<dyn IndexBackend> {
    let index = scip_index_path(root);
    if index.is_file() {
        let commit = ScipBackend::sidecar_commit(&index);
        if let Ok(backend) = ScipBackend::open(&index, root, commit) {
            return Box::new(backend);
        }
    }
    if let Ok(backend) = HeuristicBackend::open(root) {
        return Box::new(backend);
    }
    Box::new(NullBackend)
}

/// The commit `root` is on, or `None` when it is not a repository.
///
/// Never an error: a backend attached to a plain directory still answers every question it
/// can, it simply cannot say which commit the tree is on.
pub(crate) fn head_commit(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let commit = String::from_utf8(output.stdout).ok()?;
    let commit = commit.trim();
    (!commit.is_empty()).then(|| commit.to_owned())
}
