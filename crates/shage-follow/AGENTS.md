# shage-follow — the follow panel

Panel state, ranking and keybindings. A view over what `shage-index` answered, never the
other way round: the index knows nothing about terminals, and this crate is where its
answers become rows.

Stub-plus-one: only what the snapshot tests needed a subject for. The follow stack, ranking,
the four empty states and the overlay stack are `follow/panel`, `follow/freshness` and
`follow/blast`.

## Invariants you must not break

- **A row never outlives its evidence.** A `cand` badge is drawn beside the call expression
  that was ambiguous. When the width runs out, the caller's name is what gets shortened —
  never the evidence, and never the location.
- **The panel is sized from the content outward.** A bordered block takes a column from each
  side and a row from top and bottom, so a 120x24 frame leaves 118x22 and 80x24 leaves 78x22.
  Every width is computed from `Block::inner`, never from the area passed in. Laying rows out
  against the frame is how a panel that reads fine on a wide terminal writes over its own
  border on a narrow one; `crates/shage-core/AGENTS.md` records upstream hitting the same
  trap with its modals.
- **A shortened value says so.** Truncation ends in `…`, so a cut value is never mistaken for
  a complete one.
- **The stamp is not optional.** A list of call sites with no freshness line beside it cannot
  tell a reviewer whether an empty panel means "nothing calls this" or "the index never saw
  this commit", and those are opposite conclusions.
- **No literal colours.** Nothing here names an RGB value or an ANSI index. Styling is
  `Modifier` only until the theme's `Role` enum lands — two hundred literal colours at call
  sites is a theme system that has to be rewritten by hand.

## What lives where

    src/contract.rs   FollowPanel, the freshness line, the badge words. Cross-crate types.
    src/panel.rs      the ratatui widget and its layout.

## Tests

`tests/panel_snapshot.rs`, at **120x24 and 80x24**, asserting on rendered text. Both sizes
are the test: either alone proves nothing, because the interesting behaviour is the
difference between them. One row is deliberately long enough to fit at 118 columns and not at
78 — without it the narrow snapshot would assert that nothing happens.
