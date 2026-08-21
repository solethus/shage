# Debug: a test is failing and I do not know why

CONTEXT
<paste the failing output here>

Read docs/prompts/README.md first. Known non-bug on this Mac: a link failure with
`undefined symbols: _iconv` from libgit2_sys is the local toolchain -- rerun with
CARGO_TARGET_AARCH64_APPLE_DARWIN_LINKER=/usr/bin/cc before diagnosing anything. Run
tests with `cargo test --workspace`.

TASK
Work in this order and show your reasoning at each step. Do not skip ahead to a
fix.

1. RESTATE the failure in one sentence, in domain terms rather than
   stack-trace terms. "The caller list is empty for a symbol defined in a
   re-exported module", not "assertion failed: left == right".
2. HYPOTHESES -- list three plausible causes, ranked, each with the single
   cheapest observation that would rule it in or out.
3. Take that observation. Add a temporary assertion or a targeted print if you
   need one; say exactly what you added.
4. Only once one hypothesis survives, propose a fix.
5. Before writing it, answer: is the test wrong, or is the code wrong? Say
   which and why. If the test encodes an invariant we no longer believe, that
   is a contract conversation, not a fix.
6. Write the fix plus a regression test that fails without it. Show the test
   failing before and passing after. Commit both on the stack branch that owns the
   code (or a new `fix/<slug>` layer if that PR is already merged); open the PR per
   README; no attribution trailers.

DO NOT
- Change an assertion to make it pass.
- Fix more than one thing. If you find a second bug, note it and leave it.
- Refactor surrounding code while you are in there.
