# Contract: specify a slice (or a crate's contract) before writing it

READ FIRST
- docs/prompts/README.md (repo facts, stack workflow), AGENTS.md, docs/MAP.md
- crates/<CRATE>/src/contract.rs
- the AGENTS.md of the two slices nearest to this one; for the first slices of a crate,
  the crate's own AGENTS.md (if any) and the upstream trait this mirrors

CONTEXT
I am about to add <SLICE> to <CRATE>. A slice is a directory under crates/<CRATE>/src/
that owns one domain concept -- its types, its logic, its tests and its own AGENTS.md.
Slices communicate only through contract.rs. When the "slice" is the crate's contract
itself, the design note is crates/<CRATE>/AGENTS.md.
The work lands on stack branch <BRANCH> (README: init or add). This prompt produces
commit 1 (the design note); prompt 03 adds the implementation on the same branch.

TASK
Do not write implementation code. Produce a design note, then stop and wait for me.
Write the AGENTS.md containing:

1. PURPOSE -- two sentences. What this slice is responsible for, plus one sentence
   beginning "This slice knows nothing about ..." naming what it must stay ignorant of.
2. PUBLIC SURFACE -- the exact function signatures other slices may call. If a new type
   must cross the boundary, list it separately as a proposed contract.rs change and flag
   it, because that is its own pull request.
3. INVARIANTS -- at least three statements that must always hold, written so a test could
   falsify them. "Handles errors gracefully" is not an invariant. "Returns Err rather
   than an empty Vec when the index file is missing" is.
4. FILE PLAN -- the files in the slice, one line each. Nothing over 400 lines. No file
   named helpers, utils, common or misc.
5. TEST PLAN -- for each invariant, the test that proves it and where its fixture comes
   from.
6. OPEN QUESTIONS -- everything you had to assume. Be exhaustive. I would far rather
   answer five questions now than review a confident misunderstanding.

CONSTRAINTS
- Signatures use types that already exist in contract.rs, or explicitly propose
  additions. No invented types buried mid-signature.
- Prefer owned, Send return types. Anything reaching the TUI crosses an mpsc channel and
  must not borrow.
- No new dependencies. If one seems necessary, put it under OPEN QUESTIONS with one
  paragraph: what it does, its size, its maintenance status, what writing it ourselves
  would cost.

OUTPUT
- The AGENTS.md, committed on <BRANCH> as `docs(<slice>): design note` and pushed; open
  the draft PR per README (it gains the implementation commit later). No attribution
  trailers.
- The OPEN QUESTIONS repeated in your reply, numbered, so I can answer inline.

DONE WHEN
I have answered the OPEN QUESTIONS. Do not begin implementing until then.

---

## Next run (pre-filled): the crate contract of shage-index

<CRATE> = shage-index · <SLICE> = the crate contract (src/contract.rs) plus NullBackend
(src/backends/null.rs; the `backends` slice starts with Null only) · <BRANCH> = index/contract.
Design note: crates/shage-index/AGENTS.md (crate-level; it carries the file plan for both
files). Stack setup: while PRs #1/#2 are open, `gh stack checkout bootstrap/planning-docs
&& gh stack add index/contract`; once they are merged, `git checkout main && git pull &&
gh stack init index/contract`.

Facts to design against:
- Starting point, not gospel (docs/plan/shage.html, "The trait that keeps tuicr honest"):
  SymbolRef { sym_id, display, path, line, confidence }, Confidence { Exact, Heuristic,
  Candidate, Unresolved }, IndexStamp { commit, overlay }, CallSite, CommitRef,
  Blast { direct, transitive, packages, exported }, and
  trait IndexBackend: Send { stamp, symbols_in_diff(patch), definition(sym), callers(sym),
  callees(sym), history(sym, n), blast_radius(sym, depth) }. Propose changes with reasons.
- Every value crosses a std::sync::mpsc channel from a worker thread into the TUI tick
  loop: owned + Send, no borrows, no Rc. Mirror VcsBackend
  (crates/shage-core/src/vcs/traits.rs) for shape and naming.
- Freshness: every result carries a stamp; the four states are distinguishable (no index;
  index behind HEAD; index missing this file; file dirty since indexing). An unstamped
  empty list must be unrepresentable in the types.
- Keep the contract index-format-neutral: no SCIP types leak in. SCIP 0.9.0 facts for the
  scip slice later: `enclosing_range` lives on Occurrence (definition occurrences) and
  `range`/`enclosing_range` are deprecated for `typed_range`/`typed_enclosing_range`.
- NullBackend returns empty/None everywhere, its stamp says "no index", and it is the
  default so an install without an indexer behaves exactly like upstream. No deps.
- Transitive walks always take a bounded depth parameter. Nothing returns data without a
  stamp.
- Questions I expect (answer them under OPEN QUESTIONS rather than assuming): is sym_id a
  newtype; which Confidence vocabulary (Exact/Heuristic/Candidate/Unresolved vs the
  ecosystem map's exact/heuristic/unresolved); the error type (a typed enum with no deps,
  or thiserror/anyhow — both would be new dependencies for shage-index); serde derives
  (needed later for export — new dependency, ask); how CallSite relates to SymbolRef;
  what `symbols_in_diff` takes (a unified diff string vs already-parsed hunks; the core
  has a strict hunk parser upstream); whether blast_radius returns Blast or a stamped
  wrapper like everything else.
