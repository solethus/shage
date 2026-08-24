//! The symbol attributed to a line always contains that line. Never a neighbour.
//!
//! This is the bug that makes every badge in the panel subtly wrong and goes unnoticed for
//! a month: a definition ends on line 40, a call sits on line 41, and the call is credited
//! to the function above it. Hand-picked ranges never find it, because whoever picks them
//! picks the cases they already thought of — so the ranges here are generated, including
//! the nested, adjacent, single-line and identical ones nobody would think to write down.

use std::path::{Path, PathBuf};

use proptest::prelude::*;
use shage_index::contract::{ChangedFile, SymId, SymbolRef};
use shage_index::enclosing::{DefRange, SymbolTable};

/// Definitions as (first line, span). Spans overlap and nest freely, which is what real
/// Rust does: a method inside an `impl` inside a `mod` is three ranges over one line.
fn definitions() -> impl Strategy<Value = Vec<(u32, u32)>> {
    prop::collection::vec((1u32..120, 0u32..40), 0..20)
}

/// Hunks as (first line, span), the new-side ranges of a changed file.
fn hunks() -> impl Strategy<Value = Vec<(u32, u32)>> {
    prop::collection::vec((1u32..140, 0u32..12), 0..6)
}

fn table(defs: &[(u32, u32)], path: &Path) -> SymbolTable {
    let mut table = SymbolTable::new();
    for (index, (first_line, span)) in defs.iter().enumerate() {
        table.insert(DefRange {
            sym: SymbolRef {
                // Unique per definition, so a result can be traced back to the range it
                // came from rather than to a same-named neighbour.
                sym_id: SymId::new(format!("def-{index}")),
                display: format!("def_{index}"),
                path: path.to_path_buf(),
                line: *first_line,
            },
            first_line: *first_line,
            last_line: first_line + span,
        });
    }
    table
}

fn range_of(defs: &[(u32, u32)], sym: &SymId) -> (u32, u32) {
    let index: usize = sym
        .as_str()
        .strip_prefix("def-")
        .and_then(|n| n.parse().ok())
        .expect("every generated symbol id is def-<index>");
    let (first_line, span) = defs[index];
    (first_line, first_line + span)
}

proptest! {
    /// The core invariant, at every line whether or not a definition covers it.
    #[test]
    fn attributed_symbol_contains_the_line(defs in definitions(), line in 1u32..200) {
        let path = PathBuf::from("src/generated.rs");
        let table = table(&defs, &path);

        match table.enclosing(&path, line) {
            Some(symbol) => {
                let (first, last) = range_of(&defs, &symbol.sym_id);
                prop_assert!(
                    first <= line && line <= last,
                    "line {line} attributed to a definition spanning {first}..={last}"
                );
            }
            None => {
                let covering = defs.iter().find(|(f, s)| *f <= line && line <= f + s);
                prop_assert!(
                    covering.is_none(),
                    "line {line} reported unattributed while {covering:?} covers it"
                );
            }
        }
    }

    /// When several definitions contain the line, the narrowest wins — a method is not
    /// swallowed by the `impl` block around it.
    #[test]
    fn the_innermost_definition_wins(defs in definitions(), line in 1u32..200) {
        let path = PathBuf::from("src/generated.rs");
        let table = table(&defs, &path);

        let Some(symbol) = table.enclosing(&path, line) else {
            return Ok(());
        };
        let (first, last) = range_of(&defs, &symbol.sym_id);
        let chosen = last - first;
        let narrower = defs
            .iter()
            .filter(|(f, s)| *f <= line && line <= f + s)
            .any(|(_, s)| *s < chosen);
        prop_assert!(!narrower, "a narrower definition also contains line {line}");
    }

    /// The same holds for every line of a hunk, which is how the panel actually asks.
    #[test]
    fn every_hunk_line_is_attributed_or_none(defs in definitions(), spans in hunks()) {
        let path = PathBuf::from("src/generated.rs");
        let table = table(&defs, &path);
        let file = ChangedFile {
            path: path.clone(),
            new_lines: spans.iter().map(|(f, s)| *f..=(f + s)).collect(),
        };

        let attributed = table.attribute(&file);
        let expected: usize = spans.iter().map(|(_, s)| *s as usize + 1).sum();
        prop_assert_eq!(attributed.len(), expected, "one answer per changed line");

        for (line, symbol) in attributed {
            let Some(symbol) = symbol else {
                continue;
            };
            let (first, last) = range_of(&defs, &symbol.sym_id);
            prop_assert!(
                first <= line && line <= last,
                "hunk line {} attributed to {}..={}", line, first, last
            );
        }
    }

    /// A file the table has never seen attributes nothing, rather than borrowing another
    /// file's ranges.
    #[test]
    fn an_unknown_file_attributes_nothing(defs in definitions(), line in 1u32..200) {
        let table = table(&defs, Path::new("src/generated.rs"));
        prop_assert!(table.enclosing(Path::new("src/elsewhere.rs"), line).is_none());
    }
}
