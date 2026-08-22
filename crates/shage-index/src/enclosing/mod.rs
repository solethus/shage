//! Which definition encloses a line.
//!
//! Both tiers need this and neither owns it: SCIP hands back occurrences that must be
//! attributed to the definition containing them, and the tree-sitter pass has to do the
//! same for every call it finds. It is also the one place an off-by-one silently poisons
//! every badge in the panel, so it is a slice with a property test rather than a helper.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::contract::{ChangedFile, SymbolRef};

/// A definition and the lines its body spans.
///
/// Both bounds are 1-based and inclusive, matching [`SymbolRef::line`] and the new-side
/// line numbers in [`ChangedFile`], so no caller ever converts between two conventions.
/// A single-line definition has `first_line == last_line`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefRange {
    /// The definition itself. Its `line` is where the name is written, which is inside
    /// `first_line..=last_line` but not necessarily equal to `first_line` — an attribute or
    /// a doc comment can precede it.
    pub sym: SymbolRef,
    /// First line of the definition, 1-based inclusive.
    pub first_line: u32,
    /// Last line of the definition, 1-based inclusive. Never less than `first_line`.
    pub last_line: u32,
}

impl DefRange {
    /// Whether `line` falls inside this definition, both bounds included.
    pub fn contains(&self, line: u32) -> bool {
        self.first_line <= line && line <= self.last_line
    }

    /// How many lines the definition spans. Used to pick the innermost of several
    /// containing definitions, so a method never loses to the `impl` block around it.
    pub fn span(&self) -> u32 {
        self.last_line - self.first_line
    }
}

/// Every definition a backend knows, grouped by file.
///
/// Deliberately not sorted on insert: a backend builds one of these in one pass and then
/// only reads it, so ordering work belongs in [`SymbolTable::enclosing`] where it is
/// needed, not on every push.
#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    by_path: BTreeMap<PathBuf, Vec<DefRange>>,
}

impl SymbolTable {
    /// An empty table. A backend with no index has one of these, not a `None`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a definition, keyed by its own `sym.path`.
    ///
    /// Ranges may nest freely — a method inside an `impl` inside a `mod` is three
    /// overlapping entries, and [`SymbolTable::enclosing`] resolves them to the innermost.
    pub fn insert(&mut self, def: DefRange) {
        self.by_path
            .entry(def.sym.path.clone())
            .or_default()
            .push(def);
    }

    /// The innermost definition containing `line` in `path`, or `None`.
    ///
    /// Never a neighbour: the returned definition's range contains `line`, always. When
    /// several contain it the narrowest wins, and ties break on the later `first_line` so
    /// the answer is stable across two runs over the same file.
    pub fn enclosing(&self, path: &Path, line: u32) -> Option<&SymbolRef> {
        // ponytail: linear scan over one file's definitions. Rust nests three deep in
        // practice; sort + binary search if a file ever holds thousands of them.
        self.by_path
            .get(path)?
            .iter()
            .filter(|def| def.contains(line))
            .min_by_key(|def| (def.span(), std::cmp::Reverse(def.first_line)))
            .map(|def| &def.sym)
    }

    /// Every definition recorded for `path`, in insertion order.
    pub fn defs_in(&self, path: &Path) -> &[DefRange] {
        self.by_path.get(path).map_or(&[], Vec::as_slice)
    }

    /// Every definition in the table, grouped by file and in insertion order within a file.
    pub fn ranges(&self) -> impl Iterator<Item = &DefRange> {
        self.by_path.values().flatten()
    }

    /// Every file the table holds definitions for.
    pub fn paths(&self) -> impl Iterator<Item = &PathBuf> {
        self.by_path.keys()
    }

    /// The definition each changed line belongs to, one entry per line in `file.new_lines`.
    ///
    /// `None` means the line is outside every definition — a `use` at the top of the file,
    /// a blank line between two functions — and is returned as such rather than being
    /// attributed to whichever definition happens to be nearest.
    pub fn attribute(&self, file: &ChangedFile) -> Vec<(u32, Option<&SymbolRef>)> {
        file.new_lines
            .iter()
            .flat_map(|range| range.clone())
            .map(|line| (line, self.enclosing(&file.path, line)))
            .collect()
    }
}
