# Seams — where crates/shage-core differs from upstream

Upstream: tuicr (MIT, Almog Gavra) — https://github.com/agavra/tuicr
Base: tag `vendor/tuicr-v0.23.1` = commit 0dacb6b = the v0.23.1 release + upstream #559 (`git describe`: v0.23.1-1-g0dacb6b), upstream `main` on 2026-08-20.

Layout: upstream's repo root became `crates/shage-core/` (`Cargo.toml`, `src/`, `tests/`, `examples/` moved as pure renames). Everything under `crates/shage-core/` is byte-identical to the base except the files below. Any other edit under `crates/shage-core/` needs a row here — that is the whole rule.

| file | why |
|---|---|
| `crates/shage-core/Cargo.toml` | package renamed `tuicr` → `shage` (the binary becomes `shage`), version reset to 0.1.0, explicit `[lib] name = "tuicr"` so upstream `src/` compiles unmodified, and `repository`/`homepage` repointed at this fork. Name asymmetry on purpose: package/bin `shage`, library crate `tuicr`, CLI still prints `tuicr 0.1.0`, `cargo doc` → `target/doc/tuicr/`. The `repository` change is load-bearing rather than cosmetic: `src/update/install/source.rs` builds every release URL from `CARGO_PKG_REPOSITORY`, so on agavra/tuicr the version reset makes upstream 0.23.x look like an upgrade and `shage update` replaces the running binary with upstream's. Pointed at the fork it 404s instead. The startup check still polls `crates.io/api/v1/crates/shage` (`CARGO_PKG_NAME`), a name this fork does not own — a wasted request today, and a false banner if the name is ever taken, but the download behind it now resolves against the fork and cannot install upstream |
| `crates/shage-core/AGENTS.md` | verbatim copy of upstream's root AGENTS.md at the base — the slice doc for the vendored code. Refresh from upstream on every sync; never edit |
| `crates/shage-core/src/update/install/tests.rs` | three assertions hardcoded upstream's `https://github.com/agavra/tuicr` where the code under test derives it from `CARGO_PKG_REPOSITORY`; they now derive it the same way, so they follow the manifest instead of pinning the URL the manifest no longer has. The first assertion in that test already read it from the environment — this makes the other three consistent. Falls out of the `repository` seam above and has no other cause; if an upstream sync ever makes these derive the URL themselves, drop this row |

Check: `cargo xtask seams`, which runs exactly this and fails on anything not in the table above:
`git diff -M --name-status vendor/tuicr-v0.23.1 HEAD | grep -v '^R100' | grep crates/shage-core/` must list exactly the files above. No pathspec — it would disable rename detection.
The table and `DECLARED_SEAMS` in `xtask/src/main.rs` are the same list; a new row needs both.
Blob checks, by hand: `git diff vendor/tuicr-v0.23.1:Cargo.toml HEAD:crates/shage-core/Cargo.toml` and `git diff --quiet vendor/tuicr-v0.23.1:AGENTS.md HEAD:crates/shage-core/AGENTS.md`.
