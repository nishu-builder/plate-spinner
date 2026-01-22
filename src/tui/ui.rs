use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::config::AVAILABLE_SOUNDS;
use crate::models::PlateStatus;

use super::state::App;

pub fn render(frame: &mut Frame, app: &App) {
    let banner_height = if app.show_auth_banner { 1 } else { 0 };
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(banner_height),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(frame.area());

    render_header(frame, app, chunks[0]);
    if app.show_auth_banner {
        render_auth_banner(frame, chunks[1]);
    }
    render_plates(frame, app, chunks[2]);
    render_footer(frame, app, chunks[3]);

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

fn render_plates(frame: &mut Frame, app: &App, area: Rect) {
    let plates = app.display_order();
    let num_width = if plates.is_empty() {
        1
    } else {
        (plates.len().ilog10() + 1) as usize
    };
    let prefix_width = 2 + num_width + 1 + 1 + 1 + 1 + 25 + 1 + 8 + 1;
    let summary_width = (area.width as usize).saturating_sub(prefix_width).max(1);
    let mut items: Vec<ListItem> = Vec::new();
    let mut open_count = 0;
    let mut closed_started = false;

    for (idx, plate) in plates.iter().enumerate() {
        if plate.status == PlateStatus::Closed && !closed_started {
            if open_count > 0 {
                items.push(ListItem::new(Line::from("")));
            }
            items.push(ListItem::new(Line::from(Span::styled(
                "CLOSED",
                Style::default().add_modifier(Modifier::DIM),
            ))));
            closed_started = true;
        } else if plate.status != PlateStatus::Closed && !closed_started {
            if open_count == 0 {
                items.push(ListItem::new(Line::from(Span::styled(
                    "OPEN",
                    Style::default().add_modifier(Modifier::DIM),
                ))));
            }
            open_count += 1;
        }

        let is_selected = app.selected_index == Some(idx);
        let unseen_marker = if app.is_unseen(&plate.session_id) && plate.status.needs_attention() {
            "*"
        } else {
            " "
        };

        let status_color = status_color(plate.status);
        let icon = plate.status.icon();

        let label = format_label(plate.project_name(), plate.git_branch.as_deref());

        let status_short = pad_or_truncate(plate.status.short_name(), 8);
        let todo = plate.todo_progress.as_deref().unwrap_or("");
        let summary = plate.summary.as_deref().unwrap_or("");
        let full_summary = if todo.is_empty() {
            summary.to_string()
        } else {
            format!("{} {}", todo, summary)
        };

        let style = if is_selected {
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::REVERSED)
        } else {
            Style::default().fg(status_color)
        };

        let prefix = format!(
            "[{:>width$}]{} {} {} {}",
            idx + 1,
            unseen_marker,
            icon,
            label,
            status_short,
            width = num_width,
        );

        if is_selected {
            let mut lines: Vec<Line> = Vec::new();
            let indent = " ".repeat(prefix_width);
            let summary_lines: Vec<&str> = full_summary.split('\n').collect();
            let full_width = area.width as usize;

            for (line_idx, summary_line) in summary_lines.iter().enumerate() {
                let line_prefix = if line_idx == 0 {
                    format!("{} ", prefix)
                } else {
                    indent.clone()
                };

                if summary_line.chars().count() <= summary_width {
                    let line_text = format!("{}{}", line_prefix, summary_line);
                    let padded = format!("{:<width$}", line_text, width = full_width);
                    lines.push(Line::from(Span::styled(padded, style)));
                } else {
                    for (chunk_idx, chunk) in summary_line
                        .chars()
                        .collect::<Vec<_>>()
                        .chunks(summary_width)
                        .enumerate()
                    {
                        let chunk_prefix = if line_idx == 0 && chunk_idx == 0 {
                            format!("{} ", prefix)
                        } else {
                            indent.clone()
                        };
                        let wrapped: String = chunk.iter().collect();
                        let line_text = format!("{}{}", chunk_prefix, wrapped);
                        let padded = format!("{:<width$}", line_text, width = full_width);
                        lines.push(Line::from(Span::styled(padded, style)));
                    }
                }
            }
            items.push(ListItem::new(lines));
        } else {
            let collapsed_summary = full_summary.replace('\n', ". ");
            let display_summary = if collapsed_summary.chars().count() > summary_width
                && summary_width >= 3
            {
                let truncated: String = collapsed_summary.chars().take(summary_width - 3).collect();
                format!("{}...", truncated)
            } else {
                collapsed_summary
            };
            let line_text = format!("{} {}", prefix, display_summary);
            items.push(ListItem::new(Line::from(Span::styled(line_text, style))));
        }
    }

    if items.is_empty() {
        items.push(ListItem::new(Line::from("No plates")));
    }

    let list = List::new(items);
    frame.render_widget(list, area);
}

fn render_auth_banner(frame: &mut Frame, area: Rect) {
    let banner = Paragraph::new(
        " No API key configured. Run `sp auth set` for AI summaries. Press 'd' to dismiss. ",
    )
    .style(Style::default().fg(Color::Yellow));
    frame.render_widget(banner, area);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let text = if app.show_auth_banner {
        " q:quit  r:refresh  s:sounds  enter:resume  del:dismiss  1-9:jump  esc:deselect  d:dismiss banner "
    } else {
        " q:quit  r:refresh  s:sounds  enter:resume  del:dismiss  1-9:jump  esc:deselect "
    };
    let footer = Paragraph::new(text).style(Style::default().add_modifier(Modifier::DIM));
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
        (
            "Enabled",
            if app.config.sounds.enabled {
                "yes"
            } else {
                "no"
            },
        ),
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

fn status_color(status: PlateStatus) -> Color {
    match status {
        PlateStatus::Starting => Color::DarkGray,
        PlateStatus::Running => Color::Green,
        PlateStatus::Idle => Color::Cyan,
        PlateStatus::AwaitingInput => Color::Yellow,
        PlateStatus::AwaitingApproval => Color::Magenta,
        PlateStatus::Error => Color::Red,
        PlateStatus::Closed => Color::DarkGray,
    }
}

fn pad_or_truncate(s: &str, width: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > width {
        if width >= 3 {
            chars[..width - 3].iter().collect::<String>() + "..."
        } else {
            chars[..width].iter().collect()
        }
    } else {
        format!("{:<width$}", s, width = width)
    }
}

fn format_label(project: &str, branch: Option<&str>) -> String {
    let label = match branch {
        Some(b) => format!("{}/{}", project, b),
        None => project.to_string(),
    };
    pad_or_truncate(&label, 25)
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
