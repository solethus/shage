//! The index domain: symbols, edges, freshness. Knows nothing about the TUI and
//! must never depend on `shage-core`.
//!
//! `missing_docs` is denied: in this crate the doc comment *is* the invariant, so a public
//! item without one is a missing invariant, not a missing comment.
#![deny(missing_docs)]

pub mod backends;
pub mod contract;
