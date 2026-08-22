//! What a file defines, and what it brings into scope.
//!
//! Everything here is syntax. There is no type checker behind it, so the structures are
//! shaped around the two questions a name-based resolver can honestly answer: *what is
//! defined under this name*, and *is that name in scope where the call is written*.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tree_sitter::Node;

use crate::call_graph::CallGraph;
use crate::contract::{SymId, SymbolRef};
use crate::enclosing::{DefRange, SymbolTable};

/// Every definition and import a syntactic pass found across the tree.
#[derive(Debug, Clone, Default)]
pub struct Definitions {
    callables: SymbolTable,
    by_name: HashMap<String, Vec<SymbolRef>>,
    by_type: HashMap<(String, String), Vec<SymbolRef>>,
    methods: HashMap<String, Vec<SymbolRef>>,
    imports: HashMap<PathBuf, HashMap<String, String>>,
    defined_in: HashMap<PathBuf, HashMap<String, Vec<SymbolRef>>>,
}

impl Definitions {
    /// The callable definitions, for attributing a call to the function that contains it.
    pub fn callables(&self) -> &SymbolTable {
        &self.callables
    }

    /// Copies every definition and its line span into a call graph.
    pub fn seed(&self, graph: &mut CallGraph) {
        for range in self.callables.ranges() {
            graph.define(range.sym.clone());
            graph.span(range.clone());
        }
    }

    /// Every definition under a bare name, anywhere in the tree.
    pub fn named(&self, name: &str) -> &[SymbolRef] {
        self.by_name.get(name).map_or(&[], Vec::as_slice)
    }

    /// Every method under a bare name, on any type. A `x.bar()` call has no receiver type,
    /// so this is the whole honest answer for one.
    pub fn methods_named(&self, name: &str) -> &[SymbolRef] {
        self.methods.get(name).map_or(&[], Vec::as_slice)
    }

    /// Methods named `method` on the type or trait `owner`, from `Owner::method()`.
    pub fn on_type(&self, owner: &str, method: &str) -> &[SymbolRef] {
        self.by_type
            .get(&(owner.to_owned(), method.to_owned()))
            .map_or(&[], Vec::as_slice)
    }

    /// The real name behind a name written in `file`, following a `use … as …` rename.
    ///
    /// Returns `None` when `file` imports nothing under that name, which is the difference
    /// between "this name is in scope" and "a definition somewhere happens to share it".
    pub fn imported(&self, file: &Path, written: &str) -> Option<&str> {
        self.imports.get(file)?.get(written).map(String::as_str)
    }

    /// Definitions of `name` written in `file` itself.
    pub fn local_to(&self, file: &Path, name: &str) -> &[SymbolRef] {
        self.defined_in
            .get(file)
            .and_then(|names| names.get(name))
            .map_or(&[], Vec::as_slice)
    }

    /// Records everything `tree` defines and imports for `path`.
    pub fn absorb(&mut self, path: &Path, root: Node, source: &str) {
        let mut scope = Scope::default();
        self.visit(path, root, source, &mut scope);
    }

    fn visit(&mut self, path: &Path, node: Node, source: &str, scope: &mut Scope) {
        match node.kind() {
            "function_item" | "function_signature_item" => {
                if let Some(name) = field_text(node, "name", source) {
                    self.record(path, node, name, scope.owner.clone());
                }
            }
            "impl_item" => {
                // `impl Task for One` has both fields; the owner is the type, because that
                // is what `One::run()` is written against.
                let owner = field_text(node, "type", source)
                    .or_else(|| field_text(node, "trait", source))
                    .map(str::to_owned);
                return self.descend(path, node, source, &mut Scope { owner });
            }
            "trait_item" => {
                let owner = field_text(node, "name", source).map(str::to_owned);
                return self.descend(path, node, source, &mut Scope { owner });
            }
            "use_declaration" => {
                self.absorb_use(path, node, source);
                return;
            }
            _ => {}
        }
        self.descend(path, node, source, scope);
    }

    fn descend(&mut self, path: &Path, node: Node, source: &str, scope: &mut Scope) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.visit(path, child, source, scope);
        }
    }

    fn record(&mut self, path: &Path, node: Node, name: &str, owner: Option<String>) {
        let first_line = node.start_position().row as u32 + 1;
        let symbol = SymbolRef {
            // Opaque and unique: a definition is identified by where it is written, which
            // is the only identity a syntactic pass can honestly claim.
            sym_id: SymId::new(format!("{}:{}:{name}", path.display(), first_line)),
            display: name.to_owned(),
            path: path.to_path_buf(),
            line: first_line,
        };
        self.callables.insert(DefRange {
            sym: symbol.clone(),
            first_line,
            last_line: node.end_position().row as u32 + 1,
        });
        self.by_name
            .entry(name.to_owned())
            .or_default()
            .push(symbol.clone());
        self.defined_in
            .entry(path.to_path_buf())
            .or_default()
            .entry(name.to_owned())
            .or_default()
            .push(symbol.clone());
        if let Some(owner) = owner {
            self.methods
                .entry(name.to_owned())
                .or_default()
                .push(symbol.clone());
            self.by_type
                .entry((owner, name.to_owned()))
                .or_default()
                .push(symbol);
        }
    }

    /// Records `use a::b::c;` as `c → c` and `use a::b::c as d;` as `d → c`.
    ///
    /// Only the leaf matters: the path in between is module structure this pass cannot
    /// follow, and pretending to follow it is how a re-export chain turns into a confident
    /// wrong edge.
    fn absorb_use(&mut self, path: &Path, node: Node, source: &str) {
        let entry = self.imports.entry(path.to_path_buf()).or_default();
        let mut cursor = node.walk();
        let mut stack: Vec<Node> = node.named_children(&mut cursor).collect();
        while let Some(current) = stack.pop() {
            match current.kind() {
                "use_as_clause" => {
                    let real = field_text(current, "path", source).and_then(last_segment);
                    let alias = field_text(current, "alias", source);
                    if let (Some(real), Some(alias)) = (real, alias) {
                        entry.insert(alias.to_owned(), real.to_owned());
                    }
                }
                "scoped_identifier" | "identifier" => {
                    if let Some(leaf) = last_segment(node_text(current, source)) {
                        entry.insert(leaf.to_owned(), leaf.to_owned());
                    }
                }
                _ => {
                    let mut inner = current.walk();
                    stack.extend(current.named_children(&mut inner));
                }
            }
        }
    }
}

/// The `impl` or `trait` a definition is written inside, if any.
#[derive(Debug, Clone, Default)]
struct Scope {
    owner: Option<String>,
}

fn node_text<'a>(node: Node, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}

fn field_text<'a>(node: Node, field: &str, source: &'a str) -> Option<&'a str> {
    node.child_by_field_name(field)
        .map(|child| node_text(child, source))
}

/// The last `::`-separated segment of a path, which is the name it binds.
fn last_segment(path: &str) -> Option<&str> {
    path.rsplit("::")
        .next()
        .map(str::trim)
        .filter(|s| !s.is_empty())
}
