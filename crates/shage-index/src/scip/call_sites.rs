//! Which occurrences are calls, and what evidence each one carries.
//!
//! Three rules decide, and all three come from the indexer rather than from a guess here:
//!
//! 1. the occurrence does not carry the `Definition` role — it refers, it does not declare;
//! 2. its symbol is declared callable *and defined in this index*;
//! 3. a callable definition encloses the line it sits on.
//!
//! Rule 3 is what keeps `use inner::real as alias;` from becoming two call edges into
//! `real`. Both tokens on that line refer to a function, neither is inside a function body,
//! and an import is not a call. A `pub use` chain's forwarding references go the same way,
//! which is why re-export depth costs this slice nothing.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use scip::types::Document;

use super::definitions::Callable;
use super::occurrences::{is_definition, name_span};
use crate::call_graph::CallGraph;
use crate::contract::{CallSite, Resolution, SymId};

/// Records every call written in `document` into `graph`.
///
/// Definitions must already be absorbed for every document, including the ones this call
/// reaches into: a call in `main.rs` targeting `alpha.rs` has no target until `alpha.rs`
/// has been read.
pub fn absorb(
    graph: &mut CallGraph,
    document: &Document,
    callable: &Callable,
    sources: &mut SourceCache,
) {
    let path = PathBuf::from(&document.relative_path);
    for occurrence in &document.occurrences {
        if is_definition(occurrence.symbol_roles) || !callable.contains(&occurrence.symbol) {
            continue;
        }
        let Some(target) = graph.definition(&SymId::new(&occurrence.symbol)).cloned() else {
            continue; // declared callable but defined elsewhere: a dependency, not an edge
        };
        let Some(site) = name_span(occurrence) else {
            continue;
        };
        let Some(from) = graph.callables().enclosing(&path, site.first_line).cloned() else {
            continue; // rule 3: not inside a function body, so not a call
        };
        let text = sources.slice(document, &path, site.first_line, &occurrence.range);
        graph.call(CallSite {
            from,
            path: path.clone(),
            line: site.first_line,
            text,
            target: Resolution::Exact(target),
        });
    }
}

/// The source text behind an occurrence, so a badge can show its evidence.
///
/// ponytail: one read per file that contains a call, cached for the life of the build. The
/// indexer's own `Document::text` is preferred when it populated it; rust-analyzer does not.
pub struct SourceCache {
    root: PathBuf,
    lines: HashMap<PathBuf, Vec<String>>,
}

impl SourceCache {
    /// A cache resolving document paths against a repository root.
    pub fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            lines: HashMap::new(),
        }
    }

    fn slice(&mut self, document: &Document, path: &Path, line: u32, range: &[i32]) -> String {
        let root = &self.root;
        let lines = self.lines.entry(path.to_path_buf()).or_insert_with(|| {
            let text = if document.text.is_empty() {
                fs::read_to_string(root.join(path)).unwrap_or_default()
            } else {
                document.text.clone()
            };
            text.lines().map(str::to_owned).collect()
        });
        let Some(source) = lines.get(line as usize - 1) else {
            return String::new();
        };
        // Columns are byte offsets under UTF-8 and code units under UTF-16, and the index
        // says which. Rather than convert, take the slice only when it lands on character
        // boundaries and fall back to the whole line when it does not: evidence that is too
        // wide is honest, evidence chopped mid-character is not.
        match range {
            [_, start, end] | [_, start, _, end] if end > start && *start >= 0 => source
                .get(*start as usize..*end as usize)
                .map(str::to_owned)
                .unwrap_or_else(|| source.trim().to_owned()),
            _ => source.trim().to_owned(),
        }
    }
}
