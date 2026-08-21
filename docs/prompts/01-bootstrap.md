# Bootstrap: restructure the fork into a bounded workspace

CONTEXT
This repo is a fork of tuicr (MIT, Almog Gavra) at tag vendor/tuicr-v0.23.1,
renamed to shage. It is a synchronous ratatui code-review TUI. I am adding a
code index and a follow panel. Upstream is still active, so I must be able to
rebase weekly with conflicts confined to a small, declared set of files.

READ FIRST -- and nothing else yet
- Cargo.toml
- src/main.rs and src/app.rs        (the event loop and the state shape)
- src/vcs/traits.rs, src/forge/traits.rs   (the two existing backend traits)
- AGENTS.md                          (upstream's; I am replacing it)

TASK
Convert this single-crate repo into a cargo workspace WITHOUT changing any
behaviour.
1. Move the existing crate to crates/shage-core/ unchanged. Rename the package
   and the binary to shage. Reset the version to 0.1.0.
2. Create crates/shage-index/ and crates/shage-follow/, each with a lib.rs and
   an empty contract.rs. shage-index must NOT depend on shage-core.
3. Create xtask/ with a "seams" subcommand stub that exits 0.
4. Write docs/SEAMS.md listing every file under crates/shage-core/ that differs
   from vendor/tuicr-v0.23.1. Right now that should be Cargo.toml alone.
5. Write docs/MAP.md: one screen, a table of directory to one-line purpose.
6. Replace AGENTS.md with a router of at most 60 lines.

CONSTRAINTS
- Zero behaviour change. cargo test must pass identically before and after.
- Do not reformat, reorder or tidy any upstream file. Every gratuitous edit is
  a future rebase conflict. If rustfmt wants to touch an upstream file, leave
  it and tell me.
- Do not add dependencies in this change.

OUTPUT
One commit, plus a summary listing exactly which upstream files were touched
and why.

DONE WHEN
- cargo build --workspace and cargo test --workspace both pass
- git diff vendor/tuicr-v0.23.1 --stat shows only renames plus Cargo.toml
- docs/SEAMS.md matches that diff exactly

DO NOT
- Start implementing the index. That is a later prompt.
- Improve anything you notice in upstream code. Open an issue instead.
