use ratatui::layout::Rect;

use super::{
    ListRow, begin_frame, consecutive_rows_list, record_chat_output, record_diff_file_list,
    record_help_overlay, record_list, scroll_region, stacked_rows_list, take_frame,
};
use crate::presentation::app_mode::ViewportRect;
use crate::presentation::viewport::{LayoutSnapshot, ListItemHit, ListRegionKind};

fn hit(index: usize, x: u16, y: u16, width: u16, height: u16) -> ListItemHit {
    ListItemHit {
        area: ViewportRect {
            height,
            width,
            x,
            y,
        },
        index,
    }
}

#[test]
fn test_take_frame_returns_recorded_regions_and_resets() {
    // Arrange
    begin_frame();
    let region = scroll_region(Rect::new(1, 2, 30, 12), None, 40, 10);

    // Act
    record_chat_output(region);
    record_diff_file_list(Rect::new(0, 0, 10, 5));
    let snapshot = take_frame();
    let emptied = take_frame();

    // Assert
    assert_eq!(snapshot.chat_output, Some(region));
    assert_eq!(
        snapshot.diff_file_list,
        Some(ViewportRect {
            height: 5,
            width: 10,
            x: 0,
            y: 0
        })
    );
    assert_eq!(emptied, LayoutSnapshot::default());
}

#[test]
fn test_begin_frame_clears_previous_regions() {
    // Arrange
    record_help_overlay(scroll_region(Rect::new(0, 0, 5, 5), None, 3, 3));

    // Act
    begin_frame();
    let snapshot = take_frame();

    // Assert
    assert_eq!(snapshot.help_overlay, None);
}

#[test]
fn test_record_list_keeps_paint_order() {
    // Arrange
    begin_frame();
    let page_list = consecutive_rows_list(ListRegionKind::Sessions, Rect::new(0, 0, 10, 3), 0, 3);
    let overlay_list = consecutive_rows_list(
        ListRegionKind::SessionCreation,
        Rect::new(0, 0, 10, 2),
        0,
        2,
    );

    // Act
    record_list(page_list.clone());
    record_list(overlay_list.clone());
    let snapshot = take_frame();

    // Assert
    assert_eq!(snapshot.lists, vec![page_list, overlay_list]);
}

#[test]
fn test_stacked_rows_list_skips_labels_and_margins_and_clips_to_body() {
    // Arrange
    let body = Rect::new(2, 5, 20, 4);
    let rows = [
        ListRow {
            bottom_margin: 0,
            height: 1,
            index: None,
        },
        ListRow {
            bottom_margin: 1,
            height: 1,
            index: Some(0),
        },
        ListRow::item(1),
        ListRow::item(2),
    ];

    // Act
    let region = stacked_rows_list(ListRegionKind::Sessions, body, rows);

    // Assert
    assert_eq!(region.kind, ListRegionKind::Sessions);
    assert_eq!(
        region.items,
        vec![hit(0, 2, 6, 20, 1), hit(1, 2, 8, 20, 1)],
        "the label row, the margin row, and the row below the body are not items"
    );
}

#[test]
fn test_stacked_rows_list_clips_a_tall_row_at_the_body_bottom() {
    // Arrange
    let body = Rect::new(0, 0, 8, 3);
    let rows = [
        ListRow::item(4),
        ListRow {
            bottom_margin: 0,
            height: 5,
            index: Some(5),
        },
    ];

    // Act
    let region = stacked_rows_list(ListRegionKind::Projects, body, rows);

    // Assert
    assert_eq!(region.items, vec![hit(4, 0, 0, 8, 1), hit(5, 0, 1, 8, 2)]);
}

#[test]
fn test_consecutive_rows_list_starts_at_the_window_offset() {
    // Arrange
    let body = Rect::new(3, 3, 10, 10);

    // Act
    let region = consecutive_rows_list(ListRegionKind::SettingsSelector, body, 4, 2);

    // Assert
    assert_eq!(region.items, vec![hit(4, 3, 3, 10, 1), hit(5, 3, 4, 10, 1)]);
}

#[test]
fn test_consecutive_rows_list_with_no_items_is_empty() {
    // Act
    let region = consecutive_rows_list(ListRegionKind::DiffFiles, Rect::new(0, 0, 5, 5), 0, 0);

    // Assert
    assert_eq!(region.items, Vec::new());
}
