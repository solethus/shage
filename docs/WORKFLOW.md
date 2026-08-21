# Workflow — stacked PRs with gh-stack

Work lands on `main` only through pull requests, and multi-step work is a **stack**:
an ordered chain of branches, each one PR whose base is the branch below it.

## One-time setup
    gh extension install github/gh-stack
    git config rerere.enabled true          # remember conflict resolutions
    git config remote.pushDefault origin    # two remotes exist (origin, upstream)
    git config merge.directoryRenames true  # new upstream files under src/ follow the move

## Rules
- Never commit to `main`. Start a stack: `gh stack init <area>/<slug>`; add a layer:
  `gh stack add <area>/<slug>`. Branch names are used verbatim.
- One story = one stack; one concern per branch; ideally one logical commit per branch
  (the commit subject becomes the PR title under `submit --auto`).
- Stacks are strictly linear: lower layers hold contracts and seams, higher layers hold
  consumers. If a higher layer needs a lower-layer change: `gh stack down`, commit there,
  `gh stack rebase --upstack`, `gh stack top`.
- Non-interactive only: `gh stack submit --auto` (draft PRs; add `--open` for ready),
  `gh stack view --json`, `gh stack sync --prune`, `gh stack merge --yes --squash`,
  `gh stack unstack` to restructure. No attribution trailers on commits or PRs.
- Every PR, before submit: `cargo fmt --all --check`, `cargo clippy --workspace -- -D warnings`,
  `cargo test --workspace`, `cargo xtask seams --check`.
- Known limitation (2026-08-21, gh-stack v0.1.0): on this fork `gh stack submit`/`sync`/`link`
  push branches fine but cannot create or find PRs — they resolve PRs against the parent repo
  (agavra/tuicr). Open each PR explicitly instead, bottom to top, with a description that
  follows `.github/PULL_REQUEST_TEMPLATE.md` (GitHub only auto-applies it for PRs against
  `main` once it is there, so pass it explicitly):
  `gh pr create --repo solethus/shage --draft --base <branch below> --head <branch> --title "<subject>" --body-file <file>`,
  then `gh pr ready <n> --repo solethus/shage` once the gates pass. Local stack tracking
  (`init`/`add`/`view --json`/`rebase`) still works. Revisit when the extension handles forks,
  or once the repo is no longer marked as a fork.

## Upstream syncs (weekly) — prompt 06
1. `git fetch upstream`, then on a branch `sync/tuicr-vX.Y.Z` cut from `main`:
   `git merge upstream/main` — merge, never rebase `main`; conflicts are confined to the
   files in `docs/SEAMS.md` plus root files we own (AGENTS.md, README, CI). Refresh the
   mirror: `git show upstream/main:AGENTS.md > crates/shage-core/AGENTS.md`.
2. Move the vendor tag (`vendor/tuicr-vX.Y.Z`), update `docs/SEAMS.md`, run
   `cargo xtask seams --check`; open the PR and merge it with the **merge** method (never
   squash or rebase) so the upstream ancestry survives for the next sync.
3. `gh stack sync` on each open stack to restack it onto the new `main`.

## Planned stacks (from docs/plan/shage.html, "first 10 PRs" — prompt-to-branch table in docs/prompts/README.md)
- `bootstrap/*` — workspace split → planning docs (this stack).
- `index/*` — contract + NullBackend → the six seams → scip → fixtures + oracle → classify → callgraph + CLI.
- `follow/*` — follow panel → freshness states → blast radius + ranked file tree.
