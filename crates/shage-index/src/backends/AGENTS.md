# slice: backends

The `IndexBackend` implementations, and the detection that picks one.

    scip        a compiler frontend already did the work        Exact
    heuristic   parse the source, because it will not build     Heuristic / Candidates
    null        there is no index                               Unresolved, stamped "none"

## Invariants you must not break

- **Detection never errors and never blocks startup.** Every failure falls through to the
  next tier; the last tier is `NullBackend`, which behaves exactly like upstream tuicr. A
  missing indexer is silence, not a warning dialog.
- **Detection never runs an indexer.** Installing and running one is a decision a user
  makes, not a side effect of opening a review. `detect` reads a file that is already there
  or parses a tree that is already there, and reaches no network.
- **Order is precision-first and is not a preference.** A compiler-backed index beats a
  parse of the source, which beats nothing. There is no tie to break.

## What lives where

    mod.rs        detect(), scip_index_path(), head_commit(). No backend logic.
    null.rs       NullBackend and its unit tests.

The two real backends live in their own slices — `src/scip/` and `src/heuristic/` — because
each is a body of format or language knowledge, and this directory is about choosing between
them.

## The commit sidecar

SCIP records no commit; the format has nowhere to put one. So `index.scip.commit`, written
by whoever ran the indexer, is where the commit lives, and `detect` reads it.

Without it `indexed_commit` is `None`, which the panel renders "no index". That understates
freshness for an index that exists — deliberately. A stamp is what a reviewer decides how
far to trust an empty panel by, so the safe direction of error is distrust. A backend that
guessed HEAD would claim a freshness nobody checked.

## Carry-overs this slice owes

- The config key that overrides `detect`. It needs a seam in `shage-core` that does not
  exist yet, so the order is fixed here and configurable in `index/seams`.

## Tests

`tests/backends.rs` — detection returns a working backend for a tree with code and for an
empty directory, and both answer `Ok`. The per-backend invariants live there too, run
against `HeuristicBackend` because it needs no external indexer; the SCIP backend is
exercised end to end by `cargo xtask oracle`.
