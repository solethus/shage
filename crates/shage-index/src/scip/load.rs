//! Bytes to an index. Nothing else.

use protobuf::Message;
use scip::types::Index;

use crate::contract::{IndexError, Result};

/// Decodes a SCIP index from bytes.
///
/// Pure: no network, no subprocess, no mutation, no file system. The caller supplies the
/// bytes, which keeps this testable against a truncated or hostile index without one
/// existing on disk.
///
/// An index file is untrusted input — it arrives from whatever indexer the user installed,
/// at whatever version. Malformed bytes come back as [`IndexError::Corrupt`], never as a
/// panic, because a bad index must degrade the panel rather than take the TUI down.
pub fn load(bytes: &[u8]) -> Result<Index> {
    Index::parse_from_bytes(bytes).map_err(|err| IndexError::Corrupt(format!("scip: {err}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garbage_is_corrupt_not_a_panic() {
        let err = load(&[0xff, 0xff, 0xff, 0xff]).expect_err("garbage must not decode");
        assert!(matches!(err, IndexError::Corrupt(_)), "got {err:?}");
    }

    #[test]
    fn an_empty_index_is_valid() {
        // Zero bytes is a well-formed protobuf message with every field defaulted. It is an
        // index of nothing, which is a fact, not a failure.
        let index = load(&[]).expect("empty bytes decode to an empty index");
        assert!(index.documents.is_empty());
    }
}
