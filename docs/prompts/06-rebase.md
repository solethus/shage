# Upstream sync: merge tuicr main into the bounded fork

CONTEXT
crates/shage-core is a bounded fork of tuicr. docs/SEAMS.md lists every file under it we
have modified and why; docs/WORKFLOW.md describes the sync procedure. Upstream has moved.
We merge, we never rebase main: rebasing rewrites history under every open stack and
force-pushes main. Read docs/prompts/README.md first.

TASK
1. `git fetch upstream`, then review `git log --oneline vendor/tuicr-<last>..upstream/main`.
   Summarise upstream's changes in five bullets. Flag anything touching a file listed in
   docs/SEAMS.md, and any new `[profile.*]` in upstream's Cargo.toml (a workspace ignores
   non-root profiles; it would have to be lifted into the root Cargo.toml).
2. Ensure `git config merge.directoryRenames true` (git's default `conflict` stops on every
   new upstream file under src/ because src/ moved to crates/shage-core/src/).
3. Sync on a branch, not on main: `git checkout -b sync/tuicr-<new-version> main`, then
   `git merge upstream/main`. Resolve conflicts one file at a time. For each conflict, state
   in one sentence: what upstream changed, what we changed, and which you kept. Expected
   conflict sites: docs/SEAMS.md rows, root AGENTS.md (keep ours), upstream root files we
   own (README, CI workflows), Cargo.lock (regenerate with `cargo generate-lockfile` only if
   a plain merge fails, then confirm no unexpected dependency changes).
4. If upstream has refactored a seam out of existence -- renamed the enum, restructured the
   poll loop, moved the annotation type -- do NOT invent a workaround. Stop, describe the
   new shape, and we will decide together whether to follow it or reconsider the seam. A
   silently patched seam is how a bounded fork stops being bounded.
5. Refresh the mirror: `git show upstream/main:AGENTS.md > crates/shage-core/AGENTS.md`.
   Move the vendor tag (`git tag vendor/tuicr-<new-version> <upstream sha>`), update the base
   line and any changed rows in docs/SEAMS.md, run `cargo xtask seams --check`.
6. Run the gates (README). Open the PR per README with base main; it must be merged with
   the **merge** method (never squash or rebase), so git keeps the upstream ancestry and
   the next sync does not re-conflict. Say this in the PR body.
7. After it merges: `gh stack sync` on every open stack, and push the vendor tag to origin
   if the seam check runs in CI.

CONSTRAINTS
- Never resolve a conflict by dropping upstream's change to keep ours quiet. If both
  matter, keep both and say so explicitly.
- Do not take the opportunity to refactor anything. Not one line.
- No attribution trailers on the merge commit or the PR.

OUTPUT
The five-bullet upstream summary, the per-conflict decisions, the new SEAMS.md base line,
a clean `cargo test --workspace`, and the PR URL.
