# Seams — where crates/shage-core differs from upstream

Upstream: tuicr (MIT, Almog Gavra) — https://github.com/agavra/tuicr
Base: tag `vendor/tuicr-v0.23.1` = commit 0dacb6b = the v0.23.1 release + upstream #559 (`git describe`: v0.23.1-1-g0dacb6b), upstream `main` on 2026-08-20.

Layout: upstream's repo root became `crates/shage-core/` (`Cargo.toml`, `src/`, `tests/`, `examples/` moved as pure renames). Everything under `crates/shage-core/` is byte-identical to the base except the files below. Any other edit under `crates/shage-core/` needs a row here — that is the whole rule.

| file | why |
|---|---|
| `crates/shage-core/Cargo.toml` | package renamed `tuicr` → `shage` (the binary becomes `shage`), version reset to 0.1.0, explicit `[lib] name = "tuicr"` so upstream `src/` compiles unmodified. Name asymmetry on purpose: package/bin `shage`, library crate `tuicr`, CLI still prints `tuicr 0.1.0`, `cargo doc` → `target/doc/tuicr/` |
| `crates/shage-core/AGENTS.md` | verbatim copy of upstream's root AGENTS.md at the base — the slice doc for the vendored code. Refresh from upstream on every sync; never edit |

Check (no pathspec — it would disable rename detection):
`git diff -M --name-status vendor/tuicr-v0.23.1 HEAD | grep -v '^R100' | grep crates/shage-core/` must list exactly the files above.
Blob checks: `git diff vendor/tuicr-v0.23.1:Cargo.toml HEAD:crates/shage-core/Cargo.toml` (three hunks) and `git diff --quiet vendor/tuicr-v0.23.1:AGENTS.md HEAD:crates/shage-core/AGENTS.md`.
`cargo xtask seams --check` will automate this; today it is a stub.
