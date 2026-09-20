//! Pointer clicks on lists and menus.
//!
//! A left press over a list item recorded by the last frame selects that
//! item; pressing the item that is already selected activates it, which the
//! event loop delivers as a synthesized `Enter` so pointer and keyboard
//! activation share one code path. Header tab labels switch tabs directly,
//! and diff-mode file entries only select, matching `j`/`k`.
//!
//! Every list checks that its owning mode is active before touching state, so
//! a click on a page list under a modal overlay is ignored even though the
//! page painted it.

use crate::app::{App, Tab};
use crate::presentation::app_mode::AppMode;
use crate::presentation::setting::SettingsAction;
use crate::presentation::viewport::ListRegionKind;
use crate::runtime::{PresentationState, key_handler, mode};

/// What the event loop must do after a mouse event was applied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MouseOutcome {
    /// The already-selected item was clicked; run its `Enter` action.
    Activate,
    /// Nothing changed.
    Ignored,
    /// Visible state changed; repaint.
    Redraw,
    /// A header tab label was clicked.
    SwitchTab(Tab),
}

impl MouseOutcome {
    /// Maps a "did the state change" flag onto a redraw decision.
    pub(crate) fn from_changed(changed: bool) -> Self {
        if changed { Self::Redraw } else { Self::Ignored }
    }

    /// Selection outcome for a click on an item that may already be selected.
    fn from_selection(was_selected: bool, changed: bool) -> Self {
        if was_selected {
            Self::Activate
        } else {
            Self::from_changed(changed)
        }
    }
}

/// Applies a left press on the list item painted at `index`.
pub(crate) fn handle_list_click(
    app: &mut App,
    presentation: &PresentationState,
    kind: ListRegionKind,
    index: usize,
) -> MouseOutcome {
    match kind {
        ListRegionKind::DiffFiles => MouseOutcome::from_changed(mode::diff::handle_file_click(
            app,
            presentation.render_cache_store(),
            index,
        )),
        ListRegionKind::LaunchConfigurationEditor => click_settings_list(
            app,
            index,
            app.settings_presentation
                .is_launch_configuration_list_editor_open()
                && !app
                    .settings_presentation
                    .is_launch_configuration_list_editor_input_active(),
        ),
        ListRegionKind::LaunchConfigurationSelector => {
            click_launch_configuration_selector(app, index)
        }
        ListRegionKind::Projects => click_project_row(app, index),
        ListRegionKind::ProjectSwitcher => click_project_switcher(app, index),
        ListRegionKind::SessionCreation => click_session_creation_option(app, index),
        ListRegionKind::Sessions => click_session_row(app, index),
        ListRegionKind::Settings => click_settings_list(
            app,
            index,
            !app.settings_presentation.is_selector_dropdown_open()
                && !app
                    .settings_presentation
                    .is_launch_configuration_list_editor_open(),
        ),
        ListRegionKind::SettingsSelector => click_settings_list(
            app,
            index,
            app.settings_presentation.is_selector_dropdown_open(),
        ),
        ListRegionKind::StackAppendParent => click_stack_append_parent(app, index),
        ListRegionKind::Tabs => click_tab(app, index),
    }
}

/// Switches to the clicked header tab while the plain list page is active.
fn click_tab(app: &App, index: usize) -> MouseOutcome {
    if !list_page_accepts_clicks(app) {
        return MouseOutcome::Ignored;
    }

    match Tab::ALL.get(index) {
        Some(tab) if *tab != app.tabs.current() => MouseOutcome::SwitchTab(*tab),
        _ => MouseOutcome::Ignored,
    }
}

/// Selects or activates a project row on the Projects tab.
fn click_project_row(app: &mut App, index: usize) -> MouseOutcome {
    if !list_page_accepts_clicks(app) || app.tabs.current() != Tab::Projects {
        return MouseOutcome::Ignored;
    }
    let was_selected = app.projects.selected_project_index() == Some(index);

    MouseOutcome::from_selection(was_selected, app.projects.select_project_index(index))
}

/// Selects or activates a session row on the Sessions tab.
fn click_session_row(app: &mut App, index: usize) -> MouseOutcome {
    if !list_page_accepts_clicks(app)
        || app.tabs.current() != Tab::Sessions
        || index >= app.sessions.sessions().len()
    {
        return MouseOutcome::Ignored;
    }
    let was_selected = app.sessions.selected_session_index() == Some(index);
    app.sessions.select_session_index(Some(index));

    MouseOutcome::from_selection(was_selected, true)
}

/// Selects or activates an item in whichever settings list is active, when
/// `is_active` confirms the clicked list is the one on top.
fn click_settings_list(app: &mut App, index: usize, is_active: bool) -> MouseOutcome {
    if !is_active || !matches!(app.mode, AppMode::List) || app.tabs.current() != Tab::Settings {
        return MouseOutcome::Ignored;
    }
    let was_selected = app.settings_presentation.selected_list_index() == index;
    let view = app.settings.view();
    app.settings_presentation
        .apply(&view, SettingsAction::Select(index));
    let changed = app.settings_presentation.selected_list_index() == index;

    MouseOutcome::from_selection(was_selected, changed)
}

/// Selects or activates an enabled option in the session creation selector.
fn click_session_creation_option(app: &mut App, index: usize) -> MouseOutcome {
    let AppMode::SessionCreation {
        selected_option_index,
    } = app.mode
    else {
        return MouseOutcome::Ignored;
    };
    if !key_handler::session_creation_option_is_enabled(app, index) {
        return MouseOutcome::Ignored;
    }
    if selected_option_index == index {
        return MouseOutcome::Activate;
    }
    key_handler::update_session_creation_selection(app, index);

    MouseOutcome::Redraw
}

/// Selects or activates a project in the switcher popup.
fn click_project_switcher(app: &mut App, index: usize) -> MouseOutcome {
    let AppMode::ProjectSwitcher {
        selected_option_index,
    } = app.mode
    else {
        return MouseOutcome::Ignored;
    };
    if index >= app.projects.mru_project_items().len() {
        return MouseOutcome::Ignored;
    }
    if selected_option_index == index {
        return MouseOutcome::Activate;
    }
    key_handler::update_project_switcher_selection(app, index);

    MouseOutcome::Redraw
}

/// Selects or activates an eligible parent in the append-to-stack popup.
fn click_stack_append_parent(app: &mut App, index: usize) -> MouseOutcome {
    let AppMode::StackAppendParentSelection {
        selected_parent_index,
        session_id,
    } = &app.mode
    else {
        return MouseOutcome::Ignored;
    };
    if index >= key_handler::stack_append_parent_session_ids(app, session_id).len() {
        return MouseOutcome::Ignored;
    }
    if *selected_parent_index == index {
        return MouseOutcome::Activate;
    }
    app.mode = AppMode::StackAppendParentSelection {
        selected_parent_index: index,
        session_id: session_id.clone(),
    };

    MouseOutcome::Redraw
}

/// Selects or activates a command in the launch-configuration selector.
fn click_launch_configuration_selector(app: &mut App, index: usize) -> MouseOutcome {
    let AppMode::LaunchConfigurationSelector {
        commands,
        selected_command_index,
        ..
    } = &mut app.mode
    else {
        return MouseOutcome::Ignored;
    };
    if index >= commands.len() {
        return MouseOutcome::Ignored;
    }
    if *selected_command_index == index {
        return MouseOutcome::Activate;
    }
    *selected_command_index = index;

    MouseOutcome::Redraw
}

/// Reports whether the list page itself, rather than a settings overlay on
/// top of it, receives clicks.
fn list_page_accepts_clicks(app: &App) -> bool {
    matches!(app.mode, AppMode::List)
        && !app.settings_presentation.is_selector_dropdown_open()
        && !app
            .settings_presentation
            .is_launch_configuration_list_editor_open()
}

#[cfg(test)]
#[path = "click_handler_test.rs"]
mod tests;
