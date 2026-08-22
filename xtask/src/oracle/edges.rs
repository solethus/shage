//! Reading one backend's answers into a set both backends can be compared on.
//!
//! The two backends do not agree on symbol identity — SCIP symbol strings and the
//! syntactic pass's `path:line:name` ids are not comparable, and neither is anyone's to
//! change — so an edge is normalised to **locations**: the call site, and the definition it
//! reaches. That is also what "A calls B" means to a reviewer reading the panel.
//!
//! A site is a `(file, line)` pair rather than a column, because the two backends anchor a
//! call differently: SCIP marks the identifier token, tree-sitter marks the whole callee
//! expression, so `Owner::bar()` starts in two different columns. Comparing *sets of edges*
//! rather than one target per site keeps two calls on one line — `f() + g()` — distinct
//! without depending on either anchor.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use shage_index::contract::{ChangedFile, IndexBackend, Resolution, Result, SymbolRef};

/// A file and a 1-based line: where a call is written, or where a definition is.
pub type Loc = (PathBuf, u32);

/// One resolved edge: a call site, and the definition it reaches.
pub type Edge = (Loc, Loc);

/// What one backend says about a tree, reduced to what another can be compared against.
#[derive(Debug, Default)]
pub struct Edges {
    /// Edges the backend committed to — `Exact` or `Heuristic`.
    pub resolved: BTreeSet<Edge>,
    /// Sites where it offered several targets and committed to none.
    pub ambiguous: BTreeMap<Loc, BTreeSet<Loc>>,
    /// Sites it saw but could not resolve at all.
    pub unresolved: BTreeSet<Loc>,
    /// Display name per location, for the disagreement report. Never a comparison key: two
    /// `run` methods share a display name, and the location is what tells them apart.
    pub names: BTreeMap<Loc, String>,
    /// The function each call site sits in — the `A` of "expected A -> B".
    pub callers: BTreeMap<Loc, Loc>,
}

impl Edges {
    /// A location as `name (path:line)`, or just the location when the backend had no name.
    pub fn describe(&self, at: &Loc) -> String {
        let (path, line) = at;
        match self.names.get(at) {
            Some(name) if !name.is_empty() => format!("{name} ({}:{line})", path.display()),
            _ => format!("{}:{line}", path.display()),
        }
    }

    fn record(&mut self, symbol: &SymbolRef) -> Loc {
        let at = (symbol.path.clone(), symbol.line);
        self.names.insert(at.clone(), symbol.display.clone());
        at
    }
}

/// Reads every edge a backend can reach, through the trait and nothing else.
///
/// Both tiers go through this same function. The oracle never special-cases one, so a
/// backend that lands later is graded by the code that already exists rather than by code
/// written to flatter it.
pub fn of(backend: &dyn IndexBackend, files: &[ChangedFile]) -> Result<Edges> {
    let mut edges = Edges::default();
    for symbol in backend.symbols_in_diff(files)?.value.symbols {
        for call in backend.callees(&symbol.sym_id)?.value {
            let site = (call.path.clone(), call.line);
            let from = edges.record(&call.from);
            edges.callers.insert(site.clone(), from);
            edges.names.entry(site.clone()).or_insert(call.text.clone());
            match &call.target {
                Resolution::Exact(to) | Resolution::Heuristic(to) => {
                    let to = edges.record(to);
                    edges.resolved.insert((site, to));
                }
                Resolution::Candidates(set) => {
                    let targets: BTreeSet<Loc> = set
                        .as_slice()
                        .iter()
                        .map(|candidate| edges.record(candidate))
                        .collect();
                    edges.ambiguous.entry(site).or_default().extend(targets);
                }
                Resolution::Unresolved => {
                    edges.unresolved.insert(site);
                }
            }
        }
    }
    Ok(edges)
}

/// One `.rs` file per entry, every line marked changed, so `symbols_in_diff` returns
/// everything a backend knows rather than a slice of it.
pub fn whole_tree(root: &std::path::Path) -> Vec<ChangedFile> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if !matches!(entry.file_name().to_str(), Some("target" | ".git")) {
                    stack.push(path);
                }
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let lines = std::fs::read_to_string(&path)
                    .map(|text| text.lines().count().max(1) as u32)
                    .unwrap_or(1);
                if let Ok(relative) = path.strip_prefix(root) {
                    files.push(ChangedFile {
                        path: relative.to_path_buf(),
                        new_lines: vec![1..=lines],
                    });
                }
            }
        }
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    files
}
