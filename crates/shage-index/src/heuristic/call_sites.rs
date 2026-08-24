//! Calls, and how much a syntactic pass may honestly claim about each one.
//!
//! Three answers, and the line between them is *what evidence was used*, never *how many
//! matches came back*:
//!
//! | written | evidence | answer |
//! |---|---|---|
//! | `foo()`, `foo` defined here or imported here | scope | [`Resolution::Heuristic`] |
//! | `Owner::bar()` | the receiver type is written down | [`Resolution::Heuristic`] |
//! | `x.bar()` | none — the receiver's type is unknown | [`Resolution::Candidates`] |
//! | `foo()`, `foo` neither defined nor imported here | none | `Candidates`, or `Unresolved` |
//!
//! A method call never becomes `Heuristic`, even when exactly one method in the tree
//! carries the name. One name match is a weaker claim than scope analysis, and a badge that
//! says otherwise is asking the reviewer to trust arithmetic instead of evidence. Receiver
//! typing is what upgrades this row, and it has its own branch.

use std::path::Path;

use tree_sitter::Node;

use super::definitions::Definitions;
use crate::contract::{CallSite, Resolution};

/// Collects every call written in `root`, attributing each to the function containing it.
///
/// A call outside every function body is skipped rather than attributed upward: a `use`
/// line mentions a function without calling it, and so does a `pub use` that forwards one.
pub fn collect(defs: &Definitions, path: &Path, root: Node, source: &str, out: &mut Vec<CallSite>) {
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        if node.kind() == "call_expression"
            && let Some(call) = resolve(defs, path, node, source)
        {
            out.push(call);
        }
        let mut cursor = node.walk();
        stack.extend(node.named_children(&mut cursor));
    }
}

fn resolve(defs: &Definitions, path: &Path, node: Node, source: &str) -> Option<CallSite> {
    let callee = node.child_by_field_name("function")?;
    // `foo::<T>()` wraps the callee in a generic_function; the name is one level in.
    let callee = match callee.kind() {
        "generic_function" => callee.child_by_field_name("function")?,
        _ => callee,
    };
    let line = callee.start_position().row as u32 + 1;
    let from = defs.callables().enclosing(path, line)?.clone();
    Some(CallSite {
        from,
        path: path.to_path_buf(),
        line,
        text: text(callee, source).to_owned(),
        target: target(defs, path, callee, source),
    })
}

fn target(defs: &Definitions, path: &Path, callee: Node, source: &str) -> Resolution {
    match callee.kind() {
        // `x.bar()` — the receiver's type is not written down, so every method of that name
        // is a candidate and none of them is a conclusion.
        "field_expression" => {
            let Some(name) = callee.child_by_field_name("field").map(|f| text(f, source)) else {
                return Resolution::Unresolved;
            };
            Resolution::candidates(defs.methods_named(name).to_vec())
        }
        // `Owner::bar()` — the receiver type is written at the call site, which is evidence
        // about this call rather than about the name.
        "scoped_identifier" => {
            let Some(name) = callee.child_by_field_name("name").map(|n| text(n, source)) else {
                return Resolution::Unresolved;
            };
            let owner = callee
                .child_by_field_name("path")
                .map(|p| text(p, source))
                .and_then(|p| p.rsplit("::").next());
            if let Some(owner) = owner
                && let [only] = defs.on_type(owner, name)
            {
                return Resolution::Heuristic(only.clone());
            }
            Resolution::candidates(defs.named(name).to_vec())
        }
        "identifier" => in_scope(defs, path, text(callee, source)),
        _ => Resolution::Unresolved,
    }
}

/// Resolves a bare `foo()` through the scope of the file it is written in.
///
/// Being defined in this file, or imported into it by a `use`, is real evidence about
/// *this* call — it is how the compiler would start too. A name that is neither is left as
/// a set of same-named definitions from elsewhere, which is a guess and is badged as one.
fn in_scope(defs: &Definitions, path: &Path, written: &str) -> Resolution {
    if let Some(real) = defs.imported(path, written) {
        if let [only] = defs.named(real) {
            return Resolution::Heuristic(only.clone());
        }
        return Resolution::candidates(defs.named(real).to_vec());
    }
    if let [only] = defs.local_to(path, written) {
        return Resolution::Heuristic(only.clone());
    }
    Resolution::candidates(defs.named(written).to_vec())
}

fn text<'a>(node: Node, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}
