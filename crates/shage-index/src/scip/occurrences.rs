//! Decoding an occurrence: where it is, and what role it plays.

use scip::types::SymbolRole;

/// The lines an occurrence covers, 1-based and inclusive.
///
/// SCIP counts lines from zero and this crate counts from one, so the conversion happens
/// here, once, rather than at every call site where it would eventually be forgotten.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// First line, 1-based inclusive.
    pub first_line: u32,
    /// Last line, 1-based inclusive. Equal to `first_line` for a single-line occurrence.
    pub last_line: u32,
}

/// Decodes a SCIP range, or `None` if it is neither of the two legal shapes.
///
/// **Three ints is a single-line range and four is a multi-line one.** Assuming four is the
/// classic way to read this format wrong: it silently shifts every single-line occurrence's
/// end into the next line's columns, and every badge downstream is then attributed to a
/// neighbour. An unknown length is `None` rather than a guess.
pub fn decode_range(range: &[i32]) -> Option<Span> {
    let (first, last) = match range {
        // [line, start_character, end_character]
        [line, _, _] => (*line, *line),
        // [start_line, start_character, end_line, end_character]
        [start_line, _, end_line, _] => (*start_line, *end_line),
        _ => return None,
    };
    if first < 0 || last < first {
        return None;
    }
    Some(Span {
        first_line: first as u32 + 1,
        last_line: last as u32 + 1,
    })
}

/// Whether the occurrence defines its symbol rather than referring to it.
///
/// Roles are a **bit set**, not an enum value: an occurrence can be a definition and
/// generated and a test all at once, so this masks rather than compares.
pub fn is_definition(roles: i32) -> bool {
    roles & (SymbolRole::Definition as i32) != 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_ints_is_one_line() {
        let span = decode_range(&[7, 4, 9]).expect("a three-int range is legal");
        assert_eq!(
            span.first_line, 8,
            "SCIP lines are 0-based, ours are 1-based"
        );
        assert_eq!(span.last_line, 8, "three ints never spans two lines");
    }

    #[test]
    fn four_ints_spans_lines() {
        let span = decode_range(&[6, 0, 9, 1]).expect("a four-int range is legal");
        assert_eq!((span.first_line, span.last_line), (7, 10));
    }

    #[test]
    fn other_lengths_are_none() {
        for range in [&[][..], &[1][..], &[1, 2][..], &[1, 2, 3, 4, 5][..]] {
            assert_eq!(decode_range(range), None, "{range:?} is not a SCIP range");
        }
    }

    #[test]
    fn negative_and_inverted_ranges_are_none() {
        assert_eq!(decode_range(&[-1, 0, 4]), None);
        assert_eq!(decode_range(&[9, 0, 2, 1]), None, "end before start");
    }

    #[test]
    fn roles_are_a_set() {
        let definition = SymbolRole::Definition as i32;
        assert!(is_definition(definition));
        assert!(
            is_definition(definition | SymbolRole::Generated as i32 | SymbolRole::Test as i32),
            "a generated test definition is still a definition"
        );
        assert!(!is_definition(SymbolRole::ReadAccess as i32));
        assert!(!is_definition(0));
    }
}
