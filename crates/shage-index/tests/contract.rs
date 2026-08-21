//! The invariants that must hold from outside the crate, exercised the way a consumer
//! exercises them. `crates/shage-index/AGENTS.md` states them; this file falsifies them.

use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::SystemTime;

use shage_index::backends::NullBackend;
use shage_index::contract::{
    Blast, CallSite, ChangedFile, CommitRef, Confidence, DiffSymbols, FileCoverage, FileState,
    IndexBackend, IndexError, IndexStamp, Resolution, Stamped, SymId, SymbolRef,
};

fn symbol(display: &str, line: u32) -> SymbolRef {
    SymbolRef {
        sym_id: SymId::new(format!("scip . . . `{display}`#")),
        display: display.to_string(),
        path: PathBuf::from("src/lib.rs"),
        line,
    }
}

fn stamp_at(indexed: &str, repo: &str) -> IndexStamp {
    IndexStamp {
        indexed_commit: Some(indexed.to_string()),
        repo_commit: Some(repo.to_string()),
        overlay: false,
    }
}

/// Invariant 2, the half a runtime test can reach: an empty value still arrives with its
/// stamp. The other half — that no `Default` or `From<T>` exists to build one without a
/// stamp — is a compile-time property, enforced by the `compile_fail` doctest on `Stamped`.
#[test]
fn stamp_travels_with_an_empty_value() {
    let empty: Stamped<Vec<SymbolRef>> = Stamped::new(IndexStamp::none(), Vec::new());

    assert!(empty.value.is_empty());
    assert!(
        empty.stamp.indexed_commit.is_none(),
        "an empty list still says which index produced it"
    );
}

/// Invariant 3, mechanically: no two freshness states share a representation, so a panel
/// matching on them cannot collapse two into one. It does not prove the panel renders five
/// different things — that is `follow/freshness`'s snapshot tests.
#[test]
fn stamp_states_compare_unequal() {
    let no_index = IndexStamp::none();
    let current = stamp_at("a1b2c3d", "a1b2c3d");
    let differs = stamp_at("a1b2c3d", "9f8e7d6");

    assert!(no_index.indexed_commit.is_none());
    assert!(current.indexed_commit.is_some());
    assert!(differs.indexed_commit.is_some());

    assert_eq!(current.indexed_commit, current.repo_commit);
    assert_ne!(differs.indexed_commit, differs.repo_commit);
    assert_ne!(no_index, current);
    assert_ne!(current, differs);

    // The two file-level states ride on the answer, not on the stamp.
    let missing = FileState {
        path: PathBuf::from("src/new.rs"),
        coverage: FileCoverage::Missing,
    };
    let dirty = FileState {
        path: PathBuf::from("src/new.rs"),
        coverage: FileCoverage::Dirty,
    };
    assert_ne!(missing.coverage, dirty.coverage);

    let answer = DiffSymbols {
        symbols: vec![symbol("Bucket::allow", 12)],
        uncovered: vec![missing],
    };
    assert!(
        !answer.symbols.is_empty() && !answer.uncovered.is_empty(),
        "found symbols and unreadable files are reported together, not one instead of the other"
    );
}

/// Invariant 4, half one: an unresolved answer holds no symbol to misread.
#[test]
fn unresolved_holds_no_symbol() {
    assert!(Resolution::Unresolved.one().is_none());
    assert_eq!(Resolution::Unresolved.confidence(), Confidence::Unresolved);
}

/// Invariant 4, half three: a candidate badge always has candidates behind it. An empty set
/// is no answer, not a weak one, so the constructor gives back `Unresolved`.
#[test]
fn candidates_are_never_empty() {
    assert_eq!(Resolution::candidates(Vec::new()), Resolution::Unresolved);
    assert_eq!(
        Resolution::candidates(Vec::new()).confidence(),
        Confidence::Unresolved
    );

    let one = symbol("Bucket::allow", 12);
    assert_eq!(
        Resolution::candidates(vec![one.clone()]),
        Resolution::Candidates(vec![one]),
        "a single name match stays a candidate — it is a weaker claim than Heuristic"
    );
}

/// Invariant 4, half two: every variant maps to its own badge, and only `Unresolved` maps
/// to `Confidence::Unresolved`.
#[test]
fn resolution_confidence_round_trips() {
    let one = symbol("Bucket::allow", 12);
    let two = symbol("Redis::allow", 40);

    let exact = Resolution::Exact(one.clone());
    let heuristic = Resolution::Heuristic(one.clone());
    let candidates = Resolution::Candidates(vec![one.clone(), two]);

    assert_eq!(exact.confidence(), Confidence::Exact);
    assert_eq!(heuristic.confidence(), Confidence::Heuristic);
    assert_eq!(candidates.confidence(), Confidence::Candidate);

    assert_eq!(exact.one(), Some(&one));
    assert_eq!(heuristic.one(), Some(&one));
    assert!(
        candidates.one().is_none(),
        "picking one of several candidates is the panel's decision, not this crate's"
    );

    // Declaration order is the trust order, so ranking can sort rather than match.
    assert!(Confidence::Exact < Confidence::Heuristic);
    assert!(Confidence::Heuristic < Confidence::Candidate);
    assert!(Confidence::Candidate < Confidence::Unresolved);
}

/// Invariant 6: every contract type is owned and `Send + 'static`, and the trait is
/// object-safe. A borrow sneaking into any of them breaks this test rather than the TUI.
#[test]
fn contract_types_are_send() {
    fn assert_send<T: Send + 'static>() {}

    assert_send::<SymId>();
    assert_send::<SymbolRef>();
    assert_send::<Confidence>();
    assert_send::<Resolution>();
    assert_send::<CallSite>();
    assert_send::<CommitRef>();
    assert_send::<ChangedFile>();
    assert_send::<FileCoverage>();
    assert_send::<FileState>();
    assert_send::<DiffSymbols>();
    assert_send::<IndexStamp>();
    assert_send::<Blast>();
    assert_send::<IndexError>();
    assert_send::<Stamped<Vec<CallSite>>>();
    assert_send::<Box<dyn IndexBackend>>();

    fn assert_object_safe(_: Box<dyn IndexBackend>) {}
    assert_object_safe(Box::new(NullBackend));

    // CommitRef carries a std::time value, so it crosses a thread boundary unchanged.
    assert_send::<SystemTime>();
}

/// Invariant 6, for real: the seam is a worker thread sending an answer back to the tick
/// loop over an `mpsc` channel. If a contract type ever stops being owned, this stops
/// compiling.
#[test]
fn an_answer_crosses_a_channel() {
    let (tx, rx) = mpsc::channel();
    let sym = SymId::new("scip . . . `Bucket#allow().`");

    thread::spawn(move || {
        let backend = NullBackend;
        let _ = tx.send(backend.callers(&sym).unwrap());
    });

    let answer: Stamped<Vec<CallSite>> = rx.recv().expect("worker sent an answer");
    assert!(answer.value.is_empty());
    assert!(
        answer.stamp.indexed_commit.is_none(),
        "an empty caller list arrives with the reason it is empty"
    );
}
