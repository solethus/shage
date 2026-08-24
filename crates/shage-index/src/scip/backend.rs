//! The backend that answers from a SCIP index.

use std::fs;
use std::path::Path;

use super::call_sites::{self, SourceCache};
use super::definitions::{self, Callable};
use super::load;
use crate::call_graph::CallGraph;
use crate::contract::{
    Blast, CallSite, ChangedFile, CommitRef, DiffSymbols, IndexBackend, IndexError, IndexStamp,
    Resolution, Result, Stamped, SymId, SymbolRef,
};

/// Answers from an index a compiler frontend produced.
///
/// Every resolution is [`Resolution::Exact`]: the frontend already did the work that would
/// otherwise be a guess, and repeating it worse here would only add a way to disagree.
#[derive(Debug, Clone)]
pub struct ScipBackend {
    graph: CallGraph,
    stamp: IndexStamp,
}

/// The extension `open` looks for beside `index.scip` to learn the commit that produced it.
///
/// SCIP records no commit — the format has nowhere to put one — so whoever runs the indexer
/// writes `index.scip.commit`. See [`ScipBackend::open`] for what happens without it.
pub const COMMIT_SIDECAR: &str = "commit";

impl ScipBackend {
    /// Loads `index_path`, resolving document paths against the repository at `root`.
    ///
    /// `indexed_commit` is the caller's to supply because **this backend cannot know it and
    /// will not invent it**: SCIP carries no commit, so a stamp taken from whatever HEAD
    /// happens to be at load time would claim a freshness nobody checked. `None` renders as
    /// "no index", which understates freshness rather than overstating it — the only safe
    /// direction for the stamp a reviewer decides how far to trust an empty panel by.
    pub fn open(index_path: &Path, root: &Path, indexed_commit: Option<String>) -> Result<Self> {
        let index = load(&fs::read(index_path)?)?;
        let callable = Callable::of(&index);
        let mut graph = CallGraph::default();
        for document in &index.documents {
            definitions::absorb(&mut graph, document, &callable);
        }
        let mut sources = SourceCache::new(root);
        for document in &index.documents {
            call_sites::absorb(&mut graph, document, &callable, &mut sources);
        }
        Ok(Self {
            graph: graph.finish(),
            stamp: IndexStamp {
                indexed_commit,
                repo_commit: crate::backends::head_commit(root),
                overlay: false,
            },
        })
    }

    /// The commit written beside `index_path` by whoever ran the indexer, if any.
    pub fn sidecar_commit(index_path: &Path) -> Option<String> {
        let commit = fs::read_to_string(index_path.with_extension(COMMIT_SIDECAR)).ok()?;
        let commit = commit.trim();
        (!commit.is_empty()).then(|| commit.to_owned())
    }

    /// Every call edge the index holds, for a caller grading one backend against another.
    pub fn calls(&self) -> &[CallSite] {
        self.graph.calls()
    }

    /// Whether the index covers any file at all.
    ///
    /// Zero bytes decode to a well-formed index of nothing, so "it opened" is not evidence
    /// that there is an index behind it. [`crate::backends::detect`] asks this before
    /// preferring this tier over one that reads the source.
    pub fn covers_anything(&self) -> bool {
        self.graph.covers_anything()
    }

    fn answer<T>(&self, value: T) -> Stamped<T> {
        Stamped::new(self.stamp.clone(), value)
    }
}

impl IndexBackend for ScipBackend {
    fn stamp(&self) -> Result<IndexStamp> {
        Ok(self.stamp.clone())
    }

    fn symbols_in_diff(&self, files: &[ChangedFile]) -> Result<Stamped<DiffSymbols>> {
        Ok(self.answer(self.graph.symbols_in_diff(files)))
    }

    fn definition(&self, sym: &SymId) -> Result<Stamped<Resolution>> {
        let resolution = self
            .graph
            .definition(sym)
            .cloned()
            .map_or(Resolution::Unresolved, Resolution::Exact);
        Ok(self.answer(resolution))
    }

    fn callers(&self, sym: &SymId) -> Result<Stamped<Vec<CallSite>>> {
        Ok(self.answer(self.graph.callers(sym)))
    }

    fn callees(&self, sym: &SymId) -> Result<Stamped<Vec<CallSite>>> {
        Ok(self.answer(self.graph.callees(sym)))
    }

    /// Structurally impossible: a SCIP index holds no commit data.
    ///
    /// An empty list would read as "no commit ever touched this symbol", which is a
    /// different and far stronger claim than "this format cannot say".
    fn history(&self, _sym: &SymId, _limit: usize) -> Result<Stamped<Vec<CommitRef>>> {
        Err(IndexError::Unsupported("scip: no commit data"))
    }

    fn blast_radius(&self, sym: &SymId, depth: u8) -> Result<Stamped<Blast>> {
        Ok(self.answer(self.graph.blast_radius(sym, depth, package_of)))
    }
}

/// The package a symbol belongs to, via SCIP's own symbol parser.
///
/// This is not the banned kind of parsing. The ban is on scraping a *name* out of a symbol
/// string — `display` already answers that — whereas a package is a structured field of the
/// grammar, read by the grammar's own parser. When the string does not parse, the file's
/// directory is the honest fallback: a location, which is all this was going to be.
fn package_of(symbol: &SymbolRef) -> String {
    if let Ok(parsed) = scip::symbol::parse_symbol(symbol.sym_id.as_str())
        && let Some(package) = parsed.package.into_option()
        && !package.name.is_empty()
    {
        return package.name;
    }
    symbol
        .path
        .parent()
        .map_or_else(String::new, |dir| dir.display().to_string())
}
