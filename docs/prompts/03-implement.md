# Implement: build one slice from its contract

CONTEXT
crates/<CRATE>/src/<SLICE>/AGENTS.md is the approved specification for this
slice. It is the source of truth. This prompt is not.

READ FIRST -- in this order, and stop when you have enough
0. docs/prompts/README.md (repo facts, gates, stack workflow)
1. crates/<CRATE>/src/<SLICE>/AGENTS.md (or crates/<CRATE>/AGENTS.md for a crate contract)
2. crates/<CRATE>/src/contract.rs
3. any file the AGENTS.md names explicitly
Do not read other slices. If you believe you must, stop and tell me which one
and why -- that means the contract is incomplete, and finding out now is cheap.
You are on the stack branch prompt 02 created (check with `gh stack view --json`);
the implementation is the next commit on that branch.

TASK
Implement the slice exactly as specified. Write the tests from the TEST PLAN in
the same change, first where practical.

CONSTRAINTS
- Every public item gets a doc comment stating an invariant it upholds, not a
  restatement of its own name.
- Errors: anyhow at the boundary, typed errors inside the slice wherever a
  caller might reasonably branch on the variant.
- No panics on untrusted input. An index file is untrusted input.
- No new dependencies without asking. To ask, stop and give me one paragraph:
  what it does, its size, its maintenance status, and what writing it ourselves
  would cost.
- If the spec turns out to be wrong or under-specified while you are writing,
  STOP. Do not improvise. Report the gap and propose an AGENTS.md amendment.

OUTPUT
1. The code, as one commit on the stack branch (`feat(<crate>): <slice>` or similar),
   pushed with `git push --force-with-lease origin <branch>` if you amended. The draft PR
   from prompt 02 picks it up; if none exists yet, open it per README. No attribution
   trailers.
2. Real output from: cargo fmt --all --check, cargo clippy --workspace -- -D warnings,
   cargo clippy -p <CRATE> --all-targets -- -D warnings, cargo test --workspace,
   cargo xtask seams --check. Run them; do not predict them.
3. A short list of anything you did that the spec did not literally require.

DONE WHEN
All five commands are clean, every invariant has a named test, and the PR shows the
design-note commit followed by the implementation commit.

DO NOT
- Touch crates/shage-core. A seam is the separate `index/seams` pull request with
  SEAMS.md rows.
- Leave TODO comments. Either do it, or list it in the summary as out of scope.
- Add Co-Authored-By or any other attribution to the commit or the PR.
