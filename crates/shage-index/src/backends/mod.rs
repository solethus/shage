//! The [`IndexBackend`](crate::contract::IndexBackend) implementations.
//!
//! Null only, for now. Detection — the code that picks a backend and never blocks startup
//! doing it — arrives with the scip backend.

pub mod null;

pub use null::NullBackend;
