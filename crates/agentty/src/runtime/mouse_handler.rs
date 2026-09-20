//! Mouse event dispatch: wheel scrolling, scrollbar dragging, and list
//! clicks.
//!
//! Pointer coordinates are hit-tested against the [`LayoutSnapshot`] recorded
//! by the last rendered frame, so handlers never recompute layout. Wheel
//! notches scroll the panel under the pointer; a left press on a scrollbar
//! thumb starts a drag that follows the pointer until release; a left press
//! on a recorded list item selects or activates it through `click_handler`.
//! Everything else (moves, other buttons, horizontal wheel) is ignored.

use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

use crate::app::App;
use crate::presentation::app_mode::AppMode;
use crate::presentation::viewport::{
    LayoutSnapshot, MOUSE_WHEEL_SCROLL_LINES, ScrollRegion, ScrollRegionKind, ScrollbarDrag,
    ScrollbarGeometry,
};
use crate::runtime::click_handler::{self, MouseOutcome};
use crate::runtime::{PresentationState, mode};

/// Vertical wheel direction of one mouse event.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WheelDirection {
    Down,
    Up,
}

/// Applies one mouse event to the app and returns what the event loop must
/// do next.
pub(crate) fn handle_mouse_event(
    app: &mut App,
    presentation: &PresentationState,
    mouse: MouseEvent,
) -> MouseOutcome {
    let layout = presentation.layout_snapshot();

    match mouse.kind {
        MouseEventKind::ScrollDown => MouseOutcome::from_changed(handle_wheel(
            app,
            presentation,
            &layout,
            mouse.column,
            mouse.row,
            WheelDirection::Down,
        )),
        MouseEventKind::ScrollUp => MouseOutcome::from_changed(handle_wheel(
            app,
            presentation,
            &layout,
            mouse.column,
            mouse.row,
            WheelDirection::Up,
        )),
        MouseEventKind::Down(MouseButton::Left) => {
            handle_press(app, presentation, &layout, mouse.column, mouse.row)
        }
        MouseEventKind::Drag(MouseButton::Left) => {
            MouseOutcome::from_changed(handle_drag(app, presentation, &layout, mouse.row))
        }
        MouseEventKind::Up(MouseButton::Left) => {
            presentation.set_mouse_drag(None);

            MouseOutcome::Ignored
        }
        _ => MouseOutcome::Ignored,
    }
}

/// Scrolls the panel under the pointer by one wheel notch.
fn handle_wheel(
    app: &mut App,
    presentation: &PresentationState,
    layout: &LayoutSnapshot,
    column: u16,
    row: u16,
    direction: WheelDirection,
) -> bool {
    if let Some(scroll_offset) = chat_scroll_offset_mut(&mut app.mode) {
        let Some(region) = layout.chat_output else {
            return false;
        };
        if !region.contains(column, row) {
            return false;
        }
        let next_offset = match direction {
            WheelDirection::Down => {
                region.scroll_tail_down(*scroll_offset, MOUSE_WHEEL_SCROLL_LINES)
            }
            WheelDirection::Up => {
                Some(region.scroll_tail_up(*scroll_offset, MOUSE_WHEEL_SCROLL_LINES))
            }
        };

        return replace_scroll_offset(scroll_offset, next_offset);
    }

    if matches!(app.mode, AppMode::Diff { .. }) {
        return mode::diff::handle_mouse_wheel(
            app,
            presentation.render_cache_store(),
            layout,
            column,
            row,
            direction,
        );
    }

    match &mut app.mode {
        AppMode::Help { scroll_offset, .. } => {
            scroll_clamped(layout.help_overlay, scroll_offset, column, row, direction)
        }
        _ => false,
    }
}

/// Starts a scrollbar drag or jumps the thumb when the press hits a track,
/// or selects the list item under the pointer.
fn handle_press(
    app: &mut App,
    presentation: &PresentationState,
    layout: &LayoutSnapshot,
    column: u16,
    row: u16,
) -> MouseOutcome {
    presentation.set_mouse_drag(None);

    if let Some(changed) = press_scrollbar(app, presentation, layout, column, row) {
        return MouseOutcome::from_changed(changed);
    }

    layout
        .list_item_at(column, row)
        .map_or(MouseOutcome::Ignored, |(kind, index)| {
            click_handler::handle_list_click(app, presentation, kind, index)
        })
}

/// Starts a scrollbar drag or jumps the thumb when the press hits a track.
/// Returns whether a scroll position changed, or `None` when no scrollbar
/// was hit.
fn press_scrollbar(
    app: &mut App,
    presentation: &PresentationState,
    layout: &LayoutSnapshot,
    column: u16,
    row: u16,
) -> Option<bool> {
    if let Some(scroll_offset) = chat_scroll_offset_mut(&mut app.mode)
        && let Some(region) = layout.chat_output
        && region.scrollbar_contains(column, row)
    {
        let current_offset = scroll_offset.unwrap_or_else(|| region.max_scroll_offset());
        let (drag, jump_offset) = press_on_scrollbar(region, current_offset, row);
        presentation.set_mouse_drag(Some(ScrollbarDrag {
            grab_row_within_thumb: drag,
            region: ScrollRegionKind::ChatOutput,
        }));

        return Some(jump_offset.is_some_and(|offset| {
            replace_scroll_offset(scroll_offset, chat_offset_for_region(region, offset))
        }));
    }

    if let AppMode::Diff { scroll_offset, .. } = &mut app.mode
        && let Some(region) = layout.diff_panel
        && region.scrollbar_contains(column, row)
    {
        let (drag, jump_offset) = press_on_scrollbar(region, *scroll_offset, row);
        presentation.set_mouse_drag(Some(ScrollbarDrag {
            grab_row_within_thumb: drag,
            region: ScrollRegionKind::DiffPanel,
        }));

        return Some(jump_offset.is_some_and(|offset| {
            let changed = offset != *scroll_offset;
            *scroll_offset = offset;

            changed
        }));
    }

    None
}

/// Moves the dragged scrollbar thumb to follow the pointer row.
fn handle_drag(
    app: &mut App,
    presentation: &PresentationState,
    layout: &LayoutSnapshot,
    row: u16,
) -> bool {
    let Some(drag) = presentation.mouse_drag() else {
        return false;
    };

    match drag.region {
        ScrollRegionKind::ChatOutput => {
            let Some(region) = layout.chat_output else {
                return false;
            };
            let Some(scroll_offset) = chat_scroll_offset_mut(&mut app.mode) else {
                return false;
            };
            let offset = region.scroll_offset_for_pointer_row(row, drag.grab_row_within_thumb);

            replace_scroll_offset(scroll_offset, chat_offset_for_region(region, offset))
        }
        ScrollRegionKind::DiffPanel => {
            let Some(region) = layout.diff_panel else {
                return false;
            };
            let AppMode::Diff { scroll_offset, .. } = &mut app.mode else {
                return false;
            };
            let next_offset = region.scroll_offset_for_pointer_row(row, drag.grab_row_within_thumb);
            let changed = next_offset != *scroll_offset;
            *scroll_offset = next_offset;

            changed
        }
    }
}

/// Resolves a left press on a scrollbar track into the thumb grab offset and
/// an optional jump target.
///
/// Pressing on the thumb grabs it where it is. Pressing elsewhere on the
/// track centers the thumb under the pointer and scrolls there immediately.
fn press_on_scrollbar(region: ScrollRegion, current_offset: u16, row: u16) -> (u16, Option<u16>) {
    if let Some(grab_row) = region.thumb_grab_row(current_offset, row) {
        return (grab_row, None);
    }

    let track_height = region.scrollbar.map_or(0, |scrollbar| scrollbar.height);
    let geometry = ScrollbarGeometry::new(track_height, region.total_lines, current_offset);
    let grab_row = geometry.thumb_height / 2;

    (
        grab_row,
        Some(region.scroll_offset_for_pointer_row(row, grab_row)),
    )
}

/// Scrolls a plain clamped offset when the pointer is over its region.
fn scroll_clamped(
    region: Option<ScrollRegion>,
    scroll_offset: &mut u16,
    column: u16,
    row: u16,
    direction: WheelDirection,
) -> bool {
    let Some(region) = region else {
        return false;
    };
    if !region.contains(column, row) {
        return false;
    }

    let next_offset = match direction {
        WheelDirection::Down => region.scroll_down(*scroll_offset, MOUSE_WHEEL_SCROLL_LINES),
        WheelDirection::Up => region.scroll_up(*scroll_offset, MOUSE_WHEEL_SCROLL_LINES),
    };
    let changed = next_offset != *scroll_offset;
    *scroll_offset = next_offset;

    changed
}

/// Borrows the transcript scroll offset carried by a chat mode, leaving
/// composer, focus, and question state untouched.
///
/// Returns `None` when the mode does not show the session transcript panel.
/// The borrowed `None` value is the follow-tail position of a chat mode.
fn chat_scroll_offset_mut(mode: &mut AppMode) -> Option<&mut Option<u16>> {
    match mode {
        AppMode::View { scroll_offset, .. }
        | AppMode::Prompt { scroll_offset, .. }
        | AppMode::Question { scroll_offset, .. } => Some(scroll_offset),
        _ => None,
    }
}

/// Stores `next_offset` and returns whether the scroll position changed.
fn replace_scroll_offset(scroll_offset: &mut Option<u16>, next_offset: Option<u16>) -> bool {
    let changed = *scroll_offset != next_offset;
    *scroll_offset = next_offset;

    changed
}

/// Converts an absolute chat offset into the follow-tail representation:
/// reaching the bottom returns `None` so live output keeps streaming.
fn chat_offset_for_region(region: ScrollRegion, offset: u16) -> Option<u16> {
    if offset >= region.max_scroll_offset() {
        return None;
    }

    Some(offset)
}

#[cfg(test)]
#[path = "mouse_handler_test.rs"]
mod tests;
