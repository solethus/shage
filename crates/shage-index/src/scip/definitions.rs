//! Which symbols a SCIP index defines, and where.

use std::collections::HashMap;
use std::path::PathBuf;

use scip::symbol::is_local_symbol;
use scip::types::{Document, Index, SymbolInformation, symbol_information::Kind};

use super::occurrences::{decode_range, is_definition};
use crate::call_graph::CallGraph;
use crate::contract::{SymId, SymbolRef};
use crate::enclosing::DefRange;

/// The symbols an index declares callable, with the indexer's own display names.
///
/// A call target has to be a function of some kind. Filtering on kind is what stops
/// `use alpha::Alpha;` and `impl Beta` — both references to a symbol from inside a file
/// this crate indexes — from being read as calls into a type.
#[derive(Debug, Clone, Default)]
pub struct Callable {
    names: HashMap<String, String>,
}

impl Callable {
    /// Every function, method, trait method and static method the index declares.
    ///
    /// External symbols are recorded too, so the callable test stays honest for a call into
    /// a dependency — but they carry no document, so they never gain a definition and the
    /// edge is dropped for want of one. They can never be a *caller* either, which is why
    /// they are filtered before anything walks them.
    pub fn of(index: &Index) -> Self {
        let mut names = HashMap::new();
        let declared = index
            .documents
            .iter()
            .flat_map(|document| &document.symbols)
            .chain(&index.external_symbols);
        for info in declared {
            if let Some((symbol, display)) = callable_name(info) {
                names.insert(symbol, display);
            }
        }
        Self { names }
    }

    /// Whether the index declares this symbol callable.
    pub fn contains(&self, symbol: &str) -> bool {
        self.names.contains_key(symbol)
    }

    /// The indexer's own display name, never a name scraped out of the symbol string.
    ///
    /// It is the bare identifier — two `run` methods on different types share it — so
    /// anything that must tell them apart uses `path:line`, which is where they differ.
    pub fn display(&self, symbol: &str) -> String {
        self.names.get(symbol).cloned().unwrap_or_default()
    }
}

fn callable_name(info: &SymbolInformation) -> Option<(String, String)> {
    if is_local_symbol(&info.symbol) {
        return None;
    }
    matches!(
        info.kind.enum_value_or_default(),
        Kind::Function | Kind::Method | Kind::TraitMethod | Kind::StaticMethod
    )
    .then(|| (info.symbol.clone(), info.display_name.clone()))
}

/// Records every definition in `document` into `graph`.
///
/// Local symbols are skipped: `local 0` means something different in every document, so a
/// table keyed by symbol string would have them collide across files.
pub fn absorb(graph: &mut CallGraph, document: &Document, callable: &Callable) {
    let path = PathBuf::from(&document.relative_path);
    for occurrence in &document.occurrences {
        if !is_definition(occurrence.symbol_roles) || is_local_symbol(&occurrence.symbol) {
            continue;
        }
        let Some(name) = decode_range(&occurrence.range) else {
            continue;
        };
        let symbol = SymbolRef {
            sym_id: SymId::new(&occurrence.symbol),
            display: callable.display(&occurrence.symbol),
            path: path.clone(),
            line: name.first_line,
        };
        if callable.contains(&occurrence.symbol) {
            // `enclosing_range` covers the whole definition, signature and body. The name
            // range alone would leave every call in the body attributed to whatever
            // encloses the function instead of to the function.
            let body = decode_range(&occurrence.enclosing_range).unwrap_or(name);
            graph.span(DefRange {
                sym: symbol.clone(),
                first_line: body.first_line,
                last_line: body.last_line,
            });
        }
        graph.define(symbol);
    }
}
