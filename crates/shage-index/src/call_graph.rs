//! Definitions, edges, and the questions both backends answer from them.
//!
//! A SCIP index and a tree-sitter pass disagree about almost everything — what a symbol is
//! called, how sure they are, whether a macro exists — but they agree on the shape of an
//! answer: a set of definitions, a set of call sites, and a range per definition. Once both
//! have built one of these, five of the seven [`IndexBackend`](crate::contract::IndexBackend)
//! methods are the same code, so they are written once here rather than twice with a
//! bounded walk that drifts.

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use crate::contract::{
    Blast, CallSite, ChangedFile, DiffSymbols, FileCoverage, FileState, SymId, SymbolRef,
};
use crate::enclosing::{DefRange, SymbolTable};

/// Every definition a backend found, and every call between them.
#[derive(Debug, Clone, Default)]
pub struct CallGraph {
    definitions: HashMap<SymId, SymbolRef>,
    callables: SymbolTable,
    calls: Vec<CallSite>,
    seen: BTreeSet<PathBuf>,
}

impl CallGraph {
    /// Records that a backend read this file, whether or not it found anything in it.
    ///
    /// Coverage is not the same question as "does this file define a callable". A file of
    /// nothing but structs and constants was read and understood; saying the index never saw
    /// it sends a reviewer off to install an indexer they already have.
    pub fn observe(&mut self, path: impl Into<PathBuf>) {
        self.seen.insert(path.into());
    }
    /// Records a definition. The last one under a `SymId` loses: a symbol is defined once,
    /// and a second definition of the same identity means the backend built the identity
    /// wrong, which should not silently replace a good answer.
    pub fn define(&mut self, symbol: SymbolRef) {
        self.definitions
            .entry(symbol.sym_id.clone())
            .or_insert(symbol);
    }

    /// Records the lines a callable definition spans, so calls inside it can be attributed
    /// to it. Only callables belong here — a module range would swallow the `use` lines at
    /// the top of a file and turn every import into a call.
    pub fn span(&mut self, range: DefRange) {
        self.callables.insert(range);
    }

    /// Records one call edge.
    pub fn call(&mut self, site: CallSite) {
        self.calls.push(site);
    }

    /// Orders the edges by call site so two runs over one tree produce the same list.
    pub fn finish(mut self) -> Self {
        self.calls
            .sort_by(|a, b| (&a.path, a.line, &a.text).cmp(&(&b.path, b.line, &b.text)));
        self
    }

    /// Where a symbol is defined, if this graph defines it.
    pub fn definition(&self, sym: &SymId) -> Option<&SymbolRef> {
        self.definitions.get(sym)
    }

    /// Every call edge, ordered by call site.
    pub fn calls(&self) -> &[CallSite] {
        &self.calls
    }

    /// The callable definitions, for attributing a line to the symbol that owns it.
    pub fn callables(&self) -> &SymbolTable {
        &self.callables
    }

    /// Whether this graph has anything to say about a file. The difference between "nothing
    /// here calls anything" and "this file was never looked at".
    ///
    /// Answered from the files the backend read, not from the files that hold a callable:
    /// a types-only or consts-only file is covered and simply has no symbols, which is the
    /// opposite conclusion from [`FileCoverage::Missing`].
    pub fn covers(&self, path: &Path) -> bool {
        self.seen.contains(path)
    }

    /// Whether the backend read anything at all. An index that parses and covers no file is
    /// indistinguishable from having no index, and [`crate::backends::detect`] treats it so.
    pub fn covers_anything(&self) -> bool {
        !self.seen.is_empty()
    }

    /// The sites whose resolved target is `sym`. A [`crate::contract::Resolution::Candidates`]
    /// site is not a caller of anything: it names several possibilities and commits to none.
    pub fn callers(&self, sym: &SymId) -> Vec<CallSite> {
        self.caller_sites(sym).cloned().collect()
    }

    /// The sites written inside `sym`'s body.
    pub fn callees(&self, sym: &SymId) -> Vec<CallSite> {
        self.calls
            .iter()
            .filter(|call| &call.from.sym_id == sym)
            .cloned()
            .collect()
    }

    fn caller_sites(&self, sym: &SymId) -> impl Iterator<Item = &CallSite> {
        self.calls
            .iter()
            .filter(move |call| call.target.one().is_some_and(|to| &to.sym_id == sym))
    }

    /// The definitions the changed lines fall inside, plus one entry per file this graph has
    /// never seen.
    ///
    /// Only [`FileCoverage::Missing`] is reported. `Dirty` — edited since indexing — needs
    /// the indexed commit compared against the working tree, and a backend that reported
    /// clean without making that comparison would be guessing in the one direction that
    /// costs a reviewer their trust.
    pub fn symbols_in_diff(&self, files: &[ChangedFile]) -> DiffSymbols {
        let mut symbols = Vec::new();
        let mut seen = HashSet::new();
        let mut uncovered = Vec::new();
        for file in files {
            if !self.covers(&file.path) {
                uncovered.push(FileState {
                    path: file.path.clone(),
                    coverage: FileCoverage::Missing,
                });
                continue;
            }
            for (_, symbol) in self.callables.attribute(file) {
                if let Some(symbol) = symbol
                    && seen.insert(symbol.sym_id.clone())
                {
                    symbols.push(symbol.clone());
                }
            }
        }
        DiffSymbols { symbols, uncovered }
    }

    /// A reverse walk over call edges, bounded at `depth` hops.
    ///
    /// `depth` is echoed onto the result, so a caller can assert the walk was bounded rather
    /// than trusting that it was. A cycle is counted once: `seen` is checked before the
    /// counter moves, not after.
    ///
    /// `exported` is left to the caller. Neither backend can see visibility — SCIP carries
    /// none and a syntactic pass sees `pub` without knowing whether the module around it is
    /// reachable — and inventing it would feed ranking a number nobody measured.
    pub fn blast_radius(
        &self,
        sym: &SymId,
        depth: u8,
        package_of: impl Fn(&SymbolRef) -> String,
    ) -> Blast {
        let mut seen: HashSet<SymId> = HashSet::from([sym.clone()]);
        let mut packages: BTreeSet<String> = BTreeSet::new();
        let mut direct = 0u32;
        let mut transitive = 0u32;
        let mut frontier: VecDeque<(SymId, u8)> = VecDeque::from([(sym.clone(), 0)]);

        while let Some((current, hop)) = frontier.pop_front() {
            if hop >= depth {
                continue;
            }
            for call in self.caller_sites(&current) {
                if !seen.insert(call.from.sym_id.clone()) {
                    continue;
                }
                if hop == 0 {
                    direct += 1;
                } else {
                    transitive += 1;
                }
                packages.insert(package_of(&call.from));
                frontier.push_back((call.from.sym_id.clone(), hop + 1));
            }
        }

        Blast {
            direct,
            transitive,
            packages: packages.into_iter().filter(|p| !p.is_empty()).collect(),
            exported: false,
            depth,
        }
    }
}
