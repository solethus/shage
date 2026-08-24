//! Decoding an occurrence: where it is, and what role it plays.

use scip::types::{MultiLineRange, Occurrence, SingleLineRange, SymbolRole, occurrence};

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

/// Where the occurrence's name is written, from whichever field the producer populated.
///
/// SCIP deprecated the flat `range` in favour of the `typed_range` oneof, and its own schema
/// says the typed form takes precedence. rust-analyzer still writes the flat one; scip-go
/// and newer scip-typescript write the typed one. Reading only the deprecated field means a
/// perfectly good index from another producer decodes to nothing at all — and because
/// `ScipBackend::open` still succeeds, that arrives as a confident zero rather than an error.
pub fn name_span(occurrence: &Occurrence) -> Option<Span> {
    // The oneof is `#[non_exhaustive]`: a variant added by a later SCIP falls through to the
    // deprecated flat field rather than being read as "no range", because a producer that
    // writes both is common and a silent `None` here drops the occurrence entirely.
    match &occurrence.typed_range {
        Some(occurrence::Typed_range::SingleLineRange(range)) => single_line(range),
        Some(occurrence::Typed_range::MultiLineRange(range)) => multi_line(range),
        _ => decode_range(&occurrence.range),
    }
}

/// The lines the whole definition spans — signature and body — or `None` when the producer
/// left it out.
///
/// `None` is not the same as "one line". Collapsing an absent enclosing range onto the name
/// line leaves every call in the body attributed to nothing, so callers record the gap
/// instead of inventing a span.
pub fn body_span(occurrence: &Occurrence) -> Option<Span> {
    match &occurrence.typed_enclosing_range {
        Some(occurrence::Typed_enclosing_range::SingleLineEnclosingRange(range)) => {
            single_line(range)
        }
        Some(occurrence::Typed_enclosing_range::MultiLineEnclosingRange(range)) => {
            multi_line(range)
        }
        _ => decode_range(&occurrence.enclosing_range),
    }
}

fn single_line(range: &SingleLineRange) -> Option<Span> {
    (range.line >= 0).then(|| Span {
        first_line: range.line as u32 + 1,
        last_line: range.line as u32 + 1,
    })
}

fn multi_line(range: &MultiLineRange) -> Option<Span> {
    (range.start_line >= 0 && range.end_line >= range.start_line).then(|| Span {
        first_line: range.start_line as u32 + 1,
        last_line: range.end_line as u32 + 1,
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
    fn typed_range_wins_over_the_deprecated_one() {
        let mut occurrence = Occurrence::new();
        occurrence.range = vec![0, 0, 1];
        occurrence.typed_range = Some(occurrence::Typed_range::SingleLineRange(SingleLineRange {
            line: 41,
            start_character: 4,
            end_character: 9,
            ..Default::default()
        }));
        let span = name_span(&occurrence).expect("a typed range is a range");
        assert_eq!((span.first_line, span.last_line), (42, 42));
    }

    #[test]
    fn a_typed_only_index_still_decodes() {
        // The failure this guards: reading only the deprecated field makes an index from
        // scip-go or newer scip-typescript decode to nothing while still opening cleanly.
        let mut occurrence = Occurrence::new();
        occurrence.typed_enclosing_range = Some(
            occurrence::Typed_enclosing_range::MultiLineEnclosingRange(MultiLineRange {
                start_line: 6,
                start_character: 0,
                end_line: 9,
                end_character: 1,
                ..Default::default()
            }),
        );
        let span = body_span(&occurrence).expect("a typed enclosing range is a range");
        assert_eq!((span.first_line, span.last_line), (7, 10));
        assert_eq!(name_span(&occurrence), None, "no range of any kind is None");
    }

    #[test]
    fn an_absent_enclosing_range_is_none_not_the_name_line() {
        let mut occurrence = Occurrence::new();
        occurrence.range = vec![7, 4, 9];
        assert!(name_span(&occurrence).is_some());
        assert_eq!(
            body_span(&occurrence),
            None,
            "an absent body must not collapse onto the name line"
        );
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
