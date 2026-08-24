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
| 02 → 03 | `index/scip` | load index.scip, roles, enclosing-definition tree |
| 05 | `index/fixtures-oracle` | `cargo xtask fixtures` / `cargo xtask oracle` |
| 02 → 03 → 04 | `index/classify` | call-position classification, Unknown stays Unknown |
| 02 → 03 → 04 | `index/callgraph` | edges, bounded walks, `shage index callers` CLI |
| 02 → 03 → 04 | `follow/panel` | the follow panel — first user-visible feature (new stack) |
| 02 → 03 | `follow/freshness` | stamps and the four empty states |
| 02 → 03 → 04 | `follow/blast` | blast radius and the ranked file tree |

Next up: `02-contract.md` carries a pre-filled "next run" block for `index/contract`.
