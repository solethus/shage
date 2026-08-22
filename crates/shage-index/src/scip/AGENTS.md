# slice: scip

Reads an `index.scip` produced by rust-analyzer, scip-go, scip-typescript or scip-clang.
Turns protobuf into the contract types. Knows nothing about diffs, review sessions,
terminals or ranking, and names no programming language.

Every resolution it produces is `Resolution::Exact`. A compiler frontend already did the
work that would otherwise be a guess, and repeating it worse here would only add a way to
disagree.

## Invariants you must not break

- **`load()` is pure.** Bytes in, `Index` out. No network, no subprocess, no mutation, no
  file system — the caller supplies the bytes, which is what makes a truncated or hostile
  index testable without one existing on disk.
- **Occurrence ranges are SCIP-encoded: three ints is a single-line range, four is
  multi-line. Never assume four.** Assuming four shifts every single-line occurrence's end
  into the next line's columns, and every badge downstream is then attributed to a
  neighbour. An unrecognised length is `None`, never a guess.
- **A symbol string is opaque.** Never parse it to guess a name; `SymbolInformation
  .display_name` is the display source. The one exception is `scip::symbol::parse_symbol`
  for the package field, which is the grammar's own parser reading a structured field —
  not a regex scraping a name.
- **`external_symbols` are dependencies.** They have no document, so they can never be a
  caller. Filtered early.
- **An index file is untrusted input.** It arrives from whatever indexer the user installed,
  at whatever version. Malformed bytes are `IndexError::Corrupt`, never a panic: a bad index
  must degrade the panel, not take the TUI down.
- **`history` is `Unsupported`, never empty.** An empty list reads as "no commit ever touched
  this symbol", which is a far stronger claim than "this format cannot say".

## What counts as a call

Three rules, all three from the indexer rather than from a guess here:

1. the occurrence does not carry the `Definition` role — it refers, it does not declare;
2. its symbol is declared callable (`Function`, `Method`, `TraitMethod`, `StaticMethod`)
   **and defined in this index**;
3. a callable definition encloses the line it sits on.

Rule 3 carries more weight than it looks. `use inner::real as alias;` produces two
occurrences, both referring to a function, neither inside a function body — and an import is
not a call. A `pub use` chain's forwarding references go the same way, which is why
re-export depth costs this slice nothing.

`syntax_kind` is not used. rust-analyzer leaves it `UnspecifiedSyntaxKind` on every
occurrence, so a filter built on it would silently match nothing. Checked against real
output, not against the specification.

## What lives where

    load.rs          bytes -> Index. Nothing else.
    occurrences.rs   range decoding and role predicates. Roles are a bit set, not a value.
    definitions.rs   Definition-role occurrences -> definitions and callable ranges.
    call_sites.rs    reference occurrences -> CallSites, and the source slice behind each.
    backend.rs       ScipBackend: the seven trait methods, over a shared CallGraph.
    mod.rs           re-exports; no logic.

## Tests

Unit tests for the two things that are wrong most often: range decoding at every legal and
illegal shape, and the role bit set. Everything else is exercised end to end by
`cargo xtask oracle`, which runs the real indexer over generated fixtures — the encoding is
the indexer's, so a hand-written `.scip` would be asserting against a fiction.

**Never commit a `.scip` file.** It drifts from the real encoding as the indexer version
moves, and then the tests assert a museum piece while production reads something else.

## Decisions

- **`display` is the bare identifier**, because that is what `display_name` holds. Two `run`
  methods on different types share it, so everything that must tell them apart uses
  `path:line` — which is where they actually differ, and is also the oracle's comparison key.
- **`CallSite::text` is read from the source file**, one read per file that contains a call,
  cached. Naming it from the target instead would be a lie in exactly the aliased-import
  case: at `alias()` the token is `alias` and the target's name is `real`.
- **`Blast::exported` is always false.** SCIP carries no visibility. False means "not known
  to be exported", which under-ranks rather than over-ranks; a guess would feed ranking a
  number nobody measured.
