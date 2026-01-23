use ratatui::{
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::config::{AVAILABLE_SOUNDS, AVAILABLE_THEMES};
use crate::models::{Plate, PlateStatus};

use super::state::App;

pub fn render(frame: &mut Frame, app: &App) {
    let banner_height = if app.show_auth_banner { 1 } else { 0 };
    let chunks = Layout::vertical([
        Constraint::Length(2),
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
    let open_plates = app.open_plates();
    let closed_plates = app.closed_plates();
    let open_count = open_plates.len();
    let closed_count = closed_plates.len();

    let total_items = if closed_count > 0 {
        if app.closed_expanded {
            open_count + 1 + closed_count
        } else {
            open_count + 1
        }
    } else {
        open_count
    };

    let num_width = if total_items == 0 {
        1
    } else {
        (total_items.ilog10() + 1) as usize
    };
    let prefix_width = 2 + num_width + 1 + 1 + 1 + 1 + 25 + 1 + 8 + 1;
    let summary_width = (area.width as usize).saturating_sub(prefix_width).max(1);
    let mut items: Vec<ListItem> = Vec::new();

    for (idx, plate) in open_plates.iter().enumerate() {
        let is_selected = app.selected_index == Some(idx);
        items.push(render_plate_item(
            app,
            plate,
            idx,
            num_width,
            prefix_width,
            summary_width,
            area.width as usize,
            is_selected,
        ));
    }

    if closed_count > 0 {
        if open_count > 0 {
            items.push(ListItem::new(Line::from("")));
        }

        let closed_header_selected = app.is_on_closed_header();
        let indicator = if app.closed_expanded { "v" } else { ">" };
        let header_text = format!("{} CLOSED ({})", indicator, closed_count);
        let style = if closed_header_selected {
            Style::default()
                .add_modifier(Modifier::DIM)
                .add_modifier(Modifier::REVERSED)
        } else {
            Style::default().add_modifier(Modifier::DIM)
        };
        items.push(ListItem::new(Line::from(Span::styled(header_text, style))));

        if app.closed_expanded {
            for (closed_idx, plate) in closed_plates.iter().enumerate() {
                let display_idx = open_count + 1 + closed_idx;
                let is_selected = app.selected_index == Some(display_idx);
                items.push(render_plate_item(
                    app,
                    plate,
                    display_idx,
                    num_width,
                    prefix_width,
                    summary_width,
                    area.width as usize,
                    is_selected,
                ));
            }
        }
    }

    if items.is_empty() {
        items.push(ListItem::new(Line::from("No plates")));
    }

    let list = List::new(items);
    frame.render_widget(list, area);
}

#[allow(clippy::too_many_arguments)]
fn render_plate_item<'a>(
    app: &App,
    plate: &Plate,
    idx: usize,
    num_width: usize,
    prefix_width: usize,
    summary_width: usize,
    full_width: usize,
    is_selected: bool,
) -> ListItem<'a> {
    let unseen_marker = if app.is_unseen(&plate.session_id) && plate.status.needs_attention() {
        "*"
    } else {
        " "
    };

    let theme = &app.config.theme.name;
    let status_color = status_color(plate.status, theme);
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
        ListItem::new(lines)
    } else {
        let collapsed_summary = full_summary.replace('\n', ". ");
        let display_summary =
            if collapsed_summary.chars().count() > summary_width && summary_width >= 3 {
                let truncated: String = collapsed_summary.chars().take(summary_width - 3).collect();
                format!("{}...", truncated)
            } else {
                collapsed_summary
            };
        let line_text = format!("{} {}", prefix, display_summary);
        ListItem::new(Line::from(Span::styled(line_text, style)))
    }
}

fn render_auth_banner(frame: &mut Frame, area: Rect) {
    let banner = Paragraph::new(
        " No API key configured. Run `sp auth set` for AI summaries. Press 'd' to dismiss. ",
    )
    .style(Style::default().fg(Color::Yellow));
    frame.render_widget(banner, area);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let enter_action = if app.config.tmux_mode {
        "enter:jump"
    } else {
        "enter:resume(closed)"
    };
    let base = format!(
        " q:quit  r:refresh  s:settings  c:closed  {}  del:dismiss",
        enter_action
    );
    let text = if app.show_auth_banner {
        format!("{}  d:dismiss banner ", base)
    } else {
        format!("{} ", base)
    };
    let footer = Paragraph::new(text).style(Style::default().add_modifier(Modifier::DIM));
    frame.render_widget(footer, area);
}

fn render_sound_settings(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let width = 50.min(area.width.saturating_sub(4));
    let height = 14.min(area.height.saturating_sub(4));
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let modal_area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, modal_area);

    let block = Block::default().title(" Settings ").borders(Borders::ALL);
    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    let inner_width = inner.width as usize;

    let rows: Vec<(&str, String)> = vec![
        ("Theme", app.config.theme.name.clone()),
        (
            "Sounds Enabled",
            if app.config.sounds.enabled {
                "yes".to_string()
            } else {
                "no".to_string()
            },
        ),
        ("  Awaiting Input", app.config.sounds.awaiting_input.clone()),
        (
            "  Awaiting Approval",
            app.config.sounds.awaiting_approval.clone(),
        ),
        ("  Idle", app.config.sounds.idle.clone()),
        ("  Error", app.config.sounds.error.clone()),
        ("  Closed", app.config.sounds.closed.clone()),
    ];

    let mut lines: Vec<Line> = Vec::new();
    for (idx, (label, value)) in rows.iter().enumerate() {
        let is_selected = idx == app.sound_settings_row;
        let style = if is_selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        let row_text = format!("{:20} {}", label, value);
        let padded = format!("{:<width$}", row_text, width = inner_width);
        lines.push(Line::from(Span::styled(padded, style)));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "esc:close  arrows:navigate  enter/space:change",
        Style::default().add_modifier(Modifier::DIM),
    )));

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}

fn status_color(status: PlateStatus, theme: &str) -> Color {
    match theme {
        "light" => match status {
            PlateStatus::Starting => Color::Gray,
            PlateStatus::Running => Color::DarkGray,
            PlateStatus::Idle => Color::Blue,
            PlateStatus::AwaitingInput => Color::Red,
            PlateStatus::AwaitingApproval => Color::Magenta,
            PlateStatus::Error => Color::Red,
            PlateStatus::Closed => Color::Gray,
        },
        "monochrome" => Color::Reset,
        _ => match status {
            PlateStatus::Starting => Color::DarkGray,
            PlateStatus::Running => Color::Green,
            PlateStatus::Idle => Color::Cyan,
            PlateStatus::AwaitingInput => Color::Yellow,
            PlateStatus::AwaitingApproval => Color::Magenta,
            PlateStatus::Error => Color::Red,
            PlateStatus::Closed => Color::DarkGray,
        },
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

pub fn next_theme(current: &str) -> &'static str {
    let idx = AVAILABLE_THEMES
        .iter()
        .position(|&s| s == current)
        .unwrap_or(0);
    AVAILABLE_THEMES[(idx + 1) % AVAILABLE_THEMES.len()]
}
