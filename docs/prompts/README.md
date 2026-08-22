# Prompt library — how to run these

Each numbered file is a complete prompt to paste into an agent session opened in this
repo. Every prompt assumes the agent has read this file, `AGENTS.md` and `docs/WORKFLOW.md`
first; the prompts themselves stay short because the shared facts live here.

## Repo facts every prompt assumes
- Fork of tuicr (MIT, Almog Gavra). Upstream base: tag `vendor/tuicr-v0.23.1` = commit 0dacb6b
  (the v0.23.1 release + #559). The fork is `crates/shage-core/`: package/binary `shage`,
  library crate `tuicr`. Edit it only at seams listed in `docs/SEAMS.md`; every row there is a
  deliberate upstream divergence and `cargo xtask seams --check` (stub today) will enforce it.
- New code lives in `crates/shage-index` (no TUI deps, never depends on shage-core),
  `crates/shage-follow` and `xtask`. Cross-crate types live in a crate's `src/contract.rs` only.
- Where the seams will go at v0.23.1: `App`, `InputMode`, `AnnotatedLine` in
  `crates/shage-core/src/app/mod.rs`; `Action` in `crates/shage-core/src/input/keybindings.rs`;
  the poll loop and `poll_*` calls in `crates/shage-core/src/main.rs`; the two backend traits to
  mirror are `VcsBackend: Send` (`src/vcs/traits.rs`) and `ForgeBackend` (`src/forge/traits.rs`,
  not Send). The forge pattern — snapshot inputs, std thread, `std::sync::mpsc`, drain each tick,
  mutate `App` only on the main thread — is the concurrency model everything index-related copies.
- Gates for every PR: `cargo fmt --all --check`, `cargo clippy --workspace -- -D warnings`,
  `cargo test --workspace`, `cargo xtask seams --check`. No new dependencies without asking.
- The Format and Clippy jobs are pinned to a Rust version in `ci.yml`; `check` and `test`
  still track `stable`. rustfmt and clippy add rules in every release, and a fork that must
  not edit vendored code cannot fix what a newer lint finds there — clippy 1.98 flagged
  `crates/shage-core/src/vcs/jj/mod.rs:181` and turned every branch red. Develop against the
  pinned version, and bump it in the commit that fixes what the newer lint found.
- No `Co-Authored-By` or any other AI attribution on commits, PRs or PR comments.
- Local quirk (this Mac): if a build fails at link time with `undefined symbols: _iconv` from
  `libgit2_sys`, run cargo with `CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER=/usr/bin/cc`.
  It is the machine's default `cc` (mise clang), not the code.

## Stacked PRs
Work never lands on `main` directly; each prompt names the branch it produces.
1. Start or extend a stack: `gh stack init <area>/<slug>` (new stack rooted at `main`) or, while on
   the top branch of an existing stack, `gh stack add <area>/<slug>`.
2. Commit on that branch — one logical commit where possible; the subject is the PR title.
3. Open the PR yourself (on this fork gh-stack pushes but cannot open or find PRs), with a
   description written to `.github/PULL_REQUEST_TEMPLATE.md`:
   `gh pr create --repo solethus/shage --draft --base <branch below> --head <branch> --title "<subject>" --body-file <file>`
   Mark it ready once the gates pass: `gh pr ready <n> --repo solethus/shage`.
4. To update: commit or amend, `git push --force-with-lease origin <branch>`; after changing a
   lower layer, `gh stack rebase --upstack` and push the layers above.
5. Until PRs #1/#2 (the `bootstrap/*` stack) are merged, new work stacks on top of
   `bootstrap/planning-docs` (`gh stack checkout bootstrap/planning-docs`, then `gh stack add …`).
   Once they are merged, start fresh stacks from an up-to-date `main`.

## Planned order (docs/plan/shage.html, "the first ten PRs")
| prompts | branch | delivers |
|---|---|---|
| 02 → 03 | `index/contract` | contract.rs types, `IndexBackend` trait, `NullBackend` |
| 02 → 03 → 04 | `index/seams` | the six seams in shage-core, wired to nothing (SEAMS.md rows) |
| 05 | `index/fixtures-oracle` | `cargo xtask fixtures` / `cargo xtask oracle`, and the two backends they grade — `index/scip` was folded in here rather than run separately, because an oracle with nothing to read and nothing to grade is not an oracle |
| 02 → 03 → 04 | `index/classify` | call-position classification, Unknown stays Unknown |
| 02 → 03 → 04 | `index/callgraph` | edges, bounded walks, `shage index callers` CLI |
| 02 → 03 → 04 | `follow/panel` | the follow panel — first user-visible feature (new stack) |
| 02 → 03 | `follow/freshness` | stamps and the four empty states |
| 02 → 03 → 04 | `follow/blast` | blast radius and the ranked file tree |

Next up: `index/classify` (receiver typing, which is what three of the seven fixtures are
waiting for), then `index/seams`.

## Carry-overs
Something a design note deferred, and the branch that has to discharge it. Add a row when
you defer; delete it when the branch lands. Written down so the inheriting run does not
have to rediscover why.

| branch | obligation | from |
|---|---|---|
| `index/seams` | the config key that overrides `backends::detect()`. The order is precision-first and fixed in code; overriding it needs a seam in `shage-core` that `index/fixtures-oracle` was not allowed to cut | `crates/shage-index/src/backends/AGENTS.md` |
| `index/seams` | decide how a backend reaches a worker thread. `IndexBackend: Send` allows a move, not sharing, and every method takes `&self`. Upstream does not share either — the diff-watch worker moves a `Copy` options struct and re-opens the VCS inside the thread (`crates/shage-core/src/app/diff_load.rs:1046`) — but re-opening is cheap for a git handle and expensive for a SCIP index. Three options: a `Sync` bound plus `Arc`, a `handle()` returning a cheap Send clone (what `docs/plan/shage.html` sketches), or open-options re-opened per worker | prompt 04 review of PR #3 |
| `index/classify` | receiver typing in the heuristic pass. `x.bar()` is `Candidates` until something can say what `x` is, and it is the single change that would move `a-inherent-same-name`, `b-generic-bound` and `c-trait-object` off the floor | `crates/shage-index/src/heuristic/AGENTS.md` |
| whichever branch first has a backend carrying commit data | test the two halves of the history contract that are still vacuous: most-recent-first ordering, and `limit` as a hard cap. Neither existing backend can produce a single `CommitRef` to order, so both are asserted by review today | `crates/shage-index/tests/backends.rs` |
| `follow/blast` | add `is_test` to `Blast` for ranking's test de-weighting. `exported` is already on `Blast`; neither belongs on `SymbolRef`, which stays a location | `crates/shage-index/AGENTS.md` |
| `follow/blast` | give `Blast::exported` a source. Both backends report `false` — SCIP carries no visibility and a syntactic pass sees `pub` without knowing whether the module around it is reachable — and false means "not known to be exported", which under-ranks rather than over-ranks | `crates/shage-index/src/scip/AGENTS.md` |
| not a branch — CI | repoint `ci.yml` for the fork: it still runs bare `cargo check`/`clippy`/`test` against `default-members`, so nothing outside `shage-core` is built there. `--workspace` everywhere, plus an oracle job with `rustup component add rust-analyzer` | `xtask/AGENTS.md` |
| not a branch — CI | unpin the Format and Clippy toolchain once `crates/shage-core/src/vcs/jj/mod.rs:181` no longer trips `clippy::chunks_exact_to_as_chunks`, whether upstream fixes it or a `docs/SEAMS.md` row lets us. A pin that outlives its reason is a fork stuck on an old lint set | `.github/workflows/ci.yml` |

Discharged by `index/fixtures-oracle`: `crates/shage-index/src/backends/AGENTS.md`, backend
detection, `CallSite::text`, and the four invariants `NullBackend` satisfied vacuously (two
of them are now real tests; the other two are the history row above, which is honest about
still having nothing to assert against). Also the clippy 1.98 breakage, by pinning the two
lint jobs rather than by editing vendored code — which is why the row above is about
removing the pin, not about the lint.
