use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use futures_util::StreamExt;
use tokio::sync::mpsc;

use crate::config::{load_config, play_sound, save_config};
use crate::hook::DAEMON_URL;
use crate::models::{Plate, PlateStatus};

use super::state::App;
use super::ui::{next_sound, prev_sound, render};

pub async fn run() -> Result<()> {
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

    Ok(())
}

async fn handle_key(app: &mut App, key: KeyCode) {
    if app.show_sound_settings {
        handle_sound_settings_key(app, key).await;
        return;
    }

    match key {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc => app.deselect(),
        KeyCode::Char('r') => refresh(app).await,
        KeyCode::Char('s') => {
            app.show_sound_settings = true;
            app.sound_settings_row = 0;
        }
        KeyCode::Char('c') => app.toggle_closed(),
        KeyCode::Char('d') => {
            if app.show_auth_banner {
                app.dismiss_auth_banner();
            }
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
            if (1..=9).contains(&n) {
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
    let url = format!("{}/plates", DAEMON_URL);

    let Ok(resp) = client.get(&url).send().await else {
        return;
    };

    let Ok(plates): Result<Vec<Plate>, _> = resp.json().await else {
        return;
    };

    for plate in &plates {
        let prev_status = app.previous_statuses.get(&plate.session_id);

        if let Some(&prev) = prev_status {
            if prev == PlateStatus::Running && plate.status.needs_attention() {
                if plate.status != PlateStatus::Closed {
                    app.seen_plates.remove(&plate.session_id);
                }

                if app.config.sounds.enabled {
                    let sound = match plate.status {
                        PlateStatus::AwaitingInput => &app.config.sounds.awaiting_input,
                        PlateStatus::AwaitingApproval => &app.config.sounds.awaiting_approval,
                        PlateStatus::Idle => &app.config.sounds.idle,
                        PlateStatus::Error => &app.config.sounds.error,
                        PlateStatus::Closed => &app.config.sounds.closed,
                        _ => "none",
                    };
                    play_sound(sound);
                }
            }
        }

        app.previous_statuses
            .insert(plate.session_id.clone(), plate.status);
    }

    app.plates = plates;

    if let Some(idx) = app.selected_index {
        let max_idx = app.max_selectable_index();
        let has_items = !app.open_plates().is_empty() || !app.closed_plates().is_empty();
        if idx > max_idx {
            app.selected_index = if has_items { Some(max_idx) } else { None };
        }
    }
}

async fn dismiss(app: &mut App) {
    let Some(plate) = app.selected_plate() else {
        return;
    };
    let session_id = plate.session_id.clone();

    let client = reqwest::Client::new();
    let url = format!("{}/plates/{}", DAEMON_URL, session_id);
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
