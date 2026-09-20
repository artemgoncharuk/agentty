use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph};

use crate::app::Tab;
use crate::domain::project::ProjectListItem;
use crate::presentation::app_mode::ViewportRect;
use crate::presentation::viewport::{ListItemHit, ListRegion, ListRegionKind};
use crate::ui::{Component, layout_snapshot, style};

/// Header tabs rendered at the top of list mode pages.
pub struct Tabs<'a> {
    active_project_id: i64,
    current_tab: Tab,
    projects: &'a [ProjectListItem],
}

impl Tabs<'_> {
    /// Creates a tabs component with the provided active tab and project
    /// context used to label the project-scoped tab group.
    pub fn new(current_tab: Tab, active_project_id: i64, projects: &[ProjectListItem]) -> Tabs<'_> {
        Tabs {
            active_project_id,
            current_tab,
            projects,
        }
    }
}

impl Component for Tabs<'_> {
    fn render(&self, f: &mut Frame, area: Rect) {
        let segments = tab_segments(self.current_tab, self.active_project_id, self.projects);
        let block = Block::default()
            .borders(Borders::BOTTOM)
            .border_style(style::border_style())
            .padding(Padding::top(1));
        layout_snapshot::record_list(tab_list_region(&segments, block.inner(area)));
        let line = Line::from(
            segments
                .into_iter()
                .map(|(_, span)| span)
                .collect::<Vec<_>>(),
        );
        let paragraph = Paragraph::new(line).block(block);
        f.render_widget(paragraph, area);
    }
}

/// Returns the header line as styled spans, each paired with the tab it
/// selects when clicked (`None` for separators and the project label).
fn tab_segments(
    current_tab: Tab,
    active_project_id: i64,
    projects: &[ProjectListItem],
) -> Vec<(Option<Tab>, Span<'static>)> {
    let tab_count = Tab::project_scoped_tabs().len();
    let mut segments = Vec::with_capacity(2 * tab_count + 3);

    segments.push((Some(Tab::Projects), tab_span(Tab::Projects, current_tab)));
    segments.push((None, tab_separator_span()));
    segments.push((None, project_context_span(active_project_id, projects)));

    for tab in Tab::project_scoped_tabs() {
        segments.push((None, tab_separator_span()));
        segments.push((Some(*tab), tab_span(*tab, current_tab)));
    }

    segments
}

/// Maps each tab label to the header cells it occupies on the first inner
/// line, indexed by `Tab::ALL` order, clipped to the visible width.
fn tab_list_region(segments: &[(Option<Tab>, Span<'static>)], inner: Rect) -> ListRegion {
    let right_edge = inner.x.saturating_add(inner.width);
    let mut next_x = inner.x;
    let mut items = Vec::new();

    for (tab, span) in segments {
        let width = u16::try_from(span.width()).unwrap_or(u16::MAX);
        if next_x >= right_edge || inner.height == 0 {
            break;
        }
        if let Some(tab) = tab
            && let Some(index) = Tab::ALL.iter().position(|candidate| candidate == tab)
        {
            items.push(ListItemHit {
                area: ViewportRect {
                    height: 1,
                    width: width.min(right_edge.saturating_sub(next_x)),
                    x: next_x,
                    y: inner.y,
                },
                index,
            });
        }
        next_x = next_x.saturating_add(width);
    }

    ListRegion {
        items,
        kind: ListRegionKind::Tabs,
    }
}

/// Returns one styled separator span between tabs.
fn tab_separator_span() -> Span<'static> {
    Span::styled("|", Style::default().fg(style::palette::border()))
}

/// Returns one styled tab span with active/inactive affordance treatment.
fn tab_span(tab: Tab, current_tab: Tab) -> Span<'static> {
    let label = format!(" {} ", tab_label(tab));

    if tab == current_tab {
        return Span::styled(
            label,
            Style::default()
                .bg(style::palette::surface())
                .fg(style::palette::warning())
                .add_modifier(Modifier::BOLD),
        );
    }

    Span::styled(label, Style::default().fg(style::palette::text_muted()))
}

/// Returns a styled span describing the active project for project-scoped tabs.
fn project_context_span(active_project_id: i64, projects: &[ProjectListItem]) -> Span<'static> {
    let (project_name, project_scope_style) = active_project_name(active_project_id, projects)
        .map_or_else(
            || {
                (
                    "None".to_string(),
                    Style::default().fg(style::palette::text_subtle()),
                )
            },
            |project_name| {
                (
                    project_name,
                    Style::default()
                        .fg(style::palette::accent_soft())
                        .add_modifier(Modifier::BOLD),
                )
            },
        );
    let project_scope = format!(" Project: {project_name} ");

    Span::styled(project_scope, project_scope_style)
}

/// Returns the display label for a top-level tab.
fn tab_label(tab: Tab) -> &'static str {
    tab.title()
}

/// Returns the active project name shown before project-scoped tabs.
fn active_project_name(active_project_id: i64, projects: &[ProjectListItem]) -> Option<String> {
    projects
        .iter()
        .find(|project_item| project_item.project.id == active_project_id)
        .map(|project_item| project_item.project.display_label())
}

#[cfg(test)]
#[path = "tab_test.rs"]
mod tests;
