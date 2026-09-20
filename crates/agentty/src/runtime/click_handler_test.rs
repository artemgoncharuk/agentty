use std::path::PathBuf;

use super::{MouseOutcome, handle_list_click};
use crate::app::{App, Tab};
use crate::domain::project::{Project, ProjectListItem};
use crate::domain::session::Status;
use crate::presentation::app_mode::{
    AppMode, ConfirmationViewMode, DiffFocus, DiffLineComments, DiffPreview,
};
use crate::presentation::setting::SettingsAction;
use crate::presentation::viewport::ListRegionKind;
use crate::runtime::PresentationState;

/// Builds one project row for list-page click tests.
fn project_item(id: i64, name: &str) -> ProjectListItem {
    ProjectListItem {
        active_session_count: 0,
        input_tokens: 0,
        last_session_updated_at: None,
        output_tokens: 0,
        project: Project {
            created_at: 0,
            display_name: Some(name.to_string()),
            git_branch: None,
            id,
            is_favorite: false,
            last_opened_at: Some(id),
            path: PathBuf::from(format!("/tmp/{name}")),
            updated_at: 0,
        },
        session_count: 0,
    }
}

/// Creates a list-mode app on `tab` with two project rows.
async fn list_app_on(tab: Tab) -> (App, tempfile::TempDir) {
    let (mut app, base_dir) = crate::test_support::new_git_test_app().await;
    app.projects
        .replace_project_items(vec![project_item(1, "alpha"), project_item(2, "beta")]);
    app.tabs.set(tab);
    app.mode = AppMode::List;

    (app, base_dir)
}

/// Creates a list-mode app on the Sessions tab with two review sessions.
async fn sessions_app() -> (App, tempfile::TempDir) {
    let (mut app, base_dir) = crate::test_support::new_git_test_app_with_mock_tmux_client().await;
    for _ in 0..2 {
        let session_id = app
            .create_session()
            .await
            .expect("failed to create session");
        crate::test_support::set_session_status_for_test(&mut app, &session_id, Status::Review);
    }
    app.tabs.set(Tab::Sessions);
    app.mode = AppMode::List;
    app.sessions.select_session_index(Some(0));

    (app, base_dir)
}

fn click(app: &mut App, kind: ListRegionKind, index: usize) -> MouseOutcome {
    handle_list_click(app, &PresentationState::default(), kind, index)
}

#[tokio::test]
async fn test_tab_click_switches_to_another_tab_and_ignores_the_current_one() {
    // Arrange
    let (mut app, _base_dir) = list_app_on(Tab::Projects).await;

    // Act
    let switched = click(&mut app, ListRegionKind::Tabs, 2);
    let same = click(&mut app, ListRegionKind::Tabs, 0);
    let out_of_range = click(&mut app, ListRegionKind::Tabs, 9);

    // Assert
    assert_eq!(switched, MouseOutcome::SwitchTab(Tab::Settings));
    assert_eq!(same, MouseOutcome::Ignored);
    assert_eq!(out_of_range, MouseOutcome::Ignored);
}

#[tokio::test]
async fn test_tab_click_is_ignored_under_an_overlay_or_open_settings_dropdown() {
    // Arrange
    let (mut app, _base_dir) = list_app_on(Tab::Settings).await;
    let view = app.settings.view();
    app.settings_presentation
        .apply(&view, SettingsAction::Activate);
    assert!(app.settings_presentation.is_selector_dropdown_open());

    // Act
    let under_dropdown = click(&mut app, ListRegionKind::Tabs, 0);
    app.settings_presentation
        .apply(&view, SettingsAction::Cancel);
    app.mode = AppMode::SessionCreation {
        selected_option_index: 0,
    };
    let under_overlay = click(&mut app, ListRegionKind::Tabs, 0);

    // Assert
    assert_eq!(under_dropdown, MouseOutcome::Ignored);
    assert_eq!(under_overlay, MouseOutcome::Ignored);
}

#[tokio::test]
async fn test_project_row_click_selects_then_activates() {
    // Arrange
    let (mut app, _base_dir) = list_app_on(Tab::Projects).await;
    app.projects.select_project_index(0);

    // Act
    let selected = click(&mut app, ListRegionKind::Projects, 1);
    let selected_index = app.projects.selected_project_index();
    let activated = click(&mut app, ListRegionKind::Projects, 1);
    let out_of_range = click(&mut app, ListRegionKind::Projects, 5);

    // Assert
    assert_eq!(selected, MouseOutcome::Redraw);
    assert_eq!(selected_index, Some(1));
    assert_eq!(activated, MouseOutcome::Activate);
    assert_eq!(out_of_range, MouseOutcome::Ignored);
}

#[tokio::test]
async fn test_project_row_click_is_ignored_on_another_tab() {
    // Arrange
    let (mut app, _base_dir) = list_app_on(Tab::Sessions).await;

    // Act
    let outcome = click(&mut app, ListRegionKind::Projects, 1);

    // Assert
    assert_eq!(outcome, MouseOutcome::Ignored);
}

#[tokio::test]
async fn test_session_row_click_selects_then_activates() {
    // Arrange
    let (mut app, _base_dir) = sessions_app().await;

    // Act
    let selected = click(&mut app, ListRegionKind::Sessions, 1);
    let selected_index = app.sessions.selected_session_index();
    let activated = click(&mut app, ListRegionKind::Sessions, 1);
    let out_of_range = click(&mut app, ListRegionKind::Sessions, 2);

    // Assert
    assert_eq!(selected, MouseOutcome::Redraw);
    assert_eq!(selected_index, Some(1));
    assert_eq!(activated, MouseOutcome::Activate);
    assert_eq!(out_of_range, MouseOutcome::Ignored);
}

#[tokio::test]
async fn test_session_row_click_is_ignored_on_another_tab() {
    // Arrange
    let (mut app, _base_dir) = sessions_app().await;
    app.tabs.set(Tab::Projects);

    // Act
    let outcome = click(&mut app, ListRegionKind::Sessions, 1);

    // Assert
    assert_eq!(outcome, MouseOutcome::Ignored);
}

#[tokio::test]
async fn test_settings_row_click_selects_then_activates() {
    // Arrange
    let (mut app, _base_dir) = list_app_on(Tab::Settings).await;

    // Act
    let selected = click(&mut app, ListRegionKind::Settings, 3);
    let selected_index = app.settings_presentation.selected_list_index();
    let activated = click(&mut app, ListRegionKind::Settings, 3);
    let out_of_range = click(&mut app, ListRegionKind::Settings, 99);

    // Assert
    assert_eq!(selected, MouseOutcome::Redraw);
    assert_eq!(selected_index, 3);
    assert_eq!(activated, MouseOutcome::Activate);
    assert_eq!(out_of_range, MouseOutcome::Ignored);
}

#[tokio::test]
async fn test_settings_row_click_is_ignored_off_the_settings_tab_and_under_a_dropdown() {
    // Arrange
    let (mut app, _base_dir) = list_app_on(Tab::Projects).await;
    let off_tab = click(&mut app, ListRegionKind::Settings, 1);
    app.tabs.set(Tab::Settings);
    let view = app.settings.view();
    app.settings_presentation
        .apply(&view, SettingsAction::Activate);

    // Act
    let under_dropdown = click(&mut app, ListRegionKind::Settings, 1);

    // Assert
    assert_eq!(off_tab, MouseOutcome::Ignored);
    assert_eq!(under_dropdown, MouseOutcome::Ignored);
    assert_eq!(app.settings_presentation.selected_list_index(), 0);
}

#[tokio::test]
async fn test_settings_selector_click_selects_an_option_then_activates() {
    // Arrange
    let (mut app, _base_dir) = list_app_on(Tab::Settings).await;
    let closed = click(&mut app, ListRegionKind::SettingsSelector, 1);
    let view = app.settings.view();
    app.settings_presentation
        .apply(&view, SettingsAction::Activate);

    // Act
    let selected = click(&mut app, ListRegionKind::SettingsSelector, 1);
    let selected_index = app.settings_presentation.selected_list_index();
    let activated = click(&mut app, ListRegionKind::SettingsSelector, 1);
    let out_of_range = click(&mut app, ListRegionKind::SettingsSelector, 99);

    // Assert
    assert_eq!(closed, MouseOutcome::Ignored);
    assert_eq!(selected, MouseOutcome::Redraw);
    assert_eq!(selected_index, 1);
    assert_eq!(activated, MouseOutcome::Activate);
    assert_eq!(out_of_range, MouseOutcome::Ignored);
    assert!(app.settings_presentation.is_selector_dropdown_open());
}

#[tokio::test]
async fn test_launch_configuration_editor_click_selects_a_command_then_activates() {
    // Arrange
    let (mut app, _base_dir) = list_app_on(Tab::Settings).await;
    app.settings.launch_configuration = "cargo test\nnpm run dev".to_string();
    let closed = click(&mut app, ListRegionKind::LaunchConfigurationEditor, 1);
    let view = app.settings.view();
    app.settings_presentation
        .apply(&view, SettingsAction::Select(8));
    app.settings_presentation
        .apply(&view, SettingsAction::Activate);
    assert!(
        app.settings_presentation
            .is_launch_configuration_list_editor_open()
    );

    // Act
    let selected = click(&mut app, ListRegionKind::LaunchConfigurationEditor, 1);
    let selected_index = app.settings_presentation.selected_list_index();
    let activated = click(&mut app, ListRegionKind::LaunchConfigurationEditor, 1);
    app.settings_presentation
        .apply(&view, SettingsAction::StartAddingLaunchConfiguration);
    let while_typing = click(&mut app, ListRegionKind::LaunchConfigurationEditor, 0);

    // Assert
    assert_eq!(closed, MouseOutcome::Ignored);
    assert_eq!(selected, MouseOutcome::Redraw);
    assert_eq!(selected_index, 1);
    assert_eq!(activated, MouseOutcome::Activate);
    assert_eq!(while_typing, MouseOutcome::Ignored);
}

#[tokio::test]
async fn test_session_creation_click_selects_enabled_options_only() {
    // Arrange
    let (mut app, _base_dir) = list_app_on(Tab::Sessions).await;
    let outside = click(&mut app, ListRegionKind::SessionCreation, 1);
    app.mode = AppMode::SessionCreation {
        selected_option_index: 0,
    };

    // Act
    let selected = click(&mut app, ListRegionKind::SessionCreation, 1);
    let selected_option_after_click = matches!(
        app.mode,
        AppMode::SessionCreation {
            selected_option_index: 1
        }
    );
    let activated = click(&mut app, ListRegionKind::SessionCreation, 1);
    let disabled = click(&mut app, ListRegionKind::SessionCreation, 3);

    // Assert
    assert_eq!(outside, MouseOutcome::Ignored);
    assert_eq!(selected, MouseOutcome::Redraw);
    assert!(selected_option_after_click);
    assert_eq!(activated, MouseOutcome::Activate);
    assert_eq!(disabled, MouseOutcome::Ignored, "Stacked needs a parent");
}

#[tokio::test]
async fn test_project_switcher_click_selects_then_activates_within_the_mru_list() {
    // Arrange
    let (mut app, _base_dir) = list_app_on(Tab::Sessions).await;
    let outside = click(&mut app, ListRegionKind::ProjectSwitcher, 1);
    app.mode = AppMode::ProjectSwitcher {
        selected_option_index: 0,
    };

    // Act
    let selected = click(&mut app, ListRegionKind::ProjectSwitcher, 1);
    let selected_option_after_click = matches!(
        app.mode,
        AppMode::ProjectSwitcher {
            selected_option_index: 1
        }
    );
    let activated = click(&mut app, ListRegionKind::ProjectSwitcher, 1);
    let out_of_range = click(&mut app, ListRegionKind::ProjectSwitcher, 7);

    // Assert
    assert_eq!(outside, MouseOutcome::Ignored);
    assert_eq!(selected, MouseOutcome::Redraw);
    assert!(selected_option_after_click);
    assert_eq!(activated, MouseOutcome::Activate);
    assert_eq!(out_of_range, MouseOutcome::Ignored);
}

#[tokio::test]
async fn test_stack_append_parent_click_selects_then_activates_an_eligible_parent() {
    // Arrange
    let (mut app, _base_dir) = sessions_app().await;
    let outside = click(&mut app, ListRegionKind::StackAppendParent, 0);
    let source_session_id = app.sessions.sessions()[1].id.clone();
    app.mode = AppMode::StackAppendParentSelection {
        selected_parent_index: 5,
        session_id: source_session_id,
    };

    // Act
    let selected = click(&mut app, ListRegionKind::StackAppendParent, 0);
    let selected_parent_after_click = matches!(
        app.mode,
        AppMode::StackAppendParentSelection {
            selected_parent_index: 0,
            ..
        }
    );
    let activated = click(&mut app, ListRegionKind::StackAppendParent, 0);
    let out_of_range = click(&mut app, ListRegionKind::StackAppendParent, 1);

    // Assert
    assert_eq!(outside, MouseOutcome::Ignored);
    assert_eq!(selected, MouseOutcome::Redraw);
    assert!(selected_parent_after_click);
    assert_eq!(activated, MouseOutcome::Activate);
    assert_eq!(
        out_of_range,
        MouseOutcome::Ignored,
        "only one other session can parent"
    );
}

#[tokio::test]
async fn test_launch_configuration_selector_click_selects_then_activates() {
    // Arrange
    let (mut app, _base_dir) = list_app_on(Tab::Sessions).await;
    let outside = click(&mut app, ListRegionKind::LaunchConfigurationSelector, 1);
    app.mode = AppMode::LaunchConfigurationSelector {
        commands: vec!["cargo test".to_string(), "npm run dev".to_string()],
        restore_view: ConfirmationViewMode {
            scroll_offset: None,
            session_id: "session-1".into(),
        },
        selected_command_index: 0,
    };

    // Act
    let selected = click(&mut app, ListRegionKind::LaunchConfigurationSelector, 1);
    let selected_command_after_click = matches!(
        app.mode,
        AppMode::LaunchConfigurationSelector {
            selected_command_index: 1,
            ..
        }
    );
    let activated = click(&mut app, ListRegionKind::LaunchConfigurationSelector, 1);
    let out_of_range = click(&mut app, ListRegionKind::LaunchConfigurationSelector, 2);

    // Assert
    assert_eq!(outside, MouseOutcome::Ignored);
    assert_eq!(selected, MouseOutcome::Redraw);
    assert!(selected_command_after_click);
    assert_eq!(activated, MouseOutcome::Activate);
    assert_eq!(out_of_range, MouseOutcome::Ignored);
}

#[tokio::test]
async fn test_diff_file_click_selects_without_activating() {
    // Arrange
    let mut app = crate::test_support::new_test_app_without_retained_base_dir().await;
    let outside = click(&mut app, ListRegionKind::DiffFiles, 1);
    app.mode = AppMode::Diff {
        diff: "diff --git a/a.rs b/a.rs\n@@ -0,0 +1 @@\n+one\ndiff --git a/b.rs b/b.rs\n@@ -0,0 \
               +1 @@\n+two"
            .to_string(),
        file_explorer_selected_index: 0,
        focus: DiffFocus::Files,
        line_comments: DiffLineComments::default(),
        preview: DiffPreview::default(),
        review_comments: None,
        restore: None,
        scroll_cache: None,
        scroll_offset: 0,
        selected_diff_line_index: 0,
        session_id: "session-1".into(),
    };

    // Act
    let selected = click(&mut app, ListRegionKind::DiffFiles, 1);
    let repeated = click(&mut app, ListRegionKind::DiffFiles, 1);

    // Assert
    assert_eq!(outside, MouseOutcome::Ignored);
    assert_eq!(selected, MouseOutcome::Redraw);
    assert!(matches!(
        app.mode,
        AppMode::Diff {
            file_explorer_selected_index: 1,
            ..
        }
    ));
    assert_eq!(
        repeated,
        MouseOutcome::Ignored,
        "files select but never activate"
    );
}
