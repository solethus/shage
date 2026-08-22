//! The backend for a project that will not build.

use std::fs;
use std::path::{Path, PathBuf};

use tree_sitter::Parser;

use super::call_sites;
use super::definitions::Definitions;
use crate::call_graph::CallGraph;
use crate::contract::{
    Blast, CallSite, ChangedFile, CommitRef, DiffSymbols, IndexBackend, IndexError, IndexStamp,
    Resolution, Result, Stamped, SymId, SymbolRef,
};

/// Answers by parsing the source, with no compiler behind it.
///
/// Nothing it returns is [`Resolution::Exact`]. It sees text, so the strongest claim
/// available to it is that a name is in scope where it is written — good enough to be
/// useful on a tree that will not build, and never good enough to be badged as certain.
#[derive(Debug, Clone)]
pub struct HeuristicBackend {
    graph: CallGraph,
    stamp: IndexStamp,
}

impl HeuristicBackend {
    /// Parses every `.rs` file under `root`.
    ///
    /// The stamp reports no `indexed_commit`, and that is the truth rather than a gap: this
    /// backend re-reads the working tree every time it opens, so there is no indexed commit
    /// to be stale against. `repo_commit` still says where the tree is.
    pub fn open(root: &Path) -> Result<Self> {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_rust::LANGUAGE.into())
            .map_err(|err| IndexError::Unavailable(format!("tree-sitter rust: {err}")))?;

        let files = rust_files(root);
        let mut sources = Vec::with_capacity(files.len());
        let mut defs = Definitions::default();
        for path in files {
            let Ok(source) = fs::read_to_string(root.join(&path)) else {
                continue; // unreadable or not UTF-8: skipped, never fatal
            };
            let Some(tree) = parser.parse(&source, None) else {
                continue;
            };
            defs.absorb(&path, tree.root_node(), &source);
            sources.push((path, source, tree));
        }

        // Calls are collected only after every file's definitions are in: a call in
        // `main.rs` reaching into `alpha.rs` has no target until `alpha.rs` has been read.
        let mut calls = Vec::new();
        for (path, source, tree) in &sources {
            call_sites::collect(&defs, path, tree.root_node(), source, &mut calls);
        }

        let mut graph = CallGraph::default();
        defs.seed(&mut graph);
        for call in calls {
            graph.call(call);
        }

        Ok(Self {
            graph: graph.finish(),
            stamp: IndexStamp {
                indexed_commit: None,
                repo_commit: crate::backends::head_commit(root),
                overlay: false,
            },
        })
    }

    /// Every call edge found, for a caller grading one backend against another.
    pub fn calls(&self) -> &[CallSite] {
        self.graph.calls()
    }

    fn answer<T>(&self, value: T) -> Stamped<T> {
        Stamped::new(self.stamp.clone(), value)
    }
}

impl IndexBackend for HeuristicBackend {
    fn stamp(&self) -> Result<IndexStamp> {
        Ok(self.stamp.clone())
    }

    fn symbols_in_diff(&self, files: &[ChangedFile]) -> Result<Stamped<DiffSymbols>> {
        Ok(self.answer(self.graph.symbols_in_diff(files)))
    }

    /// A definition this pass found, badged [`Resolution::Heuristic`] rather than `Exact`.
    ///
    /// Even a definition it read straight out of the source is a syntactic claim: a `cfg`
    /// it cannot evaluate may mean the item is not in the build at all.
    fn definition(&self, sym: &SymId) -> Result<Stamped<Resolution>> {
        let resolution = self
            .graph
            .definition(sym)
            .cloned()
            .map_or(Resolution::Unresolved, Resolution::Heuristic);
        Ok(self.answer(resolution))
    }

    fn callers(&self, sym: &SymId) -> Result<Stamped<Vec<CallSite>>> {
        Ok(self.answer(self.graph.callers(sym)))
    }

    fn callees(&self, sym: &SymId) -> Result<Stamped<Vec<CallSite>>> {
        Ok(self.answer(self.graph.callees(sym)))
    }

    /// Structurally impossible: a parse of the working tree holds no commit data.
    fn history(&self, _sym: &SymId, _limit: usize) -> Result<Stamped<Vec<CommitRef>>> {
        Err(IndexError::Unsupported("heuristic: no commit data"))
    }

    fn blast_radius(&self, sym: &SymId, depth: u8) -> Result<Stamped<Blast>> {
        Ok(self.answer(self.graph.blast_radius(sym, depth, module_of)))
    }
}

/// The directory a symbol's file sits in — the closest thing to a module a syntactic pass
/// can name without resolving the module tree, which is the very thing it cannot do.
fn module_of(symbol: &SymbolRef) -> String {
    symbol
        .path
        .parent()
        .map_or_else(String::new, |dir| dir.display().to_string())
}

/// Every `.rs` file under `root`, repository-relative and sorted.
///
/// `target/` and `.git/` are skipped: one holds build output including generated sources
/// that are nobody's code, the other holds no code at all. Sorted so two runs over the same
/// tree build the same graph, since ties in `by_name` are resolved by insertion order.
fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name();
            if path.is_dir() {
                if !matches!(name.to_str(), Some("target" | ".git")) {
                    stack.push(path);
                }
            } else if path.extension().is_some_and(|ext| ext == "rs")
                && let Ok(relative) = path.strip_prefix(root)
            {
                found.push(relative.to_path_buf());
            }
        }
    }
    found.sort();
    found
}
