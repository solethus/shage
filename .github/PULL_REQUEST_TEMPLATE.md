<!--
Title: conventional-commit style, imperative, <= 72 chars, e.g. `feat(index): load index.scip occurrences`.
One concern per PR; a bigger change is a stack of PRs (docs/WORKFLOW.md). Delete the guidance
comments, keep the headings. "None" is a valid answer for a section.
-->

## Summary
<!-- One or two sentences: what this PR does and the visible outcome. -->

## Why
<!-- The problem or plan item it addresses. Link the plan item, issue or decision record
     (docs/plan, docs/DECISIONS) if there is one. -->

## What changed
<!-- Bullets grouped by crate or directory. Call out anything under crates/shage-core
     explicitly - every such file needs a docs/SEAMS.md row. -->

## How it was verified
<!-- Commands you actually ran and their results (test counts, before/after numbers,
     snapshot output). Screenshots or rendered text for UI changes. -->

## Risks and follow-ups
<!-- Behaviour changes, anything deliberately left red or stale, what the next PR covers. -->

## Stack
<!-- Base branch and the PR below this one ("stacked on #12"), merge order, merge method
     (squash for features and docs; merge for upstream syncs). -->

## Checklist
- [ ] `cargo fmt --all --check`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`, `cargo xtask seams --check`
- [ ] No edits under `crates/shage-core/`, or every touched file has a `docs/SEAMS.md` row
- [ ] Docs updated in this PR: `docs/MAP.md` for new directories, the slice `AGENTS.md`, `docs/WORKFLOW.md` if the process changed
- [ ] Tests cover each invariant the change introduces; no `TODO` left behind
- [ ] No `Co-Authored-By` or other attribution trailers in commits or this description
