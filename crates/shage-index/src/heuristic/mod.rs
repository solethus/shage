//! Resolving by parsing the source, for a tree that will not build.
//!
//! The fallback tier, and the reason [`IndexBackend`](crate::contract::IndexBackend)
//! exists: a repository with no `Cargo.lock` that resolves, no `node_modules`, no
//! `compile_commands.json` still gets a panel, and every badge on it says how much it is
//! worth. Nothing here is ever [`Resolution::Exact`](crate::contract::Resolution::Exact).
//!
//! What it is known to get wrong, and why that is recorded rather than patched over:
//!
//! - **A call inside a macro invocation is invisible.** `println!("{}", f())` is a
//!   `token_tree` to tree-sitter, not an expression, so `f` is not called as far as this
//!   pass can see.
//! - **A macro-generated definition does not exist.** `make!(generated, 9)` defines
//!   `generated` only after expansion, so calling it resolves to nothing.
//! - **A receiver has no type.** `x.bar()` is every `bar` in the tree, badged
//!   `Candidates`, until something can say what `x` is.
//! - **`cfg` is not evaluated.** An item excluded from the build is still a definition here.
//!
//! Each of those produces a miss the differential oracle measures, which is the point: the
//! alternative is guessing, and a confident wrong edge costs more than an honest absence.

mod backend;
mod call_sites;
mod definitions;

pub use backend::HeuristicBackend;
pub use definitions::Definitions;
