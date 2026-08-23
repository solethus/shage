//! What the panel actually draws, at a width that fits and a width that does not.
//!
//! Asserting on rendered text rather than on the state behind it is the only way to catch
//! the bug this panel is prone to: a row that reads fine at 120 columns and writes over its
//! own border at 80. Both sizes are the test — either one alone proves nothing, because the
//! interesting behaviour is the difference between them.

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use shage_follow::contract::FollowPanel;
use shage_index::contract::{CallSite, IndexStamp, Resolution, SymId, SymbolRef};
use std::path::PathBuf;

fn symbol(display: &str, path: &str, line: u32) -> SymbolRef {
    SymbolRef {
        sym_id: SymId::new(format!("{path}:{line}:{display}")),
        display: display.to_owned(),
        path: PathBuf::from(path),
        line,
    }
}

fn call(from: &str, path: &str, line: u32, text: &str, target: Resolution) -> CallSite {
    CallSite {
        from: symbol(from, path, line.saturating_sub(2).max(1)),
        path: PathBuf::from(path),
        line,
        text: text.to_owned(),
        target,
    }
}

/// One row per badge, plus one row whose target is long enough that 78 columns cannot hold
/// it and 118 can. That last row is the whole point of running two sizes.
fn panel() -> FollowPanel {
    FollowPanel {
        following: "Bucket::allow".to_owned(),
        stamp: IndexStamp {
            indexed_commit: Some("a1b2c3d4e5f6".to_owned()),
            repo_commit: Some("9f8e7d6c5b4a".to_owned()),
            overlay: false,
        },
        rows: vec![
            call(
                "main",
                "src/main.rs",
                18,
                "call_alpha",
                Resolution::Exact(symbol("call_alpha", "src/main.rs", 7)),
            ),
            call(
                "caller",
                "src/lib.rs",
                12,
                "alias",
                Resolution::Heuristic(symbol("real", "src/inner.rs", 1)),
            ),
            call(
                "call_alpha",
                "src/rate_limit/bucket_registry.rs",
                914,
                "limiter.allow",
                Resolution::candidates(vec![
                    symbol("run", "src/alpha.rs", 4),
                    symbol("run", "src/beta.rs", 4),
                ]),
            ),
            call(
                "handle",
                "src/rate_limit/bucket_registry.rs",
                914,
                "limiter.allow",
                Resolution::Exact(symbol("allow", "src/rate_limit/token_bucket.rs", 88)),
            ),
            call(
                "caller",
                "src/lib.rs",
                12,
                "generated",
                Resolution::Unresolved,
            ),
        ],
    }
}
fn draw(panel: &FollowPanel, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).expect("terminal");
    terminal
        .draw(|frame| frame.render_widget(panel, frame.area()))
        .expect("draw");
    terminal.backend().buffer().clone()
}
/// The buffer as the reviewer sees it, one string per row.
fn rows(buffer: &Buffer) -> Vec<String> {
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
        })
        .collect()
}
#[test]
fn renders_at_120x24() {
    let drawn = rows(&draw(&panel(), 120, 24));
    assert_eq!(drawn.len(), 24, "the frame is 24 rows");

    let written: Vec<&str> = drawn[..7].iter().map(String::as_str).collect();
    assert_eq!(
        written,
        [
            "┌ follow: Bucket::allow ───────────────────────────────────────────────────────────────────────────────────────────────┐",
            "│index at a1b2c3d, tree at 9f8e7d6                                                                                     │",
            "│exact call_alpha  -> call_alpha (src/main.rs:7)                                                         src/main.rs:18│",
            "│heur  alias  -> real (src/inner.rs:1)                                                                    src/lib.rs:12│",
            "│cand  limiter.allow  -> 2 candidates                                             src/rate_limit/bucket_registry.rs:914│",
            "│exact limiter.allow  -> allow (src/rate_limit/token_bucket.rs:88)                src/rate_limit/bucket_registry.rs:914│",
            "│?     generated  -> unresolved                                                                           src/lib.rs:12│",
        ]
    );
    assert!(
        drawn[23].starts_with('└') && drawn[23].ends_with('┘'),
        "the bottom border is drawn at the bottom of the frame, not under the last row"
    );
}

/// 80 columns leaves 78 to write in. Every row below is laid out against that, not against
/// the frame, which is what keeps the borders intact.
#[test]
fn renders_at_80x24() {
    let drawn = rows(&draw(&panel(), 80, 24));

    let written: Vec<&str> = drawn[..7].iter().map(String::as_str).collect();
    assert_eq!(
        written,
        [
            "┌ follow: Bucket::allow ───────────────────────────────────────────────────────┐",
            "│index at a1b2c3d, tree at 9f8e7d6                                             │",
            "│exact call_alpha  -> call_alpha (src/main.rs:7)                 src/main.rs:18│",
            "│heur  alias  -> real (src/inner.rs:1)                            src/lib.rs:12│",
            "│cand  limiter.allow  -> 2 candidates     src/rate_limit/bucket_registry.rs:914│",
            "│exact limiter.allow  -> allow (src/rat…  src/rate_limit/bucket_registry.rs:914│",
            "│?     generated  -> unresolved                                   src/lib.rs:12│",
        ]
    );
}

/// The structural promise, at both sizes: nothing ever leaves the box.
#[test]
fn no_row_escapes_its_border() {
    for width in [120u16, 80] {
        for (row, line) in rows(&draw(&panel(), width, 24)).iter().enumerate() {
            assert_eq!(
                line.chars().count(),
                width as usize,
                "{width}x24 row {row} is not {width} cells wide"
            );
            assert!(
                line.starts_with(['│', '┌', '└']) && line.ends_with(['│', '┐', '┘']),
                "{width}x24 row {row} wrote over a border: {line}"
            );
        }
    }
}

/// The row that fits at 120 and does not at 80 is shortened, not silently cut, and keeps
/// the evidence behind its badge either way.
#[test]
fn a_row_too_wide_is_marked_shortened_and_keeps_its_evidence() {
    let wide = rows(&draw(&panel(), 120, 24));
    let narrow = rows(&draw(&panel(), 80, 24));

    assert!(
        wide[5].contains("src/rate_limit/token_bucket.rs:88"),
        "at 120 the target fits whole: {}",
        wide[5]
    );
    assert!(
        !narrow[5].contains("src/rate_limit/token_bucket.rs:88") && narrow[5].contains('…'),
        "at 80 the target is shortened and says so: {}",
        narrow[5]
    );
    for line in [&wide[4], &narrow[4], &wide[5], &narrow[5]] {
        assert!(
            line.contains("limiter.allow"),
            "a badge must not outlive the call expression behind it: {line}"
        );
    }
    assert!(
        narrow[4].contains("src/rate_limit/bucket_registry.rs:914"),
        "the location is how a reviewer navigates and is never the part that is dropped: {}",
        narrow[4]
    );
}

/// The heuristic backend has no indexed commit by design, and that is not "no index".
///
/// It re-reads the working tree every open, so there is nothing to be stale against — but
/// the tree it read is a fact worth printing, and printing "no index" instead makes a real
/// heuristic answer indistinguishable from a backend with nothing behind it.
#[test]
fn a_working_tree_backend_reports_its_tree_not_no_index() {
    let mut panel = panel();
    panel.stamp = IndexStamp {
        indexed_commit: None,
        repo_commit: Some("9f8e7d6c5b4a".to_owned()),
        overlay: false,
    };
    assert_eq!(panel.freshness(), "no index, tree at 9f8e7d6");

    panel.stamp = IndexStamp::none();
    assert_eq!(
        panel.freshness(),
        "no index",
        "nothing read and nothing to read stays the bare sentence"
    );
}
