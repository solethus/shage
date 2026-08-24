# Spike: measure before you commit to a design

CONTEXT
Before building <THING>, I want a number, not an opinion. Timebox: <N> hours.
Throwaway code is fine and expected -- this branch will be deleted. Work on a plain
`spike/<slug>` branch cut from the current stack tip or main; it is never opened as a
PR. Read docs/prompts/README.md for the local linker quirk before measuring anything.

TASK
1. State the question as a falsifiable claim with a threshold. For example: "a
   warm callers query on a 400k-line workspace returns in under 50ms".
2. Build the smallest possible thing that produces that number. Hardcode
   everything. No abstractions, no error handling, no tests, no configuration.
3. Measure on real input, not a synthetic one. Name the repository and its
   size in lines and files.
4. Report: the number, the conditions, the variance across five runs, and the
   one thing most likely to make it worse in production.
5. Answer the claim: true, false, or "the question was wrong, and here is the
   better one". The third answer is the most valuable and the most commonly
   skipped.

DO NOT
- Clean it up. A tidy spike invites reuse, and spike code in production is how
  you acquire a load-bearing hack.
- Continue past the timebox. Report what you have, even if it is partial.
- Generalise. One number, one question.
- Open a PR or merge it. The report is the deliverable; if the number changes a
  decision, it goes into docs/DECISIONS/ via prompt 08.
