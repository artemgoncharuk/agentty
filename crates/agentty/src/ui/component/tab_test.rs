use std::path::PathBuf;

use ratatui::layout::Rect;
use ratatui::style::Modifier;

use super::{tab_list_region, tab_segments};
use crate::app::Tab;
use crate::domain::project::{Project, ProjectListItem};
use crate::presentation::app_mode::ViewportRect;
use crate::presentation::viewport::{ListItemHit, ListRegionKind};
use crate::ui::style;

#[test]
fn test_tab_segments_use_equal_spacing_between_labels() {
    // Arrange
    let current_tab = Tab::Projects;

    // Act
    let spans = tab_segments(current_tab, 0, &[])
        .into_iter()
        .map(|(_, span)| span)
        .collect::<Vec<_>>();
    let rendered_tabs: String = spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<Vec<_>>()
        .join("");

    // Assert
    assert_eq!(
        rendered_tabs,
        " Projects | Project: None | Sessions | Settings "
    );
}

#[test]
fn test_tab_segments_highlight_the_active_tab() {
    // Arrange
    let current_tab = Tab::Settings;

    // Act
    let spans = tab_segments(current_tab, 0, &[])
        .into_iter()
        .map(|(_, span)| span)
        .collect::<Vec<_>>();

    // Assert
    assert_eq!(spans[0].style.fg, Some(style::palette::text_muted()));
    assert_eq!(spans[2].style.fg, Some(style::palette::text_subtle()));
    assert_eq!(spans[4].style.fg, Some(style::palette::text_muted()));
    assert_eq!(spans[6].style.fg, Some(style::palette::warning()));
    assert_eq!(spans[6].style.bg, Some(style::palette::surface()));
    assert!(spans[6].style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn test_tab_segments_include_selected_project_name_in_project_scope_label() {
    // Arrange
    let current_tab = Tab::Sessions;
    let projects = vec![
        project_list_item(7, Some("Primary"), "/tmp/primary"),
        project_list_item(8, Some("Secondary"), "/tmp/secondary"),
    ];

    // Act
    let spans = tab_segments(current_tab, 7, &projects)
        .into_iter()
        .map(|(_, span)| span)
        .collect::<Vec<_>>();
    let rendered_tabs: String = spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect::<Vec<_>>()
        .join("");

    // Assert
    assert_eq!(
        rendered_tabs,
        " Projects | Project: Primary | Sessions | Settings "
    );
    assert_eq!(spans[2].style.fg, Some(style::palette::accent_soft()));
    assert!(spans[2].style.add_modifier.contains(Modifier::BOLD));
    assert_eq!(spans[4].style.fg, Some(style::palette::warning()));
    assert_eq!(spans[4].style.bg, Some(style::palette::surface()));
}

#[test]
fn test_tab_segments_render_divider_spans_with_border_color() {
    // Arrange
    let current_tab = Tab::Projects;

    // Act
    let spans = tab_segments(current_tab, 0, &[])
        .into_iter()
        .map(|(_, span)| span)
        .collect::<Vec<_>>();

    // Assert
    assert_eq!(spans[1].content.as_ref(), "|");
    assert_eq!(spans[3].content.as_ref(), "|");
    assert_eq!(spans[5].content.as_ref(), "|");
    assert_eq!(spans[1].style.fg, Some(style::palette::border()));
    assert_eq!(spans[3].style.fg, Some(style::palette::border()));
    assert_eq!(spans[5].style.fg, Some(style::palette::border()));
}

#[test]
fn test_tab_segments_dim_project_scope_when_no_project_is_selected() {
    // Arrange
    let current_tab = Tab::Settings;

    // Act
    let spans = tab_segments(current_tab, 0, &[])
        .into_iter()
        .map(|(_, span)| span)
        .collect::<Vec<_>>();

    // Assert
    assert_eq!(spans[2].content.as_ref(), " Project: None ");
    assert_eq!(spans[2].style.fg, Some(style::palette::text_subtle()));
}

/// Creates a `ProjectListItem` for tab-label rendering tests.
fn project_list_item(id: i64, display_name: Option<&str>, path: &str) -> ProjectListItem {
    ProjectListItem {
        active_session_count: 0,
        input_tokens: 0,
        last_session_updated_at: None,
        output_tokens: 0,
        project: Project {
            created_at: 0,
            display_name: display_name.map(std::string::ToString::to_string),
            git_branch: None,
            id,
            is_favorite: false,
            last_opened_at: None,
            path: PathBuf::from(path),
            updated_at: 0,
        },
        session_count: 0,
    }
}

#[test]
fn test_tab_list_region_maps_each_label_to_its_cells_in_tab_all_order() {
    // Arrange
    let segments = tab_segments(Tab::Projects, 0, &[]);
    let inner = Rect::new(2, 1, 60, 1);

    // Act
    let region = tab_list_region(&segments, inner);

    // Assert
    // " Projects " = 10 cells, "|" = 1, " Project: None " = 15, "|", " Sessions
    // " = 10, "|", " Settings ".
    assert_eq!(region.kind, ListRegionKind::Tabs);
    assert_eq!(
        region.items,
        vec![
            ListItemHit {
                area: ViewportRect {
                    height: 1,
                    width: 10,
                    x: 2,
                    y: 1
                },
                index: 0,
            },
            ListItemHit {
                area: ViewportRect {
                    height: 1,
                    width: 10,
                    x: 29,
                    y: 1
                },
                index: 1,
            },
            ListItemHit {
                area: ViewportRect {
                    height: 1,
                    width: 10,
                    x: 40,
                    y: 1
                },
                index: 2,
            },
        ]
    );
}

#[test]
fn test_tab_list_region_clips_labels_to_the_visible_width() {
    // Arrange
    let segments = tab_segments(Tab::Projects, 0, &[]);
    let inner = Rect::new(0, 0, 33, 1);

    // Act
    let region = tab_list_region(&segments, inner);

    // Assert
    assert_eq!(region.items.len(), 2);
    assert_eq!(region.items[1].area.x, 27);
    assert_eq!(region.items[1].area.width, 6);
}

#[test]
fn test_tab_list_region_is_empty_without_an_inner_line() {
    // Arrange
    let segments = tab_segments(Tab::Projects, 0, &[]);

    // Act
    let region = tab_list_region(&segments, Rect::new(0, 0, 40, 0));

    // Assert
    assert_eq!(region.items, Vec::new());
}
