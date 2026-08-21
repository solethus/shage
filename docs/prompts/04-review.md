# Review: adversarial self-review before this becomes a pull request

CONTEXT
A change is staged in this repo. You did not write it. Read it as a hostile
reviewer whose job is to find the one thing that will page someone at 3am.

READ FIRST
- docs/prompts/README.md
- the change under review: `git diff --staged` for uncommitted work, or for a stack
  layer `git diff <branch below>...<branch>` (the same diff as
  `gh pr diff <number> --repo solethus/shage`)
- the AGENTS.md of every slice the diff touches
- docs/SEAMS.md

TASK
Produce a review. Use exactly this shape for each finding:

  [ISSUE|SUGGESTION|NOTE|PRAISE] path:line -- one sentence
  Why it matters: ...
  Concrete fix: ...

Then work this checklist explicitly and say "clear" for each one that passes,
so I can see it was actually checked:

  1. Does it violate an invariant listed in a touched slice's AGENTS.md?
  2. Does it touch crates/shage-core without a SEAMS.md entry?
  3. Does it import across a slice boundary without going through contract.rs?
  4. Is any file now over 400 lines?
  5. Does anything crossing an mpsc boundary fail to be owned and Send?
  6. Is any unwrap, expect or panic reachable from untrusted input?
  7. Is there a new dependency, and is it justified?
  8. Does every new public item have an invariant doc comment?
  9. Are the tests falsifiable? Name any test that would still pass against a
     stub implementation -- those are the dangerous ones.
 10. What is the worst possible input to this code, and is there a test for it?
 11. Does any commit or the PR carry Co-Authored-By or other attribution? (must not)
 12. For crates/shage-index: does it depend on shage-core, ratatui or crossterm? (must not)
 13. For a seams PR: does every touched crates/shage-core file have a SEAMS.md row, and
     does `cargo xtask seams --check` pass?

CONSTRAINTS
- Rank ISSUEs first, with line numbers.
- If you find nothing serious, say so plainly. Do not manufacture findings to
  look thorough; a padded review is worse than a short one.
- Do not fix anything. Review only.

OUTPUT
The findings, the checklist, then one paragraph: would you merge this, yes or
no, and the single change that would most improve it.
