//! The only surface other crates may touch.
//!
//! Two rules run through every type here, and the crate's `AGENTS.md` argues for both.
//! A [`SymbolRef`] is a location and nothing else: judgement about an answer lives on
//! [`Resolution`] and [`Blast`], never on the symbol. And nothing leaves this crate
//! without the commit that produced it, because [`Stamped`] is the only way out.

use std::ops::RangeInclusive;
use std::path::PathBuf;
use std::time::SystemTime;

use thiserror::Error;

/// An opaque symbol identity, as produced by whichever backend is installed.
///
/// Deliberately not `Display` and deliberately not `AsRef<str>`: rendering a sym_id to a
/// user is a bug ([`SymbolRef::display`] is the human-readable field), and parsing one for
/// a name is the bug that makes a call graph dishonest. SCIP symbol strings are opaque by
/// specification. [`SymId::as_str`] is the single, greppable way to get at the string.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SymId(String);

impl SymId {
    /// Wraps a backend's symbol string without inspecting it. Whatever the backend
    /// produced round-trips through [`SymId::as_str`] unchanged.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The identity as the backend wrote it. Use it to look a symbol up, never to show it.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A symbol, at its definition site. A location, not a judgement.
///
/// Nothing here says how much to trust the answer that produced it — that is
/// [`Resolution`]'s job — and nothing here says how far a change to it would reach — that
/// is [`Blast`]'s.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolRef {
    /// The backend's own identity for this symbol, opaque to everyone else.
    pub sym_id: SymId,
    /// Human-readable, backend-formatted, e.g. `"(*Bucket).Allow"`. For display only.
    pub display: String,
    /// Repository-relative. Never absolute, never `/`-prefixed, no `..`.
    pub path: PathBuf,
    /// 1-based line of the definition.
    pub line: u32,
}

/// The badge vocabulary: four words, one renderer.
///
/// Read it off a [`Resolution`] with [`Resolution::confidence`] when the panel wants to
/// badge a row without destructuring it. The declaration order is the trust order, so
/// `Exact < Heuristic < Candidate < Unresolved` and "sort by how much I trust this" is a
/// sort rather than a match.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Confidence {
    /// A compiler-backed index resolved it (SCIP, rust-analyzer).
    Exact,
    /// Scope and receiver analysis resolved it. One plausible target.
    Heuristic,
    /// Name matching found several plausible targets.
    Candidate,
    /// The backend could not resolve it. Not "there is nothing" — "I do not know".
    Unresolved,
}

/// What a backend was able to point at, and how sure it is.
///
/// The variants carry their targets, so [`Resolution::Unresolved`] has no [`SymbolRef`] to
/// misread and a resolved answer has no confidence to contradict it. `Candidates` keeps the
/// whole set rather than picking one: a Go `l.Allow` that matches four `Allow` methods is
/// four candidates, and collapsing it to one is the false edge this project exists to
/// avoid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// A compiler-backed index resolved it. One target, and the backend stands behind it.
    Exact(SymbolRef),
    /// Scope and receiver analysis resolved it. One plausible target, not a proof.
    Heuristic(SymbolRef),
    /// Several plausible targets, and never zero — the payload is a [`CandidateSet`], which
    /// cannot be built empty. Construct it with [`Resolution::candidates`].
    ///
    /// A single candidate stays a candidate: one name match is a weaker claim than scope
    /// analysis, and promoting it to `Heuristic` would overstate what the backend knows.
    Candidates(CandidateSet),
    /// The backend could not resolve it. Holds no [`SymbolRef`], so there is nothing here to
    /// misread as an answer.
    Unresolved,
}

/// A set of candidate targets that is never empty.
///
/// The `Vec` is private and [`Resolution::candidates`] is the only thing that fills it, so
/// "never empty" is a property of the type rather than a rule each backend has to remember.
/// Without it, `Resolution::Candidates(vec![])` is a Candidate badge with nothing behind it
/// that any crate can write — the dishonest UI this project exists to avoid.
///
/// In backend order, which is not an invariant: the panel sorts for display.
///
/// ```
/// use shage_index::contract::{Confidence, Resolution, SymId, SymbolRef};
/// let sym = SymbolRef {
///     sym_id: SymId::new("scip . . . `Bucket#allow().`"),
///     display: "(*Bucket).Allow".to_string(),
///     path: "src/lib.rs".into(),
///     line: 12,
/// };
/// let answer = Resolution::candidates(vec![sym]);
/// assert_eq!(answer.confidence(), Confidence::Candidate);
/// match &answer {
///     Resolution::Candidates(set) => assert_eq!(set.as_slice().len(), 1),
///     other => panic!("expected candidates, got {other:?}"),
/// }
/// ```
///
/// The emptiness is the invariant, so it is tested as one — this must not compile:
///
/// ```compile_fail
/// use shage_index::contract::{CandidateSet, Resolution};
/// let _ = Resolution::Candidates(CandidateSet(Vec::new()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateSet(Vec<SymbolRef>);

impl CandidateSet {
    /// The candidates. Never empty, and in backend order rather than a ranked one.
    pub fn as_slice(&self) -> &[SymbolRef] {
        &self.0
    }
}

impl Resolution {
    /// The only way to build [`Resolution::Candidates`], because [`CandidateSet`] holds a
    /// private `Vec`: an empty set of candidates is not a weak answer, it is no answer, so
    /// it comes back as `Unresolved` rather than as a candidate badge with nothing behind it.
    pub fn candidates(targets: Vec<SymbolRef>) -> Self {
        if targets.is_empty() {
            Resolution::Unresolved
        } else {
            Resolution::Candidates(CandidateSet(targets))
        }
    }

    /// The badge for this answer. Returns [`Confidence::Unresolved`] for exactly the
    /// [`Resolution::Unresolved`] variant and never for any other.
    pub fn confidence(&self) -> Confidence {
        match self {
            Resolution::Exact(_) => Confidence::Exact,
            Resolution::Heuristic(_) => Confidence::Heuristic,
            Resolution::Candidates(_) => Confidence::Candidate,
            Resolution::Unresolved => Confidence::Unresolved,
        }
    }

    /// The single target a backend stands behind, or `None` for `Candidates` and
    /// `Unresolved`. Picking one of several candidates is the panel's decision to offer,
    /// not this crate's to make.
    pub fn one(&self) -> Option<&SymbolRef> {
        match self {
            Resolution::Exact(sym) | Resolution::Heuristic(sym) => Some(sym),
            Resolution::Candidates(_) | Resolution::Unresolved => None,
        }
    }
}

/// One syntactic call, at the line where the call is written.
///
/// [`IndexBackend::callers`] returns the sites whose `target` reaches the queried symbol;
/// [`IndexBackend::callees`] returns the sites whose `from` is that symbol. In both
/// directions the [`Resolution`] says what the edge is worth.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallSite {
    /// The symbol whose body contains this call.
    pub from: SymbolRef,
    /// Where the call is written — not where `from` is defined.
    pub path: PathBuf,
    /// 1-based line of the call, in the same file as `path`.
    pub line: u32,
    /// The call expression as written, e.g. `"l.Allow"`. The evidence behind the badge: a
    /// [`Resolution::Candidates`] row that cannot show the reviewer *what* was ambiguous is
    /// asking them to trust a guess, which is the dishonest UI this crate exists to avoid.
    pub text: String,
    /// What the call resolves to, and how much this edge is worth. Carried per site, so a
    /// heuristic edge is never rendered as a compiler-backed one.
    pub target: Resolution,
}

/// A commit that touched a symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitRef {
    /// The full commit id as the backend's VCS writes it. Opaque: never truncated here, and
    /// never parsed — [`CommitRef::short_id`] is the abbreviated form.
    pub id: String,
    /// A prefix of `id`, abbreviated by the backend rather than by the panel, so a repo
    /// with colliding short hashes still renders unambiguous ones.
    pub short_id: String,
    /// The first line of the commit message, never the body: the panel has one row per
    /// commit and a wrapped body would push the next one off the screen.
    pub summary: String,
    /// Display name, backend-formatted. For display only — never matched against a user or
    /// parsed for an address.
    pub author: String,
    /// `std::time`, not chrono: this crate formats nothing and depends on nothing that
    /// does. The render site converts in one infallible line, because chrono provides
    /// `impl From<SystemTime> for DateTime<Utc>` under its `std` feature.
    pub time: SystemTime,
}

/// One changed file, reduced to what the index needs: which new-side lines moved.
///
/// The caller already holds parsed hunks, so this crate never learns what a patch is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangedFile {
    /// Repository-relative, new-side. Same convention as [`SymbolRef::path`]: never
    /// absolute, never `/`-prefixed, no `..`.
    pub path: PathBuf,
    /// 1-based, inclusive, new-side. Empty for a pure deletion.
    pub new_lines: Vec<RangeInclusive<u32>>,
}

/// Why the index cannot speak for a file.
///
/// There is no `Indexed` variant: a file the index covers simply does not appear in
/// [`DiffSymbols::uncovered`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileCoverage {
    /// The index has never seen this path.
    Missing,
    /// The file changed in the working tree since indexing, and no overlay covers it.
    Dirty,
}

/// One file the index cannot speak for, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileState {
    /// One of the paths the caller passed to [`IndexBackend::symbols_in_diff`], echoed
    /// unchanged so the caller can match it against its own input.
    pub path: PathBuf,
    /// Why the index cannot speak for [`FileState::path`]. The two variants are different
    /// conclusions for a reviewer, so they are never collapsed into "no data".
    pub coverage: FileCoverage,
}

/// The answer to [`IndexBackend::symbols_in_diff`].
///
/// `uncovered` is the difference between "this diff touches no symbols" and "the index has
/// never seen three of these files", which are opposite conclusions for a reviewer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffSymbols {
    /// Symbols whose definitions contain at least one changed line. Empty is a real answer,
    /// but only readable alongside `uncovered` and the stamp — never on its own.
    pub symbols: Vec<SymbolRef>,
    /// One entry per input file the index cannot speak for. A file absent from this list is
    /// covered at [`IndexStamp::indexed_commit`].
    pub uncovered: Vec<FileState>,
}

/// Which commit answered, and which commit the repository was on when it did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexStamp {
    /// The commit the index was built from. `None` means there is no index at all, which is
    /// why an empty answer is never on its own evidence of anything.
    pub indexed_commit: Option<String>,
    /// The commit the repository was on when the backend answered. `None` when the backend
    /// is not attached to a repository.
    ///
    /// Deliberately a commit and not a distance. Computing "N commits behind" needs an
    /// ancestry walk this crate does not do, and on a stacked pull request the index is
    /// usually not an ancestor at all — it is on a sibling branch, where "behind" is a lie.
    /// Rendering both ids is correct in every topology.
    pub repo_commit: Option<String>,
    /// Working-tree edits were folded into this answer.
    pub overlay: bool,
}

impl IndexStamp {
    /// The stamp of a backend with no index: both commits `None`, no overlay. Every field
    /// says "I know nothing", which is the one honest answer an empty index can give.
    pub fn none() -> Self {
        Self {
            indexed_commit: None,
            repo_commit: None,
            overlay: false,
        }
    }
}

/// A value that cannot exist without the stamp that produced it.
///
/// It derives no `Default` and implements no `From<T>` on purpose: adding either would let
/// an empty `Vec` reach the TUI without a commit behind it, and an unstamped empty list is
/// indistinguishable from a confident zero.
///
/// ```
/// use shage_index::contract::{IndexStamp, Stamped};
/// let answer = Stamped::new(IndexStamp::none(), Vec::<u8>::new());
/// assert!(answer.value.is_empty() && answer.stamp.indexed_commit.is_none());
/// ```
///
/// The absence is the invariant, so it is tested as one — this must not compile:
///
/// ```compile_fail
/// use shage_index::contract::Stamped;
/// let _: Stamped<Vec<u8>> = Stamped::default();
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stamped<T> {
    /// Which index produced [`Stamped::value`], and which commit the repository was on when
    /// it did.
    pub stamp: IndexStamp,
    /// The answer. Never interpret it without [`Stamped::stamp`]: an empty value is not a
    /// zero until the stamp says the index was there to count.
    pub value: T,
}

impl<T> Stamped<T> {
    /// The only constructor there is, so supplying a stamp is not something a backend can
    /// forget to do.
    pub fn new(stamp: IndexStamp, value: T) -> Self {
        Self { stamp, value }
    }
}

/// What a change to a symbol would reach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blast {
    /// Symbols that call it directly.
    pub direct: u32,
    /// Distinct symbols that reach it in `2..=depth` hops. Excludes `direct`; a cycle is
    /// counted once.
    pub transitive: u32,
    /// Packages or modules containing any of the above. Backend-formatted display strings,
    /// sorted and deduplicated, because the ranked file tree annotates with the names and a
    /// count would not do.
    pub packages: Vec<String>,
    /// The symbol is part of the crate's or package's exported surface.
    pub exported: bool,
    /// The depth this answer was computed at. Echoes the argument, so a caller can assert
    /// the walk was bounded rather than take it on faith.
    pub depth: u8,
}

/// Why a backend could not answer. Never used to mean "the index is empty" — that is a
/// stamp, not an error.
#[derive(Debug, Error)]
pub enum IndexError {
    /// The index exists but could not be read or parsed. An index file is untrusted input,
    /// so this is a normal outcome rather than a panic.
    #[error("index is unreadable: {0}")]
    Corrupt(String),

    /// The backend needs a file or a program that is not there.
    #[error("indexer unavailable: {0}")]
    Unavailable(String),

    /// This backend structurally cannot answer this question — a SCIP index file holds no
    /// git history, so its history query fails loudly instead of returning an empty list.
    /// `&'static str` on purpose: a fixed statement about a backend's capabilities, not a
    /// runtime detail.
    #[error("unsupported by this index backend: {0}")]
    Unsupported(&'static str),

    /// The backend touched the filesystem and it failed. Distinct from `Corrupt`: the bytes
    /// were never read, rather than read and not understood.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Every fallible call in this crate returns this. There is no `anyhow` here: the TUI picks
/// its empty state by matching on [`IndexError`], and erasing the variant would erase the
/// choice.
pub type Result<T> = std::result::Result<T, IndexError>;

mod backend;

pub use backend::IndexBackend;
