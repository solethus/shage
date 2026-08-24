# shage-index — the crate contract

The crate contract: `src/contract/`, and the rules every slice under it upholds. Each slice
carries its own AGENTS.md — `src/enclosing/`, `src/scip/`, `src/heuristic/`, `src/backends/`
today, and `classify/`, `callgraph/`, `freshness/` and `resolve/` as they land. Read this
note plus the one slice you are changing; the whole crate is not the unit.

## Purpose

`shage-index` answers four questions about a symbol — where it is defined, who calls it,
what it calls, and what changing it would reach — and stamps every answer with the commit
that produced it; this slice is the only surface other crates may touch, holding the owned
`Send` types that cross an mpsc channel, the `IndexBackend` trait, and the backend that
always answers "there is no index".

`src/contract/` knows nothing about diffs, patches, review sessions, keybindings, ratatui,
crossterm, `shage-core`, SCIP, protobuf, tree-sitter, or any on-disk index format — it names
no file format and no programming language, and it performs no I/O. Format and language
knowledge is quarantined in the backend slices, one format or language each, and every one
of them speaks only the types below.

The crate as a whole knows nothing about the TUI, and that is the rule with teeth: see
invariant 8.

## The shape of an answer

Two rules run through every type below.

**A `SymbolRef` is a location and nothing else.** It says where a symbol is. It carries no
judgement — no confidence, no exportedness, no test-ness — because those are properties of
an *answer to a question*, not of the symbol. Judgement lives on `Resolution` and `Blast`.

**Nothing leaves this crate without the commit that produced it.** `Stamped<T>` is the only
way out, so an empty `Vec` can never be mistaken for a confident zero.

```
    question ──► backend ──► Stamped<T> ──► mpsc ──► TUI
                              │      │
                              │      └── T: owned, Send, no borrows
                              └── IndexStamp: which commit answered, and whether
                                  the working tree was folded in
```

## Public surface

Everything below is `#[derive(Debug, Clone)]`, owned, `Send + 'static`, and lives in
`src/contract/` (`mod.rs` for the types, `backend.rs` for the trait, re-exported so the
public path stays `contract::IndexBackend`). Nothing else in the crate is `pub` outside its
slice.

```rust
/// An opaque symbol identity, as produced by whichever backend is installed.
///
/// Deliberately not `Display`: rendering a sym_id to a user is a bug (`SymbolRef::display`
/// is the human-readable field), and parsing one for a name is the bug that makes a call
/// graph dishonest. SCIP symbol strings are opaque by specification.
pub struct SymId(String);
impl SymId {
    pub fn new(s: impl Into<String>) -> Self;
    pub fn as_str(&self) -> &str;
}
// + PartialEq, Eq, Hash, Ord — it is a map key and a sort key.

/// A symbol, at its definition site. A location, not a judgement.
pub struct SymbolRef {
    pub sym_id: SymId,
    /// Human-readable, backend-formatted, e.g. "(*Bucket).Allow". For display only.
    pub display: String,
    /// Repository-relative. Never absolute, never `/`-prefixed, no `..`.
    pub path: PathBuf,
    /// 1-based line of the definition.
    pub line: u32,
}

/// The badge vocabulary: four words, one renderer, `Copy` so the UI can pass it around.
///
/// Read off a `Resolution` with `.confidence()` when the panel wants to badge a row
/// without destructuring it. `Ord` runs Exact < Heuristic < Candidate < Unresolved, so
/// "sort by how much I trust this" is a sort, not a match.
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
/// The variants carry their targets, so `Unresolved` has no `SymbolRef` to misread and a
/// resolved answer has no confidence to contradict it. `Candidates` keeps the whole set
/// rather than picking one: a Go `l.Allow` that matches four `Allow` methods is four
/// candidates, and collapsing it to one is the false edge this project exists to avoid.
pub enum Resolution {
    Exact(SymbolRef),
    Heuristic(SymbolRef),
    /// Never empty, structurally — the payload cannot be built empty. A single candidate
    /// stays a candidate: one name match is a weaker claim than scope analysis.
    Candidates(CandidateSet),
    Unresolved,
}

/// A set of candidate targets that is never empty, because the `Vec` is private and
/// `Resolution::candidates` is the only thing that fills it — see invariant 4.
pub struct CandidateSet(Vec<SymbolRef>);   // private field on purpose
impl CandidateSet {
    /// Never empty, and in backend order rather than a ranked one.
    pub fn as_slice(&self) -> &[SymbolRef];
}

impl Resolution {
    /// The only way to build `Candidates`, because `CandidateSet` holds a private `Vec`: an
    /// empty set of candidates is not a weak answer, it is no answer, so it comes back as
    /// `Unresolved` rather than as a candidate badge with nothing behind it.
    pub fn candidates(targets: Vec<SymbolRef>) -> Resolution;
    pub fn confidence(&self) -> Confidence;
    /// The single target, or `None` for `Candidates` and `Unresolved`. Jumping to a
    /// candidate is the panel's decision to offer, not this crate's to make.
    pub fn one(&self) -> Option<&SymbolRef>;
}

/// One syntactic call, at the line where the call is written.
///
/// `callers(sym)` returns the sites whose `target` reaches `sym`; `callees(sym)` returns
/// the sites whose `from` is `sym`. The same type answers both directions, and in both the
/// `Resolution` says how much the edge is worth.
pub struct CallSite {
    /// The symbol whose body contains this call.
    pub from: SymbolRef,
    /// Where the call is written — not where `from` is defined.
    pub path: PathBuf,
    pub line: u32,
    /// The call expression as written, e.g. "l.Allow". Evidence for the badge: a
    /// `Candidates` row that cannot show *what* was ambiguous is asking the reviewer
    /// to trust a guess.
    pub text: String,
    pub target: Resolution,
}

/// A commit that touched a symbol.
pub struct CommitRef {
    pub id: String,
    pub short_id: String,
    pub summary: String,
    pub author: String,
    /// `std::time`, not chrono — see Decisions.
    pub time: SystemTime,
}

/// One changed file, reduced to what the index needs: which new-side lines moved.
///
/// The caller already holds parsed hunks; this crate must never learn to parse a patch.
pub struct ChangedFile {
    pub path: PathBuf,
    /// 1-based, inclusive, new-side. Empty for a pure deletion.
    pub new_lines: Vec<RangeInclusive<u32>>,
}

/// Why the index cannot speak for a file. There is no `Indexed` variant: a file the index
/// covers simply does not appear in `DiffSymbols::uncovered`.
pub enum FileCoverage {
    /// The index has never seen this path.
    Missing,
    /// The file changed in the working tree since indexing, and no overlay covers it.
    Dirty,
}
pub struct FileState {
    pub path: PathBuf,
    pub coverage: FileCoverage,
}

/// The answer to `symbols_in_diff`: what was found, and which inputs it could not speak
/// for. The second half is the difference between "this diff touches no symbols" and "the
/// index has never seen three of these files".
pub struct DiffSymbols {
    pub symbols: Vec<SymbolRef>,
    /// One entry per input file the index cannot speak for. A file absent from this list
    /// is covered at `IndexStamp::indexed_commit`.
    pub uncovered: Vec<FileState>,
}

/// Which commit answered, and which commit the repository was on when it did.
pub struct IndexStamp {
    /// The commit the index was built from. `None` means there is no index at all.
    pub indexed_commit: Option<String>,
    /// The commit the repository was on when the backend answered. `None` when the backend
    /// is not attached to a repository. A commit, deliberately not a distance — see below.
    pub repo_commit: Option<String>,
    /// Working-tree edits were folded into this answer.
    pub overlay: bool,
}
impl IndexStamp {
    /// The stamp of a backend with no index: both commits `None`, no overlay.
    pub fn none() -> Self;
}

/// A value that cannot exist without the stamp that produced it.
///
/// No `Default`, no `From<T>`: the only way to build one is to supply a stamp.
pub struct Stamped<T> {
    pub stamp: IndexStamp,
    pub value: T,
}
impl<T> Stamped<T> {
    pub fn new(stamp: IndexStamp, value: T) -> Self;
}

/// What a change to a symbol would reach.
pub struct Blast {
    /// Symbols that call it directly.
    pub direct: u32,
    /// Distinct symbols that reach it in 2..=depth hops. Excludes `direct`; a cycle is
    /// counted once.
    pub transitive: u32,
    /// Packages or modules containing any of the above. Backend-formatted display strings,
    /// sorted and deduplicated. The ranked file tree annotates with these, so a count
    /// would not do.
    pub packages: Vec<String>,
    /// The symbol is part of the crate's or package's exported surface.
    pub exported: bool,
    /// The depth this answer was computed at. Echoes the argument, so a caller can assert
    /// the walk was bounded.
    pub depth: u8,
}

#[derive(Debug, thiserror::Error)]
pub enum IndexError {
    /// The index exists but could not be read or parsed.
    Corrupt(String),
    /// The backend needs a file or a program that is not there.
    Unavailable(String),
    /// Structurally cannot answer — a SCIP index holds no git history, so its `history` is
    /// `Unsupported("scip: no commit data")`, never an empty list. See invariant 7.
    Unsupported(&'static str),
    Io(#[from] std::io::Error),
}
pub type Result<T> = std::result::Result<T, IndexError>;

/// Mirrors `VcsBackend: Send` (crates/shage-core/src/vcs/traits.rs) for shape and naming,
/// with one deliberate divergence: no method has a default body, because those backends
/// differ in *capability* and these differ in *precision*. See invariant 7.
pub trait IndexBackend: Send {
    /// The freshness of the index itself, with no query attached. Drives the status bar.
    fn stamp(&self) -> Result<IndexStamp>;

    fn symbols_in_diff(&self, files: &[ChangedFile]) -> Result<Stamped<DiffSymbols>>;

    fn definition(&self, sym: &SymId) -> Result<Stamped<Resolution>>;

    fn callers(&self, sym: &SymId) -> Result<Stamped<Vec<CallSite>>>;

    fn callees(&self, sym: &SymId) -> Result<Stamped<Vec<CallSite>>>;

    /// Most recent first. `limit` is a hard cap, not a hint; `0` returns empty.
    fn history(&self, sym: &SymId, limit: usize) -> Result<Stamped<Vec<CommitRef>>>;

    /// `depth` is not optional and has no default. An unbounded walk on a real workspace
    /// is a hang, and a hang in a TUI reads as a crash.
    fn blast_radius(&self, sym: &SymId, depth: u8) -> Result<Stamped<Blast>>;
}

/// The default backend. Returns empty everywhere, stamped "no index", so an install with
/// no indexer behaves exactly like upstream tuicr.
pub struct NullBackend;   // in src/backends/null.rs
```

## The freshness states

An empty caller list means "nothing calls this" or "the index never saw this commit", and
those are opposite conclusions. Four of the states below are degraded and one — the index
commit matching the repo commit — is the healthy one. The four degraded ones are the four
`follow/freshness` owes a snapshot test each: no index, index at a different commit, file
missing from the index, file dirty since indexing.

```
   IndexStamp                              what the panel says
   ─────────────────────────────────────────────────────────────────────────────
   indexed_commit: None                →    "no index"               degraded
   Some(a), repo_commit: Some(a)       →    "index at a1b2c3d"       healthy
   Some(a), repo_commit: Some(b), a≠b  →    "index at a1b2c3d,       degraded
                                             tree at 9f8e7d6"

   DiffSymbols::uncovered
   ─────────────────────────────────────────────────────────────────────────────
   FileCoverage::Missing               →    "never seen this file"   degraded
   FileCoverage::Dirty                 →    "edited since indexing"  degraded
```

No two share a representation, and no state is the absence of the others.

Why the third state names two commits instead of a distance — a stack of pull requests is
the normal case, not the exception:

```
   main ──● a1b2c3d  ← index built here
           \
            ● index/contract        "index at a1b2c3d, tree at 9f8e7d6"   honest
             \
              ● index/seams  ← HEAD  "3 commits behind"                    needs an
                                                                           ancestry walk
   sibling stack
   main ──● a1b2c3d ──● follow/panel ← HEAD   "behind" is simply false here
```

A backend that grows a repository handle can add a relation (ancestor / descendant /
divergent) later without touching this shape. Until then the stamp states facts.

## Invariants

1. **No index is never an error.** Every `NullBackend` method returns `Ok`, with
   `stamp.indexed_commit == None`. A missing indexer is silence, not a failure dialog.
2. **No value crosses the boundary unstamped.** Every `IndexBackend` method returns
   `Stamped<_>`; `Stamped` has no `Default` and no `From<T>`, so an empty `Vec` cannot
   reach the TUI without the commit that produced it. The absence is itself tested, by a
   `compile_fail` doctest on `Stamped` — the same trick guards `CandidateSet`, see 4.
3. **The freshness states are mutually exclusive, and none is an absence of another.** The
   table above is exhaustive and its rows do not overlap: four degraded states plus the
   healthy one.
4. **A badge never outlives its evidence.** `Resolution::Unresolved` holds no `SymbolRef`;
   `Resolution::candidates(vec![])` is `Unresolved`, and `Candidates` is never empty
   *structurally* — `CandidateSet`'s `Vec` is private, so an outside crate cannot construct,
   destructure or `Default` its way to an empty one; and `Resolution::confidence()` returns
   `Unresolved` for exactly the `Unresolved` variant and never for any other.
5. **`blast_radius` is bounded and says so.** `Blast::depth` equals the `depth` argument,
   for every backend, including Null.
6. **Every contract type is `Send + 'static`, and the trait is object-safe.**
   `Box<dyn IndexBackend>` compiles, and no contract type borrows.
7. **A backend that cannot answer says `Unsupported`, never empty.** No trait method has a
   default body, so the compiler catches the omission before a reviewer does.
8. **`shage-index` never depends on `shage-core` or any TUI crate.** It takes exactly four
   dependencies, each earning its place by being the thing it wraps: `thiserror` (which
   `shage-core` already pins, so no new crate entered the lockfile), `scip` and its
   `protobuf` runtime — the encoding is the protocol's, and a hand-rolled reader would drift
   from it silently — and `tree-sitter` with `tree-sitter-rust` for the pass that runs
   without a compiler. `proptest` is a dev-dependency. `cargo tree -p shage-index --edges
   normal` lists those and their transitive deps, and no `ratatui`, `crossterm` or
   `shage-core`. The direction is the invariant: `shage-follow` depends on this crate, and
   nothing here depends back.

## File plan

| file | contents |
|---|---|
| `AGENTS.md` | this note |
| `src/contract/mod.rs` | every type above, `IndexStamp::none()`, `Stamped::new()`, `Resolution::{candidates, confidence, one}`, `CandidateSet::as_slice()`. No I/O, no `#[cfg(test)]` — the tests live outside so they exercise the surface the way a consumer does |
| `src/contract/backend.rs` | the `IndexBackend` trait alone, re-exported by `mod.rs` so the public path stays `contract::IndexBackend` |
| `src/call_graph.rs` | definitions, edges, and the five trait methods both real backends answer identically. Written once rather than twice with a bounded walk that drifts |
| `src/enclosing/` | which definition encloses a line. Shared, because both backends need it and neither owns it |
| `src/scip/` | the SCIP reader. `load`, `occurrences`, `definitions`, `call_sites`, `backend` |
| `src/heuristic/` | the tree-sitter pass. `definitions`, `call_sites`, `backend` |
| `src/backends/` | `detect()`, and `null.rs` |
| `src/lib.rs` | the module list and `#![deny(missing_docs)]` |
| `tests/contract.rs` | the invariants that must hold from outside the crate |
| `tests/attribution.rs` | the line-attribution property, over generated ranges |
| `tests/backends.rs` | the invariants `NullBackend` could only satisfy vacuously, run against a backend that answers |

No file over 400 lines, none named helpers, utils, common or misc. `contract.rs` reached the
cap and was split as planned: the trait moved to `contract/backend.rs` behind a
`contract/mod.rs` re-export, which is why the public path is unchanged. The `lines` column
above is hand-maintained and rots on any edit — treat it as a sketch, not a fact.

## Test plan

`src/contract/` reads no repository, so its tests need no fixtures. The backend slices do,
and those fixtures are generated by `cargo xtask fixtures` and never committed — see
`xtask/AGENTS.md`.

| invariant | test | where |
|---|---|---|
| 1, never an error | `null_answers_ok_everywhere` — call all seven methods, assert `is_ok()` and `indexed_commit.is_none()` | `src/backends/null.rs` |
| 1, empty everywhere | `null_answers_empty` — `Vec::is_empty()`, `definition` is `Unresolved`, `uncovered` is empty, `Blast` all-zero with `exported == false` | `src/backends/null.rs` |
| 2, nothing unstamped | `stamp_travels_with_an_empty_value` — build a `Stamped<Vec<SymbolRef>>` through the only constructor there is | `tests/contract.rs` |
| 2, the absence of `Default` | a `compile_fail` doctest on `Stamped` asserting `Stamped::default()` does not compile, paired with a passing doctest so the failure is attributable | `src/contract/mod.rs` |
| 3, states mutually exclusive | `stamp_states_compare_unequal` — build the three stamps and the two `FileState`s, assert each matches exactly one row of the table and none of the others. Whether the panel *renders* five different things is `follow/freshness`'s snapshot tests, not this one | `tests/contract.rs` |
| 4, `Candidates` never empty | a `compile_fail` doctest on `CandidateSet` asserting `Resolution::Candidates(CandidateSet(Vec::new()))` does not compile outside the crate, paired with a passing one so the failure is attributable. Construction, destructuring and `Default` are all rejected (E0423, E0532, E0277) | `src/contract/mod.rs` |
| 4, badge needs evidence | `unresolved_holds_no_symbol`; `candidates_are_never_empty` — `Resolution::candidates(vec![])` is `Unresolved` and a one-element list stays `Candidates`; `resolution_confidence_round_trips` — each variant maps to its own `Confidence` and `Candidates(vec![a, b]).one().is_none()` | `tests/contract.rs` |
| 5, bounded depth | `null_blast_echoes_depth` — `blast_radius(&sym, 0)` and `(&sym, 7)` come back with `depth == 0` and `depth == 7` | `src/backends/null.rs` |
| 6, Send and object-safe | `contract_types_are_send` — `fn assert_send<T: Send + 'static>() {}` over every contract type; `fn assert_object_safe(_: Box<dyn IndexBackend>) {}`; and a `NullBackend` moved into `thread::spawn` that sends a `Stamped<Vec<CallSite>>` back over an `mpsc::channel`, which is the shape of the real seam | `tests/contract.rs` |
| 7, no silent defaults | `IndexBackend` has no default bodies, so an incomplete `impl` fails to compile. Asserted by review, not by a test — a test that proves a compile error needs `trybuild`, a dependency this crate will not take | — |
| 8, no dependencies | `cargo tree -p shage-index --edges normal` prints `shage-index` plus `thiserror` and its proc-macro deps, and nothing else | CI, and by hand before submit |

## Decisions

Settled, with the reasoning that is not obvious from the code:

- **`Confidence` has four variants**, and `Resolution` carries the targets so the fourth
  cannot be attached to a symbol. The panel matches on `Confidence`; the type system
  guarantees the match is meaningful.
- **`Stamped<T>` on every method**, over per-type stamp fields, so the rule is enforced by
  the compiler rather than by remembering.
- **`SymId` is a newtype** and is not `Display`. It is the one string that must never be
  parsed for a name or shown to a user.
- **`symbols_in_diff` takes `&[ChangedFile]`**, not a patch string. The caller already
  parsed the hunks; this crate never learns what a patch is.
- **`thiserror = "2.0"`** over a hand-written `Display`. shage-core already pins it, so no
  new crate enters the lockfile — one line in this crate's `Cargo.toml`, one edge in
  `Cargo.lock`.
- **`CommitRef::time` is `SystemTime`.** Nothing here formats a date. Upstream's
  `CommitInfo` uses `DateTime<Utc>`, which drops the author's timezone offset just as
  `SystemTime` does, so nothing is lost that the UI shows today.
- **No `is_test` or `exported` on `SymbolRef`.** Ranking (`follow/blast`) calls
  `blast_radius` per changed symbol anyway, so `Blast` is where both belong; `is_test`
  joins it in that PR. Keeping judgement off `SymbolRef` is the rule that makes this
  obvious.
- **`CallSite` carries the call text.** Added by `index/fixtures-oracle`, the branch whose
  panel first renders a `Candidates` badge, exactly as the carry-over required. The
  heuristic backend produces candidates constantly — every `x.bar()` is one — so the field
  went in with the badge rather than after it.
- **No serde.** Export (`shage index callers --json`) is `index/callgraph` and can take
  that dependency then, as its own decision.
- **No detection.** `NullBackend` is the default but nothing constructs it yet: `detect()`
  and the config key belong to `index/scip`, where there is a second backend to choose
  between.
- **`Resolution::Candidates` is in backend order, and that order is not an invariant.**
  The panel sorts for display. Promising a stable first candidate would make every backend
  responsible for a ranking none of them can justify.
- **`stamp()` stays on the trait** even though every answer is stamped. The status bar shows
  the index's freshness before the reviewer asks anything, and "no index" is exactly the
  state a reviewer needs to see before they start trusting an empty panel.
