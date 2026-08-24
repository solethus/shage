//! The follow panel: cursor → symbol, follow stack, ranking.
//!
//! A view over what `shage-index` answered, and never the other way round: the index knows
//! nothing about terminals and this crate is where its answers become rows.
#![deny(missing_docs)]

pub mod contract;
pub mod panel;
