# slice: heuristic

Resolving by parsing the source, for a tree that will not build.

The fallback tier, and the reason `IndexBackend` exists: a repository with no resolvable
`Cargo.lock`, no `node_modules` and no `compile_commands.json` still gets a panel, and every
badge on it says what it is worth.

## Invariants you must not break

- **Nothing here is ever `Resolution::Exact`.** It sees text. Even a definition read straight
  out of the source is a syntactic claim — a `cfg` it cannot evaluate may mean the item is
  not in the build at all.
- **A method call is never `Heuristic`.** `x.bar()` is every `bar` in the tree, badged
  `Candidates`, however many that turns out to be. One name match is a weaker claim than
  scope analysis, and a badge that says otherwise asks the reviewer to trust arithmetic
  instead of evidence.
- **The line between `Heuristic` and `Candidates` is what evidence was used, never how many
  matches came back.** Being defined in this file or imported into it is evidence about
  *this* call. Sharing a name with something in another file is not.
- **A call outside every function body is skipped, not attributed upward.** A `use` line
  mentions a function without calling it, and so does a `pub use` that forwards one.
- **Unparseable and unreadable files are skipped, never fatal.** A tree that will not build
  is the premise; a tree that will not parse is the same premise, one file at a time.

## What it resolves

| written | evidence | answer |
|---|---|---|
| `foo()`, `foo` defined here or imported here | scope | `Heuristic` |
| `Owner::bar()` | the receiver type is written down | `Heuristic` |
| `x.bar()` | none — the receiver's type is unknown | `Candidates` |
| `foo()`, `foo` neither defined nor imported here | none | `Candidates`, or `Unresolved` |

## What it is known to get wrong

Recorded rather than patched over, because the differential oracle measures each one and the
alternative is guessing. A confident wrong edge costs more than an honest absence.

- **A call inside a macro invocation is invisible.** `println!("{}", f())` is a `token_tree`
  to tree-sitter, not an expression, so `f` is not called as far as this pass can see.
- **A macro-generated definition does not exist.** `make!(generated, 9)` defines `generated`
  only after expansion, so calling it resolves to nothing. Fixture `f-macro-generated`.
- **A receiver has no type.** Fixtures `a-inherent-same-name`, `b-generic-bound` and
  `c-trait-object` all land here, which is why their recall is the lowest of the seven.
- **`cfg` is not evaluated.** An item excluded from the build is still a definition here.
- **A `use` path is followed only to its leaf.** The modules in between are structure this
  pass cannot resolve, and pretending to follow them is how a re-export chain becomes a
  confident wrong edge.

## What lives where

    definitions.rs   the tree walk: what a file defines, and what it brings into scope.
    call_sites.rs    call expressions, and how much may honestly be claimed about each.
    backend.rs       HeuristicBackend, the file walk, the seven trait methods.
    mod.rs           re-exports; no logic.

Definitions for every file are collected before any calls are, because a call in `main.rs`
reaching into `alpha.rs` has no target until `alpha.rs` has been read. Files are visited in
sorted order so two runs over one tree build the same graph.

## Carry-overs this slice owes

- Receiver typing, in `index/classify`. It is the single change that would move `x.bar()`
  off `Candidates`, and it is what three of the seven fixtures are waiting for.

## Tests

The oracle. Asserting by hand on what a resolver *should* say about trait dispatch is how
you write down your own misunderstanding and then defend it; `cargo xtask oracle` asks
rust-analyzer instead. Node kinds were read off real parse trees rather than taken from the
grammar's documentation — `method_call_expression` does not exist in tree-sitter-rust, and a
method call is a `call_expression` whose callee is a `field_expression`.
