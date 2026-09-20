//! Per-frame recorder for scrollable panel and clickable list geometry.
//!
//! Pages call the `record_*` helpers while painting so the runtime can hit-test
//! mouse coordinates against the exact rectangles the last frame used. The
//! recorder is thread-local frame scratch state, mirroring the scoped active
//! theme in `style.rs`: `render_app()` clears it before routing the frame and
//! takes the finished snapshot afterwards, so no render context needs to
//! thread a snapshot handle through every page constructor.

use std::cell::RefCell;

use ratatui::layout::Rect;

use crate::presentation::app_mode::ViewportRect;
use crate::presentation::viewport::{
    LayoutSnapshot, ListItemHit, ListRegion, ListRegionKind, ScrollRegion,
};

thread_local! {
    static FRAME_LAYOUT: RefCell<LayoutSnapshot> = RefCell::new(LayoutSnapshot::default());
}

/// Clears recorded regions before a new frame is routed.
pub(crate) fn begin_frame() {
    FRAME_LAYOUT.with(|layout| *layout.borrow_mut() = LayoutSnapshot::default());
}

/// Returns the regions recorded since [`begin_frame`], leaving the recorder
/// empty.
pub(crate) fn take_frame() -> LayoutSnapshot {
    FRAME_LAYOUT.with(|layout| std::mem::take(&mut *layout.borrow_mut()))
}

/// Records the session transcript panel painted by the current frame.
pub(crate) fn record_chat_output(region: ScrollRegion) {
    FRAME_LAYOUT.with(|layout| layout.borrow_mut().chat_output = Some(region));
}

/// Records the diff-mode file explorer column painted by the current frame.
pub(crate) fn record_diff_file_list(area: Rect) {
    FRAME_LAYOUT.with(|layout| layout.borrow_mut().diff_file_list = Some(viewport_rect(area)));
}

/// Records the diff-mode right panel painted by the current frame.
pub(crate) fn record_diff_panel(region: ScrollRegion) {
    FRAME_LAYOUT.with(|layout| layout.borrow_mut().diff_panel = Some(region));
}

/// Records the help popup painted by the current frame.
pub(crate) fn record_help_overlay(region: ScrollRegion) {
    FRAME_LAYOUT.with(|layout| layout.borrow_mut().help_overlay = Some(region));
}

/// Records one selectable list painted by the current frame.
///
/// Lists are kept in paint order so overlays painted later shadow the page
/// lists underneath them during hit-testing.
pub(crate) fn record_list(list: ListRegion) {
    FRAME_LAYOUT.with(|layout| layout.borrow_mut().lists.push(list));
}

/// One rendered list row, as painted by a `ratatui` table or line list.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ListRow {
    /// Rows of trailing spacing the row adds below itself.
    pub(crate) bottom_margin: u16,
    /// Rows the item itself occupies.
    pub(crate) height: u16,
    /// Item the row selects, or `None` for labels and separators.
    pub(crate) index: Option<usize>,
}

impl ListRow {
    /// A one-line selectable row without spacing.
    pub(crate) const fn item(index: usize) -> Self {
        Self {
            bottom_margin: 0,
            height: 1,
            index: Some(index),
        }
    }
}

/// Builds the list region for rows stacked from the top of `body`, in the
/// order a `ratatui` table paints them after its scroll offset.
///
/// Rows that would start below `body` are dropped, so the region covers only
/// what is on screen. Callers pass rows starting at the table's first visible
/// row.
pub(crate) fn stacked_rows_list(
    kind: ListRegionKind,
    body: Rect,
    rows: impl IntoIterator<Item = ListRow>,
) -> ListRegion {
    let bottom = body.y.saturating_add(body.height);
    let mut next_y = body.y;
    let mut items = Vec::new();

    for row in rows {
        if next_y >= bottom {
            break;
        }
        if let Some(index) = row.index {
            let height = row.height.min(bottom.saturating_sub(next_y));
            items.push(ListItemHit {
                area: ViewportRect {
                    height,
                    width: body.width,
                    x: body.x,
                    y: next_y,
                },
                index,
            });
        }
        next_y = next_y
            .saturating_add(row.height)
            .saturating_add(row.bottom_margin);
    }

    ListRegion { items, kind }
}

/// Builds the list region for `count` one-line items painted consecutively
/// from `first_index` at the top of `body`, the shape of every option popup
/// and windowed dropdown.
pub(crate) fn consecutive_rows_list(
    kind: ListRegionKind,
    body: Rect,
    first_index: usize,
    count: usize,
) -> ListRegion {
    stacked_rows_list(
        kind,
        body,
        (first_index..first_index.saturating_add(count)).map(ListRow::item),
    )
}

/// Builds a scroll region from Ratatui geometry and rendered content metrics.
pub(crate) fn scroll_region(
    area: Rect,
    scrollbar: Option<Rect>,
    total_lines: usize,
    viewport_height: u16,
) -> ScrollRegion {
    ScrollRegion {
        area: viewport_rect(area),
        scrollbar: scrollbar.map(viewport_rect),
        total_lines: u16::try_from(total_lines).unwrap_or(u16::MAX),
        viewport_height,
    }
}

/// Converts Ratatui geometry into the frontend-neutral rectangle contract.
pub(crate) fn viewport_rect(area: Rect) -> ViewportRect {
    ViewportRect {
        height: area.height,
        width: area.width,
        x: area.x,
        y: area.y,
    }
}

#[cfg(test)]
#[path = "layout_snapshot_test.rs"]
mod tests;
