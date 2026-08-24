# Test: build the differential oracle

CONTEXT
shage resolves symbols two ways. Tier 3 reads a SCIP index produced by a real
compiler frontend (rust-analyzer). Tier 1 is our own tree-sitter pass, used
when a project will not build. For our purposes tier 3 is ground truth.
Read docs/prompts/README.md first. `xtask/` already exists (std-only, `seams` stub,
`cargo xtask <cmd>` alias in .cargo/config.toml): add the `fixtures` and `oracle`
subcommands there. This work is the `index/fixtures-oracle` layer (README table); it
depends on the scip slice below it in the stack.

TASK
Build a differential harness that uses tier 3 to grade tier 1.

1. cargo xtask fixtures -- generate small, real cargo projects into
   target/fixtures/, each isolating exactly one resolution hazard:
     a. two inherent methods with the same name on different types
     b. a trait method with two impls, called through a generic bound
     c. a trait object call (dynamic dispatch)
     d. a pub-use re-export chain three modules deep
     e. an aliased import
     f. a macro-generated function
     g. a symbol renamed between two commits in the fixture's own git history
   Generate them from code. Never commit a .scip file or a binary fixture --
   they drift from the real encoding, and then the tests assert a museum piece.

2. For each fixture, run the real `rust-analyzer scip <fixture> --output <file>`
   (rust-analyzer must be on PATH; fail with a clear message if it is not) and load
   the result through the scip slice. Never parse the protobuf here.
3. Run our tier-1 resolver over the same fixture.
4. Compare edge sets. Report per fixture: precision, recall, and every
   disagreement as "expected A -> B, got A -> C".
5. Assert a floor, not equality. Tier 1 will never match tier 3, and a test
   demanding parity gets deleted by whoever hits it on a Friday. Commit a
   baseline file and fail only on regression against it.

ALSO ADD
- Snapshot tests of the follow panel using ratatui TestBackend at 120x24 and at
  80x24, asserting on rendered text. Remember the usable content area is
  meaningfully smaller than the frame once borders are drawn -- size the panel
  from the content outward.
- A property test: for any hunk and any symbol table, the symbol attributed to
  a line always has a range containing that line, or is None. Never a
  neighbour. Generate the ranges; do not hand-pick them.

DONE WHEN
cargo xtask oracle prints a table and exits non-zero on regression against the
committed baseline. It runs in CI on every pull request once ci.yml has been repointed
for the fork; until then the PR body lists that wiring as the follow-up. Commit on the
stack branch, open the PR per README, no attribution trailers.
