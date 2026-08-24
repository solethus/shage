# slice: enclosing

Which definition encloses a line.

Both backends need this and neither owns it. SCIP hands back occurrences that must be
attributed to the definition containing them; the syntactic pass has to do the same for
every call it finds. Putting it inside either one would mean writing it twice, and the
second copy is where the off-by-one lives.

## Why this is a slice and not a helper

It is the single place a one-line error poisons every badge in the panel. A definition ends
on line 40, a call sits on line 41, the call is credited to the function above it, and
nothing looks wrong until someone follows a caller into the wrong function. That failure
mode is worth a named slice with a property test rather than three lines inside a loop.

## Invariants you must not break

- **The returned definition contains the line.** Never a neighbour, never the nearest.
- **`None` means no definition contains the line**, not "none was close enough". A `use` at
  the top of a file belongs to nothing, and saying so is the answer.
- **The innermost wins.** A method inside an `impl` inside a `mod` is three containing
  ranges; the narrowest is the answer, so a method is never swallowed by its `impl`.
- **Ties are stable.** Two ranges of equal width containing the same line resolve the same
  way on every run, so two passes over one file build the same graph.
- **Both bounds are 1-based and inclusive**, matching `SymbolRef::line` and `ChangedFile`.
  No caller ever converts between two conventions; SCIP's 0-based ranges are converted once,
  in `scip/occurrences.rs`.

## What lives where

    mod.rs    DefRange, SymbolTable, enclosing(), attribute(). Nothing else.

## Tests

`tests/attribution.rs`, with generated ranges. Hand-picked ranges only ever cover the cases
whoever picked them already thought of; the generator produces the nested, adjacent,
single-line and identical ranges that nobody writes down. Four properties: the returned
range contains the line, `None` means nothing covers it, the innermost wins, and an unknown
file attributes nothing.

The test has been shown to fail: changing `contains` to `first_line <= line + 1` — the
classic off-by-one — shrinks to `line 66 attributed to a definition spanning 67..=67`.

## Decisions

- **Linear scan, not an interval tree.** Rust nests three deep in practice and the scan is
  per file, not per repository. The `ponytail:` comment names the upgrade path so nobody has
  to rediscover that the choice was deliberate.
- **Only callables go in the table.** Both backends insert function-shaped definitions and
  nothing else, which is what makes "the line is inside a function body" the same test as
  "something encloses it". A module range would cover a file's `use` lines and turn every
  import into a call.
