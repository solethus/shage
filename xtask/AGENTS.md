# xtask — repo automation

Three commands, run as `cargo xtask <cmd>` (the alias is in `.cargo/config.toml`).

    seams      stub; will diff crates/shage-core against the vendor tag
    fixtures   write the hazard projects into target/fixtures/
    oracle     grade the heuristic backend against rust-analyzer

## The fixture rule, worth writing on the wall

**Generate fixtures from code. Never commit a `.scip` file or a tarball of a repository.**

A committed binary fixture drifts from the real encoding as the indexer version moves, and
then the tests assert against a museum piece while production reads something else. The
generator *is* the fixture. It is also the only readable description of what each hazard is,
which is why the hazards carry doc comments and the generated projects carry none.

Each fixture is a real cargo project with an empty `[workspace]` table — they live under
`target/`, inside this repository's workspace, and cargo walks upward looking for one.
Without it every fixture fails with "believes it's in a workspace when it's not".

Every fixture gets `git init` and at least one commit, with the identity and timestamp
supplied on the command line rather than read from `~/.gitconfig`: a fixture whose commit
hash depends on who generated it cannot be compared between two machines, and a machine with
no `user.email` would fail here rather than in CI.

## The oracle

Resolution correctness is the one thing in this project that cannot be eyeballed. Same-name
methods, trait dispatch, re-export chains, aliased imports and macro-generated items all look
right in review and are wrong in the panel. So rather than hand-write assertions about them,
the oracle runs the real `rust-analyzer scip` over the fixtures, runs the heuristic pass over
the same trees, and diffs the edge sets.

Both sides are read **through `IndexBackend`**. Nothing in `oracle/` knows which tier it is
talking to, so a backend that lands later is graded by code that already exists rather than by
code written to flatter it.

### Edge identity

SCIP symbol strings and the heuristic pass's ids are not comparable and neither is anyone's
to change, so an edge is normalised to **locations**: the call site, and the definition it
reaches. That is also what "A calls B" means to a reviewer reading the panel.

A site is `(file, line)` and not a column, because the two backends anchor a call
differently — SCIP marks the identifier token, tree-sitter marks the whole callee expression,
so `Owner::bar()` starts in two different columns. Comparing *sets of edges* rather than one
target per site keeps two calls on one line (`f() + g()`) distinct without depending on
either anchor.

### Scoring

| quantity | rule |
|---|---|
| true positive | the site is in both, with the same target |
| false positive | the pass under test committed to an edge the oracle does not have |
| false negative | the oracle has an edge the pass under test did not commit to |
| precision | `TP / (TP + FP)`, defined as `1.00` when it committed to nothing |
| recall | `TP / truth`, defined as `1.00` when there is nothing to find |
| ambiguous | sites answered with a candidate set, reported beside the metrics |
| contained | of those, how many held the right answer |

**A candidate set scores as a miss.** Counting one as a hit would let a pass reach recall
1.00 by returning every symbol in the repository, and the panel already renders a candidate
set as "I do not know which" — the metric agrees with the badge rather than flattering it.
`contained` is reported separately so the gap between "no idea" and "nearly there" stays
visible without being scored. Argue with this table before changing a number.

### The baseline is a floor

`xtask/oracle.baseline`, two decimals, one row per fixture. The pass under test will never
match a compiler frontend, and a test demanding parity is one that gets deleted rather than
fixed. What is defended is that the numbers never go *down* without someone saying so:
`cargo xtask oracle` exits non-zero on a drop and prints which fixture and which metric.
`--bless` rewrites the file, which is how a deliberate drop is recorded.

A fixture the floor has never seen is **not** a regression. It is new work, and failing on it
would mean a new fixture could never be added without a red build; the run names it instead.

**The floor is a measurement of one indexer's answers**, so it is only comparable against the
version that produced it. A newer rust-analyzer resolves things the old one did not and the
numbers move for reasons that have nothing to do with our resolver, which reads as a
regression and is not one. That is why the oracle job in `ci.yml` pins its toolchain, and why
the run prints `graded against rust-analyzer <version>` above the table. Bump the pin and
re-bless in the same commit, never separately.

### rust-analyzer

The availability check runs `rust-analyzer --version` rather than probing `PATH`, because on
a rustup toolchain `PATH` lies: the shim exists whether or not the component is installed,
and only running it says which. Both failure paths print `rustup component add
rust-analyzer`.

## What lives where

    src/main.rs            argument dispatch and the repository root.
    src/fixtures/mod.rs    Fixture, writing, git init and commit.
    src/fixtures/hazards.rs the seven hazards, as source.
    src/oracle/mod.rs      the run: index, grade, table, verdict.
    src/oracle/edges.rs    one backend's answers -> a comparable edge set.
    src/oracle/scoring.rs  precision, recall, and the disagreement list.
    src/oracle/baseline.rs the committed floor.

xtask depends on `shage-index` and nothing else. It reads `.scip` through the slice that owns
that format and never parses a protobuf itself: a second reader is a second thing to be
wrong, and the pass under test would end up grading itself.

## In CI

`ci.yml` runs `cargo xtask fixtures` then `cargo xtask oracle` on every pull request, in a job
that installs rust-analyzer as a rustup component at the pinned toolchain. A regression
against the floor fails the build and names the fixture and the metric.
