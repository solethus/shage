//! The `IndexBackend` trait.
//!
//! Split out of `contract/mod.rs` only because that file reached the 400-line cap; it is
//! re-exported from `contract`, so `shage_index::contract::IndexBackend` is unchanged and
//! remains the only path consumers use.

use super::{
    Blast, CallSite, ChangedFile, CommitRef, DiffSymbols, IndexStamp, Resolution, Result, Stamped,
    SymId,
};

/// Mirrors `VcsBackend: Send` (`crates/shage-core/src/vcs/traits.rs`) for shape and naming,
/// with one deliberate divergence: no method has a default body.
///
/// `VcsBackend` defaults to an unsupported-operation error because its backends differ in
/// *capability* — Mercurial has no index to stage against. `IndexBackend`'s backends differ
/// in *precision*, and precision must never be expressed as a missing implementation. A
/// default here would let a half-written backend compile and answer "nothing calls this". A
/// backend that genuinely cannot answer says so with
/// [`IndexError::Unsupported`](super::IndexError::Unsupported).
///
/// `Send` because every answer crosses an `std::sync::mpsc` channel from a worker thread
/// into the TUI tick loop.
pub trait IndexBackend: Send {
    /// The freshness of the index itself, with no query attached. The status bar shows this
    /// before the reviewer asks anything, so "no index" is visible before an empty panel
    /// can be misread.
    fn stamp(&self) -> Result<IndexStamp>;

    /// The symbols whose definitions contain the given changed lines, plus the files this
    /// index cannot speak for.
    fn symbols_in_diff(&self, files: &[ChangedFile]) -> Result<Stamped<DiffSymbols>>;

    /// Where the symbol is defined, and how sure the backend is that this is it.
    fn definition(&self, sym: &SymId) -> Result<Stamped<Resolution>>;

    /// The call sites that reach this symbol. Each carries its own [`Resolution`], so a
    /// heuristic edge is never presented as a compiler-backed one.
    fn callers(&self, sym: &SymId) -> Result<Stamped<Vec<CallSite>>>;

    /// The call sites written inside this symbol's body.
    fn callees(&self, sym: &SymId) -> Result<Stamped<Vec<CallSite>>>;

    /// Commits that touched this symbol, most recent first. `limit` is a hard cap, not a
    /// hint; `0` returns empty.
    fn history(&self, sym: &SymId, limit: usize) -> Result<Stamped<Vec<CommitRef>>>;

    /// What changing this symbol would reach, walked no further than `depth` hops.
    ///
    /// `depth` is not optional and has no default: an unbounded walk on a real workspace is
    /// a hang, and a hang in a TUI reads as a crash. The returned [`Blast::depth`] echoes
    /// this argument.
    fn blast_radius(&self, sym: &SymId, depth: u8) -> Result<Stamped<Blast>>;
}
