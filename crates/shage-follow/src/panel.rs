//! Rendering the follow panel.
//!
//! **Sized from the content outward.** The frame is not the canvas: a bordered block takes
//! a column from each side and a row from the top and bottom, so a 120x24 frame leaves
//! 118x22 to write in, and 80x24 leaves 78x22. Laying rows out against the frame is how a
//! panel that looks right on a wide terminal writes over its own border on a narrow one —
//! upstream records the same trap for its modals. Every width below is computed from
//! `Block::inner`, never from the area passed in.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Widget};

use shage_index::contract::CallSite;

use crate::contract::{FollowPanel, badge, target_of};

/// Width of the badge column. Fixed, so the eye reads down it rather than hunting for it.
const BADGE: usize = 5;
/// Longest evidence the panel will show before shortening it.
const EVIDENCE: usize = 18;
/// Below this, a row is badge and location only — anything else would be ellipses.
const MIN_FOR_DETAIL: usize = 34;

impl Widget for &FollowPanel {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(format!(" follow: {} ", self.following));
        let inner = block.inner(area);
        block.render(area, buf);
        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let width = inner.width as usize;
        let mut lines = vec![Line::from(Span::styled(
            truncate(&self.freshness(), width),
            Style::default().add_modifier(Modifier::DIM),
        ))];

        if self.rows.is_empty() {
            lines.push(Line::from(truncate("no call sites", width)));
        }
        for call in &self.rows {
            lines.push(row(call, width));
        }

        // One row per line, clipped to the inner height. Scrolling is the panel's next
        // branch; clipping is what an unscrollable list must do rather than overflow.
        for (offset, line) in lines.into_iter().take(inner.height as usize).enumerate() {
            let at = Rect {
                y: inner.y + offset as u16,
                height: 1,
                ..inner
            };
            line.render(at, buf);
        }
    }
}

/// One call site: badge, caller, evidence, and where it is written.
///
/// The location is laid out first and never truncated — it is how a reviewer navigates, and
/// half a path is worse than none. Evidence is reserved next, because a `cand` badge whose
/// call expression was truncated away is a badge outliving its evidence, which is the
/// dishonest row this project exists to avoid. The caller's name gets what is left.
fn row(call: &CallSite, width: usize) -> Line<'static> {
    let location = format!("{}:{}", call.path.display(), call.line);
    let mark = badge(call.target.confidence());
    if width < MIN_FOR_DETAIL {
        return Line::from(format!(
            "{mark:<BADGE$} {}",
            truncate(&location, width.saturating_sub(BADGE + 1))
        ));
    }

    let evidence = truncate(&call.text, EVIDENCE);
    let fixed = BADGE + 1 + evidence.chars().count() + 2 + location.chars().count() + 2;
    let target = truncate(
        &format!("-> {}", target_of(call)),
        width.saturating_sub(fixed),
    );
    let mut line = format!("{mark:<BADGE$} {evidence}  {target}");
    let pad = width.saturating_sub(line.chars().count() + location.chars().count());
    line.push_str(&" ".repeat(pad.max(1)));
    line.push_str(&location);
    Line::from(truncate(&line, width))
}

/// Truncates to `width` display cells, marking the cut with `…` so a shortened value is
/// never mistaken for a complete one.
fn truncate(text: &str, width: usize) -> String {
    let count = text.chars().count();
    if count <= width {
        return text.to_owned();
    }
    if width == 0 {
        return String::new();
    }
    if width == 1 {
        return "…".to_owned();
    }
    text.chars().take(width - 1).chain(['…']).collect()
}
