use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::config::{load_config, play_sound, save_config};
use crate::hook::DAEMON_URL;
use crate::models::{Session, SessionStatus};

use super::state::App;
use super::ui::{next_sound, prev_sound, render};

pub async fn run() -> Result<Option<(String, String)>> {
    let config = load_config();
    let mut app = App::new(config);

    let mut terminal = ratatui::init();

    let (tx, mut rx) = mpsc::channel::<()>(16);

    tokio::spawn(connect_websocket(tx));

    refresh(&mut app).await;

    loop {
        terminal.draw(|f| render(f, &app))?;

        tokio::select! {
            _ = rx.recv() => {
                refresh(&mut app).await;
            }
            _ = tokio::time::sleep(std::time::Duration::from_millis(50)) => {
                if event::poll(std::time::Duration::ZERO)? {
                    if let Event::Key(key) = event::read()? {
                        if key.kind == KeyEventKind::Press {
                            handle_key(&mut app, key.code).await;
                        }
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    ratatui::restore();

    Ok(app.resume_session)
}

async fn handle_key(app: &mut App, key: KeyCode) {
    if app.show_sound_settings {
        handle_sound_settings_key(app, key).await;
        return;
    }

    match key {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('r') => refresh(app).await,
        KeyCode::Char('s') => {
            app.show_sound_settings = true;
            app.sound_settings_row = 0;
        }
        KeyCode::Up => {
            app.move_up();
            app.mark_seen();
        }
        KeyCode::Down => {
            app.move_down();
            app.mark_seen();
        }
        KeyCode::Enter => {
            app.mark_seen();
            app.select();
        }
        KeyCode::Delete | KeyCode::Backspace => dismiss(app).await,
        KeyCode::Char(c) if c.is_ascii_digit() => {
            let n = c.to_digit(10).unwrap_or(0) as usize;
            if n >= 1 && n <= 9 {
                app.jump(n - 1);
                app.mark_seen();
            }
        }
        _ => {}
    }
}

async fn handle_sound_settings_key(app: &mut App, key: KeyCode) {
    match key {
        KeyCode::Esc => app.show_sound_settings = false,
        KeyCode::Up => {
            if app.sound_settings_row > 0 {
                app.sound_settings_row -= 1;
            }
        }
        KeyCode::Down => {
            if app.sound_settings_row < 5 {
                app.sound_settings_row += 1;
            }
        }
        KeyCode::Left | KeyCode::Right | KeyCode::Enter | KeyCode::Char(' ') => {
            let forward = matches!(key, KeyCode::Right | KeyCode::Enter | KeyCode::Char(' '));
            match app.sound_settings_row {
                0 => app.config.sounds.enabled = !app.config.sounds.enabled,
                1 => {
                    app.config.sounds.awaiting_input = if forward {
                        next_sound(&app.config.sounds.awaiting_input)
                    } else {
                        prev_sound(&app.config.sounds.awaiting_input)
                    }
                    .to_string();
                }
                2 => {
                    app.config.sounds.awaiting_approval = if forward {
                        next_sound(&app.config.sounds.awaiting_approval)
                    } else {
                        prev_sound(&app.config.sounds.awaiting_approval)
                    }
                    .to_string();
                }
                3 => {
                    app.config.sounds.idle = if forward {
                        next_sound(&app.config.sounds.idle)
                    } else {
                        prev_sound(&app.config.sounds.idle)
                    }
                    .to_string();
                }
                4 => {
                    app.config.sounds.error = if forward {
                        next_sound(&app.config.sounds.error)
                    } else {
                        prev_sound(&app.config.sounds.error)
                    }
                    .to_string();
                }
                5 => {
                    app.config.sounds.closed = if forward {
                        next_sound(&app.config.sounds.closed)
                    } else {
                        prev_sound(&app.config.sounds.closed)
                    }
                    .to_string();
                }
                _ => {}
            }
            let _ = save_config(&app.config);
        }
        _ => {}
    }
}

async fn refresh(app: &mut App) {
    let client = reqwest::Client::new();
    let url = format!("{}/sessions", DAEMON_URL);

    let Ok(resp) = client.get(&url).send().await else {
        return;
    };

    let Ok(sessions): Result<Vec<Session>, _> = resp.json().await else {
        return;
    };

    for session in &sessions {
        let prev_status = app.previous_statuses.get(&session.session_id);

        if let Some(&prev) = prev_status {
            if prev == SessionStatus::Running && session.status.needs_attention() {
                if session.status != SessionStatus::Closed {
                    app.seen_sessions.remove(&session.session_id);
                }

                if app.config.sounds.enabled {
                    let sound = match session.status {
                        SessionStatus::AwaitingInput => &app.config.sounds.awaiting_input,
                        SessionStatus::AwaitingApproval => &app.config.sounds.awaiting_approval,
                        SessionStatus::Idle => &app.config.sounds.idle,
                        SessionStatus::Error => &app.config.sounds.error,
                        SessionStatus::Closed => &app.config.sounds.closed,
                        _ => "none",
                    };
                    play_sound(sound);
                }
            }
        }

        app.previous_statuses
            .insert(session.session_id.clone(), session.status);
    }

    app.sessions = sessions;

    let max_idx = app.display_order().len().saturating_sub(1);
    if app.selected_index > max_idx {
        app.selected_index = max_idx;
    }
}

async fn dismiss(app: &mut App) {
    let sessions = app.display_order();
    let Some(session) = sessions.get(app.selected_index) else {
        return;
    };

    let client = reqwest::Client::new();
    let url = format!("{}/sessions/{}", DAEMON_URL, session.session_id);
    let _ = client.delete(&url).send().await;

    refresh(app).await;
}

async fn connect_websocket(tx: mpsc::Sender<()>) {
    let url = format!("{}/ws", DAEMON_URL.replace("http://", "ws://"));

    loop {
        if let Ok((ws_stream, _)) = tokio_tungstenite::connect_async(&url).await {
            let (_, mut read) = ws_stream.split();

            while let Some(msg) = read.next().await {
                if msg.is_ok() {
                    let _ = tx.send(()).await;
                }
            }
        }

        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}
