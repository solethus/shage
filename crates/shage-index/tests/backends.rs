//! The invariants `NullBackend` could only satisfy vacuously.
//!
//! An empty backend upholds "paths are repository-relative" and "packages are sorted and
//! deduplicated" by never returning a path or a package. These run the same assertions
//! against a backend that actually answers, which is the first point at which they can
//! fail.
//!
//! The heuristic backend is the one under test here because it needs no external indexer:
//! it parses the tree it is pointed at, so the whole file runs in `cargo test` without
//! rust-analyzer installed. The SCIP backend is exercised end to end by
//! `cargo xtask oracle`, which needs the real indexer and is therefore not a unit test.

use std::fs;
use std::path::{Path, PathBuf};

use shage_index::backends;
use shage_index::contract::{IndexBackend, IndexError, SymId};
use shage_index::heuristic::HeuristicBackend;

/// A tree with a call chain four deep, so a bounded walk has something to be bounded by.
const TREE: &str = "\
pub fn leaf() -> u32 {
    1
}

pub fn middle() -> u32 {
    leaf()
}

pub fn top() -> u32 {
    middle()
}

pub fn other() -> u32 {
    middle()
}
";

struct Sandbox(PathBuf);

impl Sandbox {
    fn new(name: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("shage-index-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("src")).expect("create sandbox");
        fs::write(dir.join("src/lib.rs"), TREE).expect("write sandbox");
        Self(dir)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn symbol_named(backend: &HeuristicBackend, name: &str) -> SymId {
    backend
        .calls()
        .iter()
        .flat_map(|call| [&call.from, call.target.one().unwrap_or(&call.from)])
        .find(|symbol| symbol.display == name)
        .unwrap_or_else(|| panic!("the sandbox defines {name}"))
        .sym_id
        .clone()
}

#[test]
fn every_path_is_repository_relative() {
    let sandbox = Sandbox::new("paths");
    let backend = HeuristicBackend::open(sandbox.path()).expect("open");
    assert!(!backend.calls().is_empty(), "the sandbox has call edges");

    for call in backend.calls() {
        for path in [&call.path, &call.from.path] {
            assert!(!path.is_absolute(), "{} is absolute", path.display());
            assert!(
                !path.components().any(|c| c.as_os_str() == ".."),
                "{} escapes the repository root",
                path.display()
            );
            assert!(
                sandbox.path().join(path).exists(),
                "{} does not resolve against the root it came from",
                path.display()
            );
        }
    }
}

#[test]
fn blast_packages_are_sorted_and_deduplicated() {
    let sandbox = Sandbox::new("packages");
    let backend = HeuristicBackend::open(sandbox.path()).expect("open");
    let leaf = symbol_named(&backend, "leaf");

    let blast = backend.blast_radius(&leaf, 3).expect("blast").value;
    let mut expected = blast.packages.clone();
    expected.sort();
    expected.dedup();
    assert_eq!(
        blast.packages, expected,
        "packages are sorted and deduplicated"
    );
}

#[test]
fn the_walk_is_bounded_and_says_so() {
    let sandbox = Sandbox::new("bounded");
    let backend = HeuristicBackend::open(sandbox.path()).expect("open");
    let leaf = symbol_named(&backend, "leaf");

    for depth in [0u8, 1, 3, 7] {
        let blast = backend.blast_radius(&leaf, depth).expect("blast").value;
        assert_eq!(blast.depth, depth, "Blast::depth echoes its argument");
    }

    // `middle` calls `leaf`; `top` and `other` call `middle`. One hop sees one caller, and
    // the second hop is the difference between direct and transitive.
    let one = backend.blast_radius(&leaf, 1).expect("blast").value;
    assert_eq!((one.direct, one.transitive), (1, 0), "one hop is one hop");
    let three = backend.blast_radius(&leaf, 3).expect("blast").value;
    assert_eq!(
        (three.direct, three.transitive),
        (1, 2),
        "three hops reach the rest"
    );
    assert_eq!(
        backend.blast_radius(&leaf, 0).expect("blast").value.direct,
        0,
        "depth zero walks nothing"
    );
}

/// A backend that structurally cannot answer says so, rather than returning an empty list
/// that reads as "no commit ever touched this".
///
/// The ordering and `limit` halves of the history contract — most-recent-first, and `limit`
/// as a hard cap — stay untested until a backend carries commit data, because neither
/// backend that exists can produce a single `CommitRef` to order.
#[test]
fn history_is_unsupported_not_empty() {
    let sandbox = Sandbox::new("history");
    let backend = HeuristicBackend::open(sandbox.path()).expect("open");
    let leaf = symbol_named(&backend, "leaf");

    for limit in [0usize, 1, 50] {
        match backend.history(&leaf, limit) {
            Err(IndexError::Unsupported(why)) => assert!(!why.is_empty(), "the reason is named"),
            other => panic!("expected Unsupported, got {other:?}"),
        }
    }
}

/// Detection never errors and never blocks startup. Whatever it picks answers `Ok`.
#[test]
fn detection_always_returns_a_working_backend() {
    let sandbox = Sandbox::new("detect");
    let backend = backends::detect(sandbox.path());
    assert!(
        backend.stamp().is_ok(),
        "a detected backend can be asked for its stamp"
    );
    assert!(
        backend.symbols_in_diff(&[]).is_ok(),
        "a detected backend answers an empty diff"
    );

    let empty = std::env::temp_dir().join(format!("shage-index-{}-empty", std::process::id()));
    fs::create_dir_all(&empty).expect("create");
    assert!(
        backends::detect(&empty).stamp().is_ok(),
        "a directory with no code still yields a backend rather than a failure"
    );
    let _ = fs::remove_dir_all(&empty);
}
