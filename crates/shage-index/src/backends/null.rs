//! The backend that has no index.

use crate::contract::{
    Blast, CallSite, ChangedFile, CommitRef, DiffSymbols, IndexBackend, IndexStamp, Resolution,
    Result, Stamped, SymId,
};

/// The default backend: empty everywhere, stamped "no index", so an install with no indexer
/// behaves exactly like upstream tuicr.
///
/// Every answer is `Ok`. A missing indexer is silence, not a failure dialog, and the empty
/// results are never mistaken for confident zeros because
/// [`IndexStamp::indexed_commit`] is `None` on all of them.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NullBackend;

impl IndexBackend for NullBackend {
    fn stamp(&self) -> Result<IndexStamp> {
        Ok(IndexStamp::none())
    }

    /// Reports no symbols and no uncovered files.
    ///
    /// `uncovered` stays empty on purpose: `Missing` means "this index does not hold that
    /// file", and with no index at all the stamp already says so. Listing every input as
    /// missing would imply an index with gaps.
    fn symbols_in_diff(&self, _files: &[ChangedFile]) -> Result<Stamped<DiffSymbols>> {
        Ok(Stamped::new(
            IndexStamp::none(),
            DiffSymbols {
                symbols: Vec::new(),
                uncovered: Vec::new(),
            },
        ))
    }

    fn definition(&self, _sym: &SymId) -> Result<Stamped<Resolution>> {
        Ok(Stamped::new(IndexStamp::none(), Resolution::Unresolved))
    }

    fn callers(&self, _sym: &SymId) -> Result<Stamped<Vec<CallSite>>> {
        Ok(Stamped::new(IndexStamp::none(), Vec::new()))
    }

    fn callees(&self, _sym: &SymId) -> Result<Stamped<Vec<CallSite>>> {
        Ok(Stamped::new(IndexStamp::none(), Vec::new()))
    }

    fn history(&self, _sym: &SymId, _limit: usize) -> Result<Stamped<Vec<CommitRef>>> {
        Ok(Stamped::new(IndexStamp::none(), Vec::new()))
    }

    /// Echoes `depth` like every other backend, so the bounded-walk invariant is testable
    /// without an index.
    fn blast_radius(&self, _sym: &SymId, depth: u8) -> Result<Stamped<Blast>> {
        Ok(Stamped::new(
            IndexStamp::none(),
            Blast {
                direct: 0,
                transitive: 0,
                packages: Vec::new(),
                exported: false,
                depth,
            },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sym() -> SymId {
        SymId::new("rust-analyzer cargo shage_index 0.1.0 contract/SymId#")
    }

    #[test]
    fn null_answers_ok_everywhere() {
        let b = NullBackend;
        let s = sym();

        assert!(b.stamp().unwrap().indexed_commit.is_none());
        assert!(
            b.symbols_in_diff(&[])
                .unwrap()
                .stamp
                .indexed_commit
                .is_none()
        );
        assert!(b.definition(&s).unwrap().stamp.indexed_commit.is_none());
        assert!(b.callers(&s).unwrap().stamp.indexed_commit.is_none());
        assert!(b.callees(&s).unwrap().stamp.indexed_commit.is_none());
        assert!(b.history(&s, 5).unwrap().stamp.indexed_commit.is_none());
        assert!(
            b.blast_radius(&s, 2)
                .unwrap()
                .stamp
                .indexed_commit
                .is_none()
        );
    }

    #[test]
    fn null_answers_empty() {
        let b = NullBackend;
        let s = sym();

        assert_eq!(b.definition(&s).unwrap().value, Resolution::Unresolved);
        assert!(b.callers(&s).unwrap().value.is_empty());
        assert!(b.callees(&s).unwrap().value.is_empty());
        assert!(b.history(&s, 5).unwrap().value.is_empty());
        assert!(b.symbols_in_diff(&[]).unwrap().value.symbols.is_empty());

        let blast = b.blast_radius(&s, 2).unwrap().value;
        assert_eq!(blast.direct, 0);
        assert_eq!(blast.transitive, 0);
        assert!(blast.packages.is_empty());
        assert!(!blast.exported);
    }

    #[test]
    fn null_blast_echoes_depth() {
        let b = NullBackend;
        let s = sym();

        assert_eq!(b.blast_radius(&s, 0).unwrap().value.depth, 0);
        assert_eq!(b.blast_radius(&s, 7).unwrap().value.depth, 7);
    }

    #[test]
    fn null_reports_no_uncovered_files() {
        let files: Vec<ChangedFile> = ["a.rs", "b.rs", "c.rs"]
            .iter()
            .map(|p| ChangedFile {
                path: PathBuf::from(p),
                new_lines: vec![1..=10],
            })
            .collect();

        assert!(
            NullBackend
                .symbols_in_diff(&files)
                .unwrap()
                .value
                .uncovered
                .is_empty(),
            "no index means the stamp says so; listing files as Missing would imply gaps"
        );
    }
}
