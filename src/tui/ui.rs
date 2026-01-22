use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::config::AVAILABLE_SOUNDS;
use crate::models::SessionStatus;

use super::state::App;

pub fn render(frame: &mut Frame, app: &App) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(frame.area());

    render_header(frame, app, chunks[0]);
    render_sessions(frame, app, chunks[1]);
    render_footer(frame, chunks[2]);

    if app.show_sound_settings {
        render_sound_settings(frame, app);
    }
}

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let attention = app.attention_count();
    let title = if attention > 0 {
        format!(" Plate Spinner ({}) ", attention)
    } else {
        " Plate Spinner ".to_string()
    };

    let header = Paragraph::new(title).style(Style::default().add_modifier(Modifier::BOLD));
    frame.render_widget(header, area);
}

fn render_sessions(frame: &mut Frame, app: &App, area: Rect) {
    let sessions = app.display_order();
    let mut items: Vec<ListItem> = Vec::new();
    let mut open_count = 0;
    let mut closed_started = false;

    for (idx, session) in sessions.iter().enumerate() {
        if session.status == SessionStatus::Closed && !closed_started {
            if open_count > 0 {
                items.push(ListItem::new(Line::from("")));
            }
            items.push(ListItem::new(Line::from(Span::styled(
                "CLOSED",
                Style::default().add_modifier(Modifier::DIM),
            ))));
            closed_started = true;
        } else if session.status != SessionStatus::Closed && !closed_started {
            if open_count == 0 {
                items.push(ListItem::new(Line::from(Span::styled(
                    "OPEN",
                    Style::default().add_modifier(Modifier::DIM),
                ))));
            }
            open_count += 1;
        }

        let is_selected = idx == app.selected_index;
        let unseen_marker = if app.is_unseen(&session.session_id) && session.status.needs_attention()
        {
            "*"
        } else {
            " "
        };

        let status_color = status_color(session.status);
        let icon = session.status.icon();

        let label = format_label(session.project_name(), session.git_branch.as_deref());

        let status_short = session.status.short_name();
        let todo = session.todo_progress.as_deref().unwrap_or("");
        let summary = session.summary.as_deref().unwrap_or("");

        let line_text = format!(
            "[{}]{} {} {:20} {:8} {:12} {}",
            idx + 1,
            unseen_marker,
            icon,
            label,
            status_short,
            todo,
            summary
        );

        let style = if is_selected {
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(status_color)
        };

        items.push(ListItem::new(Line::from(Span::styled(line_text, style))));
    }

    if items.is_empty() {
        items.push(ListItem::new(Line::from("No sessions")));
    }

    let list = List::new(items);
    frame.render_widget(list, area);
}

fn render_footer(frame: &mut Frame, area: Rect) {
    let footer = Paragraph::new(" q:quit  r:refresh  s:sounds  enter:resume  del:dismiss  1-9:jump ")
        .style(Style::default().add_modifier(Modifier::DIM));
    frame.render_widget(footer, area);
}

fn render_sound_settings(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let width = 50.min(area.width.saturating_sub(4));
    let height = 12.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, modal_area);

    let block = Block::default()
        .title(" Sound Settings ")
        .borders(Borders::ALL);
    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let rows = [
        ("Enabled", if app.config.sounds.enabled { "yes" } else { "no" }),
        ("Awaiting Input", &app.config.sounds.awaiting_input),
        ("Awaiting Approval", &app.config.sounds.awaiting_approval),
        ("Idle", &app.config.sounds.idle),
        ("Error", &app.config.sounds.error),
        ("Closed", &app.config.sounds.closed),
    ];

    let mut lines: Vec<Line> = Vec::new();
    for (idx, (label, value)) in rows.iter().enumerate() {
        let is_selected = idx == app.sound_settings_row;
        let style = if is_selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        lines.push(Line::from(Span::styled(
            format!("{:20} {}", label, value),
            style,
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "esc:close  arrows:navigate  enter/space:change",
        Style::default().add_modifier(Modifier::DIM),
    )));

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}

fn status_color(status: SessionStatus) -> Color {
    match status {
        SessionStatus::Starting => Color::DarkGray,
        SessionStatus::Running => Color::Green,
        SessionStatus::Idle => Color::Cyan,
        SessionStatus::AwaitingInput => Color::Yellow,
        SessionStatus::AwaitingApproval => Color::Magenta,
        SessionStatus::Error => Color::Red,
        SessionStatus::Closed => Color::DarkGray,
    }
}

fn format_label(project: &str, branch: Option<&str>) -> String {
    let label = match branch {
        Some(b) => format!("{}/{}", project, b),
        None => project.to_string(),
    };
    if label.len() > 20 {
        format!("{}...", &label[..17])
    } else {
        label
    }
}

pub fn next_sound(current: &str) -> &'static str {
    let idx = AVAILABLE_SOUNDS
        .iter()
        .position(|&s| s == current)
        .unwrap_or(0);
    AVAILABLE_SOUNDS[(idx + 1) % AVAILABLE_SOUNDS.len()]
}

pub fn prev_sound(current: &str) -> &'static str {
    let idx = AVAILABLE_SOUNDS
        .iter()
        .position(|&s| s == current)
        .unwrap_or(0);
    AVAILABLE_SOUNDS[(idx + AVAILABLE_SOUNDS.len() - 1) % AVAILABLE_SOUNDS.len()]
}
