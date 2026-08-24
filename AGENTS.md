# shage

Fork of tuicr (MIT, Almog Gavra) — upstream base `vendor/tuicr-v0.23.1` (commit 0dacb6b =
v0.23.1 + upstream #559) — adding a code index and a follow panel. Upstream lives in
`crates/shage-core/` and is edited ONLY at seams declared in `docs/SEAMS.md`. New code
lives in crates upstream has never heard of.

## Before you change anything
1. Find the directory in `docs/MAP.md`.
2. Read that directory's AGENTS.md — for the fork that is `crates/shage-core/AGENTS.md`,
   upstream's own — and the crate's `src/contract.rs`.
3. Do NOT read the whole repo. If you feel you need to, the boundary is wrong — say so.

## Hard rules
- Any edit under `crates/shage-core/` needs a row in `docs/SEAMS.md`. Never reformat,
  reorder or tidy upstream files; never "fix" upstream code — open an issue instead.
- `crates/shage-index` must never depend on `shage-core` or any TUI crate.
- Cross-crate types live in `contract.rs`. Nowhere else.
- In non-vendored crates: no file over 400 lines, no utils/common/shared/misc.
- No new dependencies without asking.
- Every PR: `cargo fmt --all --check`, `cargo clippy --workspace -- -D warnings`,
  `cargo test --workspace`, `cargo xtask seams --check`.

## Workflow: stacked PRs (gh-stack)
- Never commit to `main`. One story = one `gh stack`; one concern per branch; lower
  layers hold contracts and seams, higher layers hold consumers (stacks are linear).
- Non-interactive only: `gh stack init|add <branch>`, `gh stack submit --auto`,
  `gh stack sync --prune`, `gh stack view --json`, `gh stack merge --yes --squash`.
  On this fork `submit` cannot open PRs — use `gh pr create --repo solethus/shage` per layer.
- Upstream syncs: merge `upstream/main` into `main` (never rebase main), then `gh stack sync`.
  Setup, the full command set and the planned stacks: `docs/WORKFLOW.md`.

## Commands
    cargo build --workspace              # everything; bare cargo = crates/shage-core only
    cargo run -- <args>                  # the TUI (binary: shage)
    cargo test --workspace
    cargo xtask seams                    # stub today; will diff shage-core against the vendor tag
    cargo install --path crates/shage-core

## Where things are
    crates/shage-core     the fork; treat as vendored (package shage, lib crate tuicr)
    crates/shage-index    the index domain; knows no TUI types         (stub)
    crates/shage-follow   panel state, ranking, keybindings             (stub)
    xtask                 seam checking; later fixtures and the oracle
    docs/SEAMS.md         every diverging file under crates/shage-core, with a reason
    docs/MAP.md           one screen, directory → purpose
    docs/WORKFLOW.md      stacked-PR workflow, upstream sync procedure, planned stacks
    docs/prompts          the prompt library (01 bootstrap … 09 spike); keep it current
    docs/plan             dated planning snapshot (interactive HTML) + its corrections

## Not yet
No index, no follow panel, no seams wired. The binary is `shage` but still prints
"tuicr" in --version and the status bar, uses ~/.config/tuicr, and carries upstream's
self-updater — do NOT run `shage update` (it would fetch tuicr binaries). The rename
checklist is later work.
