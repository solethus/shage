//! Reads a SCIP index produced by a real compiler frontend.
//!
//! `rust-analyzer scip`, `scip-go`, `scip-typescript` and `scip-clang` all emit the same
//! format, so this slice names no language. It knows nothing about diffs, review sessions,
//! terminals or ranking.
//!
//! The invariants live in `AGENTS.md` beside this file. The two that bite hardest:
//! occurrence ranges are three ints for a single line and four for a multi-line span —
//! never assume four — and a symbol string is opaque, so nothing here parses one for a
//! name.

mod backend;
mod call_sites;
mod definitions;
mod load;
mod occurrences;

pub use backend::{COMMIT_SIDECAR, ScipBackend};
pub use load::load;
pub use occurrences::{Span, decode_range, is_definition};
