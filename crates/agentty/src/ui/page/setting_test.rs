use ratatui::layout::{Constraint, Rect};
use ratatui::text::Line;
use ratatui::widgets::{Table, TableState};

use super::{
    ROW_HIGHLIGHT_SYMBOL, SettingsPage, TABLE_COLUMN_SPACING, project_section_title,
    render_settings_selector_dropdown, section_table_state, settings_footer_line_for_mode,
    settings_section_height, settings_selector_dropdown_area, settings_selector_dropdown_lines,
    settings_selector_option_window_start, settings_table_rows,
};
use crate::presentation::help_action;
use crate::presentation::setting::{
    LaunchConfigurationListEditorMode, LaunchConfigurationListEditorSnapshot,
    SettingsScreenSnapshot, SettingsSelectorDropdown, SettingsSelectorDropdownOption,
};
use crate::presentation::viewport::{LayoutSnapshot, ListRegionKind};
use crate::ui::render::Page;
use crate::ui::{layout_snapshot, style};

#[test]
fn test_row_highlight_symbol_uses_background_only_selection() {
    // Arrange
    let highlight_symbol = ROW_HIGHLIGHT_SYMBOL;

    // Act
    let is_empty_symbol = highlight_symbol.is_empty();

    // Assert
    assert!(is_empty_symbol);
}

#[test]
fn test_settings_table_column_spacing_is_wider_for_readability() {
    // Arrange
    let expected_spacing = 2;

    // Act
    let spacing = TABLE_COLUMN_SPACING;

    // Assert
    assert_eq!(spacing, expected_spacing);
}

#[test]
fn test_render_uses_palette_text_for_setting_rows() {
    // Arrange
    let settings_rows = [("Theme", "Agentty Default".to_string())];
    let rows = settings_table_rows(&settings_rows);
    let table = Table::new(
        rows,
        [Constraint::Percentage(50), Constraint::Percentage(50)],
    );
    let backend = ratatui::backend::TestBackend::new(100, 20);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create terminal");

    // Act
    terminal
        .draw(|frame| {
            frame.render_widget(table, frame.area());
        })
        .expect("failed to draw settings page");

    // Assert
    let buffer = terminal.backend().buffer();
    let theme_cell = buffer
        .content()
        .iter()
        .find(|cell| cell.symbol() == "T" && cell.fg == style::palette::text())
        .expect("expected Theme row to use palette text");
    assert_eq!(theme_cell.fg, style::palette::text());
}

#[test]
fn test_project_section_title_wraps_project_name_in_quotes() {
    // Arrange
    let project_name = Some("Agentty");

    // Act
    let section_title = project_section_title(project_name);

    // Assert
    assert_eq!(section_title, "'Agentty' settings");
}

#[test]
fn test_project_section_title_falls_back_without_project_name() {
    // Arrange
    let project_name = None;

    // Act
    let section_title = project_section_title(project_name);

    // Assert
    assert_eq!(section_title, "Project settings");
}

#[test]
fn test_settings_section_height_includes_table_chrome() {
    // Arrange
    let setting_row_count = 6;

    // Act
    let height = settings_section_height(setting_row_count);

    // Assert
    assert_eq!(height, Constraint::Length(8));
}

#[test]
fn test_section_table_state_selects_global_row() {
    // Arrange
    let mut table_state = TableState::default();
    table_state.select(Some(0));

    // Act
    let section_state = section_table_state(table_state.selected(), 0, 1);

    // Assert
    assert_eq!(section_state.selected(), Some(0));
}

#[test]
fn test_section_table_state_offsets_project_row_selection() {
    // Arrange
    let mut table_state = TableState::default();
    table_state.select(Some(3));

    // Act
    let section_state = section_table_state(table_state.selected(), 1, 6);

    // Assert
    assert_eq!(section_state.selected(), Some(2));
}

#[test]
fn test_section_table_state_leaves_unselected_rows_unhighlighted() {
    // Arrange
    let mut table_state = TableState::default();
    table_state.select(Some(0));

    // Act
    let section_state = section_table_state(table_state.selected(), 1, 6);

    // Assert
    assert_eq!(section_state.selected(), None);
}

#[test]
fn test_settings_selector_dropdown_lines_highlight_selected_option() {
    // Arrange
    let selector_dropdown = SettingsSelectorDropdown {
        options: vec![
            SettingsSelectorDropdownOption {
                label: "Agentty Default".to_string(),
            },
            SettingsSelectorDropdownOption {
                label: "Agentty Green".to_string(),
            },
        ],
        row_index: 0,
        selected_index: 1,
        title: "Select setting value",
    };

    // Act
    let lines = settings_selector_dropdown_lines(&selector_dropdown, 48, 8);

    // Assert
    assert_eq!(lines.len(), 4);
    assert_eq!(lines[1].spans[0].style.bg, Some(style::palette::accent()));
    assert!(lines[1].to_string().contains("> Agentty Green"));
    assert!(lines[3].to_string().contains("Enter: select"));
}

#[test]
fn test_settings_selector_dropdown_lines_window_to_selected_option() {
    // Arrange
    let selector_dropdown = SettingsSelectorDropdown {
        options: (0..12)
            .map(|option_index| SettingsSelectorDropdownOption {
                label: format!("Option {option_index}"),
            })
            .collect(),
        row_index: 2,
        selected_index: 10,
        title: "Select model",
    };

    // Act
    let lines = settings_selector_dropdown_lines(&selector_dropdown, 48, 7);
    let dropdown_text = lines
        .iter()
        .map(Line::to_string)
        .collect::<Vec<_>>()
        .join("\n");

    // Assert
    assert!(dropdown_text.contains("> Option 10"));
    assert!(!dropdown_text.contains("Option 0"));
    assert!(lines.len() <= 5);
}

#[test]
fn test_settings_selector_option_window_start_keeps_tail_selection_visible() {
    // Arrange
    let option_count = 12;
    let selected_index = 11;
    let visible_option_count = 3;

    // Act
    let window_start =
        settings_selector_option_window_start(option_count, selected_index, visible_option_count);

    // Assert
    assert_eq!(window_start, 9);
}

#[test]
fn test_settings_selector_dropdown_area_stays_in_main_area() {
    // Arrange
    let main_area = Rect::new(0, 0, 80, 18);
    let table_chunks = vec![Rect::new(0, 0, 80, 3), Rect::new(0, 3, 80, 8)];
    let selector_dropdown = SettingsSelectorDropdown {
        options: vec![
            SettingsSelectorDropdownOption {
                label: "low".to_string(),
            },
            SettingsSelectorDropdownOption {
                label: "medium".to_string(),
            },
            SettingsSelectorDropdownOption {
                label: "high".to_string(),
            },
        ],
        row_index: 3,
        selected_index: 2,
        title: "Select reasoning level",
    };

    // Act
    let area = settings_selector_dropdown_area(main_area, &table_chunks, 1, &selector_dropdown);

    // Assert
    assert!(area.x >= main_area.x);
    assert!(area.y >= main_area.y);
    assert!(area.x.saturating_add(area.width) <= main_area.x.saturating_add(main_area.width));
    assert!(area.y.saturating_add(area.height) <= main_area.y.saturating_add(main_area.height));
}

#[test]
fn test_settings_selector_dropdown_renders_stage_title() {
    // Arrange
    let selector_dropdown = SettingsSelectorDropdown {
        options: vec![SettingsSelectorDropdownOption {
            label: "codex/gpt-5.6-sol".to_string(),
        }],
        row_index: 2,
        selected_index: 0,
        title: "Select model",
    };
    let backend = ratatui::backend::TestBackend::new(80, 20);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create terminal");

    // Act
    terminal
        .draw(|frame| {
            let main_area = frame.area();
            let table_chunks = [Rect::new(0, 0, 80, 3), Rect::new(0, 3, 80, 8)];
            render_settings_selector_dropdown(
                frame,
                main_area,
                &table_chunks,
                1,
                &selector_dropdown,
            );
        })
        .expect("failed to draw selector dropdown");
    let rendered_text = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect::<String>();

    // Assert
    assert!(rendered_text.contains("Select model"));
    assert!(rendered_text.contains("codex/gpt-5.6-sol"));
}

#[test]
fn test_settings_footer_line_uses_inline_hint_while_overlay_is_open() {
    // Arrange
    let footer_hint = "Editing launch configurations";

    // Act
    let footer_line = settings_footer_line_for_mode(true, footer_hint);

    // Assert
    assert_eq!(footer_line, Line::from(footer_hint.to_string()));
}

#[test]
fn test_settings_footer_line_uses_shared_actions_in_list_mode() {
    // Arrange
    let footer_hint = "unused while not editing";
    let expected_line =
        crate::ui::help_format::footer_line(&help_action::settings_footer_actions());

    // Act
    let footer_line = settings_footer_line_for_mode(false, footer_hint);

    // Assert
    assert_eq!(footer_line, expected_line);
}

/// Returns the screen row where `needle` is painted.
fn painted_row(buffer: &ratatui::buffer::Buffer, needle: &str) -> u16 {
    (0..buffer.area.height)
        .find(|row| {
            (0..buffer.area.width)
                .map(|column| buffer[(column, *row)].symbol())
                .collect::<String>()
                .contains(needle)
        })
        .expect("needle must be painted")
}

/// Returns the recorded row for `index` in the only list of `kind`.
fn recorded_row(snapshot: &LayoutSnapshot, kind: ListRegionKind, index: usize) -> u16 {
    snapshot
        .lists
        .iter()
        .filter(|list| list.kind == kind)
        .flat_map(|list| list.items.iter())
        .find(|item| item.index == index)
        .expect("item must be recorded")
        .area
        .y
}

fn settings_snapshot() -> SettingsScreenSnapshot {
    SettingsScreenSnapshot {
        footer_hint: "",
        global_rows: vec![
            ("Theme", "Agentty Default".to_string()),
            ("Mouse Support", "Enabled".to_string()),
        ],
        launch_configuration_list_editor: None,
        project_rows: vec![
            ("Default Smart Model", "claude/opus".to_string()),
            ("Launch Configurations", "cargo test".to_string()),
        ],
        selected_row_index: Some(3),
        selector_dropdown: None,
    }
}

fn draw_settings(snapshot: &SettingsScreenSnapshot) -> (ratatui::buffer::Buffer, LayoutSnapshot) {
    let backend = ratatui::backend::TestBackend::new(100, 30);
    let mut terminal = ratatui::Terminal::new(backend).expect("failed to create terminal");
    layout_snapshot::begin_frame();
    terminal
        .draw(|frame| {
            SettingsPage::new(snapshot, None).render(frame, frame.area());
        })
        .expect("failed to draw settings page");

    (
        terminal.backend().buffer().clone(),
        layout_snapshot::take_frame(),
    )
}

#[test]
fn test_render_records_settings_rows_across_both_sections() {
    // Arrange
    let snapshot = settings_snapshot();

    // Act
    let (buffer, layout) = draw_settings(&snapshot);

    // Assert
    assert_eq!(
        recorded_row(&layout, ListRegionKind::Settings, 0),
        painted_row(&buffer, "Theme")
    );
    assert_eq!(
        recorded_row(&layout, ListRegionKind::Settings, 1),
        painted_row(&buffer, "Mouse Support")
    );
    assert_eq!(
        recorded_row(&layout, ListRegionKind::Settings, 2),
        painted_row(&buffer, "Default Smart Model")
    );
    assert_eq!(
        recorded_row(&layout, ListRegionKind::Settings, 3),
        painted_row(&buffer, "Launch Configurations")
    );
}

#[test]
fn test_render_records_open_dropdown_options_after_the_settings_rows() {
    // Arrange
    let mut snapshot = settings_snapshot();
    snapshot.selector_dropdown = Some(SettingsSelectorDropdown {
        options: vec![
            SettingsSelectorDropdownOption {
                label: "Option Alpha".to_string(),
            },
            SettingsSelectorDropdownOption {
                label: "Option Beta".to_string(),
            },
        ],
        row_index: 1,
        selected_index: 1,
        title: "Select setting value",
    });

    // Act
    let (buffer, layout) = draw_settings(&snapshot);

    // Assert
    assert_eq!(
        recorded_row(&layout, ListRegionKind::SettingsSelector, 0),
        painted_row(&buffer, "Option Alpha")
    );
    assert_eq!(
        recorded_row(&layout, ListRegionKind::SettingsSelector, 1),
        painted_row(&buffer, "Option Beta")
    );
    assert_eq!(
        layout.lists.last().map(|list| list.kind),
        Some(ListRegionKind::SettingsSelector),
        "the dropdown is painted after, so hit-tested before, the rows"
    );
}

#[test]
fn test_render_records_launch_configuration_editor_commands() {
    // Arrange
    let mut snapshot = settings_snapshot();
    snapshot.launch_configuration_list_editor = Some(LaunchConfigurationListEditorSnapshot {
        commands: vec!["cargo test".to_string(), "npm run dev".to_string()],
        input: None,
        mode: LaunchConfigurationListEditorMode::Browse,
        selected_index: 1,
    });

    // Act
    let (buffer, layout) = draw_settings(&snapshot);

    // Assert
    assert_eq!(
        recorded_row(&layout, ListRegionKind::LaunchConfigurationEditor, 1),
        painted_row(&buffer, "npm run dev")
    );
}
