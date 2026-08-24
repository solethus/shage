//! The index domain: symbols, edges, freshness. Knows nothing about the TUI and
//! must never depend on `shage-core`.
//!
//! Two backends answer the same questions at different precisions, and the crate is laid
//! out by how they answer rather than by how good they are: [`scip`] reads what a compiler
//! frontend already worked out, [`heuristic`] parses the source when the project will not
//! build. [`enclosing`] is the part neither owns and both need.
//!
//! `missing_docs` is denied: in this crate the doc comment *is* the invariant, so a public
//! item without one is a missing invariant, not a missing comment.
#![deny(missing_docs)]

pub mod backends;
pub mod call_graph;
pub mod contract;
pub mod enclosing;
pub mod heuristic;
pub mod scip;
