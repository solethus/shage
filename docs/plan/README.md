# Planning snapshot — 2026-08-21

`shage.html` (build plan: feasibility, seams, precision ladder, fork mechanics, repo
structure, the prompt library, testing, first PRs, UI layer) and
`shage-ecosystem-map.html` (inventory: inherited vs. to-build, by phase) are the
interactive pages this repo was bootstrapped from. They are a dated snapshot, not
maintained documentation: when they disagree with `docs/MAP.md`, `docs/SEAMS.md`,
`docs/WORKFLOW.md` or the code, the repo wins. Open them locally in a browser; they
cross-link each other, load fonts from Google Fonts, and are otherwise self-contained.
The prompts they contain are extracted as files under `docs/prompts/`.

Known corrections since the snapshot (verified 2026-08-21):

- Upstream base is tuicr `main` at 0dacb6b (the v0.23.1 release + #559), tagged
  `vendor/tuicr-v0.23.1` — not v0.19.0.
- The fork is package/binary `shage` with library crate `tuicr`, so upstream `src/`
  is untouched. Config/data paths, `--version` and the status bar still say tuicr
  until the rename checklist is done; do not run `shage update` (it still targets
  agavra/tuicr releases).
- Upstream moved things since the plan's seam map was drawn: `App`, `InputMode` and
  `AnnotatedLine` live in `src/app/mod.rs`; `Action` in `src/input/keybindings.rs`;
  the poll loop in `src/main.rs`.
- SCIP: `enclosing_range` is a field of `Occurrence` (the definition occurrence), not
  `SymbolInformation`, and `range`/`enclosing_range` are deprecated in scip 0.9.0 in
  favour of `typed_range`/`typed_enclosing_range`. The 3-vs-4-int rule applies only
  to the deprecated fields.
- `scip-io` exists but is small (7 stars, last push 2026-06) — read it, don't depend on it.
- Upstream syncs: merge `upstream/main` into `main` rather than rebasing `main`, and
  set `git config merge.directoryRenames true` (git's default `conflict` would stop
  on every new upstream file under `src/`). Stacked PRs restack with `gh stack sync`.
- crates.io: `shage` was unclaimed on 2026-08-21 (first-come-first-served, no formal
  squatting rule). Do not publish until README, `repository` and the updater are repointed.
- Work lands as GitHub stacked PRs (`gh stack`), not commits on `main` — see `docs/WORKFLOW.md`.
