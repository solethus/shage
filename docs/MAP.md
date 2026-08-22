# Map — directory → purpose

| path | purpose |
|---|---|
| `Cargo.toml` | Workspace root (virtual manifest, resolver 3). `default-members` is `crates/shage-core`, so bare `cargo build/test/run/clippy` behave exactly like upstream; use `--workspace` for everything. |
| `crates/shage-core/` | The fork: upstream tuicr, vendored at `vendor/tuicr-v0.23.1`. Package + binary `shage`, library crate `tuicr`. Edit only at seams listed in `docs/SEAMS.md`. Its `AGENTS.md` is upstream's own. |
| `crates/shage-core/src/` | Upstream source: `app/` (App, InputMode, AnnotatedLine), `input/` (keybindings, Action), `ui/`, `vcs/` (git/jj/hg), `forge/` (GitHub/GitLab/Bitbucket/Azure), `update/` (self-updater), `config/`, `theme/`, … |
| `crates/shage-core/tests/`, `crates/shage-core/examples/` | Upstream fixtures and example theme files; tests reach them by relative path, so they moved with the crate. |
| `crates/shage-index/` | The index domain: symbols, edges, freshness. Must never depend on `shage-core` or any TUI crate. Its `AGENTS.md` is the crate contract; each slice below carries its own. |
| `crates/shage-index/src/contract/` | The types that cross a crate boundary, the `IndexBackend` trait, and nothing else. No I/O, no file format, no language. |
| `crates/shage-index/src/enclosing/` | Which definition encloses a line. Shared by both backends; the subject of the line-attribution property test. |
| `crates/shage-index/src/scip/` | Reads an `index.scip` from rust-analyzer, scip-go, scip-typescript or scip-clang. Every answer is `Exact`. |
| `crates/shage-index/src/heuristic/` | The tree-sitter pass, for a tree that will not build. Never `Exact`; its known gaps are measured by the oracle. |
| `crates/shage-index/src/backends/` | `detect()` — Scip → Heuristic → Null, never errors, never blocks startup — plus `NullBackend`. |
| `crates/shage-follow/` | The follow panel: cursor → symbol, follow stack, ranking. Panel and freshness line only so far; the stack, ranking and empty states are the `follow/*` branches. |
| `xtask/` | Repo automation: `cargo xtask seams` (stub), `fixtures` (the hazard projects), `oracle` (the differential harness). Depends on `shage-index` and nothing else. |
| `xtask/oracle.baseline` | The committed floor the oracle fails against. A floor, not an equality; `cargo xtask oracle --bless` rewrites it. |
| `target/fixtures/` | Generated cargo projects, one per resolution hazard. Never committed — a checked-in `.scip` drifts from the real encoding and the tests then assert a museum piece. |
| `.cargo/config.toml` | The `cargo xtask` alias. |
| `.impeccable/config.json` | Design-hook config: `docs/plan/**` is ignored because the pages there are a frozen snapshot, not maintained UI. |
| `docs/` | `SEAMS.md` (declared divergence from upstream), `MAP.md` (this file), `WORKFLOW.md` (stacked PRs, upstream syncs), plus upstream's user docs (`CONFIG.md`, `KEYBINDINGS.md`, `REVIEW_CLI.md`, forge guides). |
| `docs/plan/` | Dated planning snapshot: the interactive build plan and ecosystem map (HTML) plus `README.md` with the corrections found since. The repo wins on disagreement. |
| `docs/prompts/` | The prompt library (`01-bootstrap` … `09-spike`) plus `README.md` (repo facts, stack workflow, prompt-to-branch order); versioned with the code, fixed when a prompt produces a bad result. `02-contract.md` is pre-filled for the next run (`index/contract`). |
| `skills/tuicr/` | Upstream's agent skill bundle (still tuicr-named; the rename is a follow-up). |
| `.github/` | `PULL_REQUEST_TEMPLATE.md` (ours); `workflows/` is upstream CI — `ci.yml`/`release.yml` minimally patched for the new layout with `ci.yml` repointed at `--workspace` and its Format, Clippy and oracle jobs pinned to a Rust version (the comments there say why, and the oracle's reason is not the lint jobs'), `build_nix.yml` unverified; release and self-update plumbing still target agavra/tuicr — do not run `shage update`. |
| `scripts/demo/`, `public/` | Upstream demo recording tooling and assets (`record-demo.sh` still builds `--bin tuicr`). |
| `README.md`, `CHANGELOG.md`, `CONTRIBUTING.md`, `RELEASE.md`, `PLAN.md`, `cliff.toml`, `flake.nix` | Upstream root files, untouched; they still describe tuicr (`PLAN.md` is upstream's plan, not ours). |
| `.tuicrignore` | This repo's own review-ignore file, read by the binary when reviewing this repo. |
