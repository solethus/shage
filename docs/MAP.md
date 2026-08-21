# Map — directory → purpose

| path | purpose |
|---|---|
| `Cargo.toml` | Workspace root (virtual manifest, resolver 3). `default-members` is `crates/shage-core`, so bare `cargo build/test/run/clippy` behave exactly like upstream; use `--workspace` for everything. |
| `crates/shage-core/` | The fork: upstream tuicr, vendored at `vendor/tuicr-v0.23.1`. Package + binary `shage`, library crate `tuicr`. Edit only at seams listed in `docs/SEAMS.md`. Its `AGENTS.md` is upstream's own. |
| `crates/shage-core/src/` | Upstream source: `app/` (App, InputMode, AnnotatedLine), `input/` (keybindings, Action), `ui/`, `vcs/` (git/jj/hg), `forge/` (GitHub/GitLab/Bitbucket/Azure), `update/` (self-updater), `config/`, `theme/`, … |
| `crates/shage-core/tests/`, `crates/shage-core/examples/` | Upstream fixtures and example theme files; tests reach them by relative path, so they moved with the crate. |
| `crates/shage-index/` | The index domain: symbols, edges, freshness. Stub. Must never depend on `shage-core` or any TUI crate. |
| `crates/shage-follow/` | The follow panel: cursor → symbol, follow stack, ranking. Stub. |
| `xtask/` | Repo automation: `cargo xtask seams` (stub; will diff `crates/shage-core` against the vendor tag). |
| `.cargo/config.toml` | The `cargo xtask` alias. |
| `.impeccable/config.json` | Design-hook config: `docs/plan/**` is ignored because the pages there are a frozen snapshot, not maintained UI. |
| `docs/` | `SEAMS.md` (declared divergence from upstream), `MAP.md` (this file), `WORKFLOW.md` (stacked PRs, upstream syncs), plus upstream's user docs (`CONFIG.md`, `KEYBINDINGS.md`, `REVIEW_CLI.md`, forge guides). |
| `docs/plan/` | Dated planning snapshot: the interactive build plan and ecosystem map (HTML) plus `README.md` with the corrections found since. The repo wins on disagreement. |
| `docs/prompts/` | The prompt library (`01-bootstrap` … `09-spike`) plus `README.md` (repo facts, stack workflow, prompt-to-branch order); versioned with the code, fixed when a prompt produces a bad result. `02-contract.md` is pre-filled for the next run (`index/contract`). |
| `skills/tuicr/` | Upstream's agent skill bundle (still tuicr-named; the rename is a follow-up). |
| `.github/workflows/` | Upstream CI. `ci.yml`/`release.yml` minimally patched for the new layout; `build_nix.yml` unverified; release and self-update plumbing still target agavra/tuicr — do not run `shage update`. |
| `scripts/demo/`, `public/` | Upstream demo recording tooling and assets (`record-demo.sh` still builds `--bin tuicr`). |
| `README.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, `RELEASE.md`, `PLAN.md`, `cliff.toml`, `flake.nix` | Upstream root files, untouched; they still describe tuicr (`PLAN.md` is upstream's plan, not ours). |
| `.tuicrignore` | This repo's own review-ignore file, read by the binary when reviewing this repo. |
