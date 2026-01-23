use std::collections::{HashMap, HashSet};
use std::process::Command;

use crate::config::{get_data_dir, Config};
use crate::daemon::summarizer::get_api_key;
use crate::models::{Plate, PlateStatus};

pub struct App {
    pub plates: Vec<Plate>,
    pub selected_index: Option<usize>,
    pub seen_plates: HashSet<String>,
    pub previous_statuses: HashMap<String, PlateStatus>,
    pub config: Config,
    pub should_quit: bool,
    pub show_sound_settings: bool,
    pub sound_settings_row: usize,
    pub show_auth_banner: bool,
    pub closed_expanded: bool,
    pub status_message: Option<String>,
}

impl App {
    pub fn new(config: Config) -> Self {
        let has_api_key = get_api_key().is_some();
        let banner_dismissed = get_data_dir().join(".auth_banner_dismissed").exists();
        Self {
            plates: Vec::new(),
            selected_index: Some(0),
            seen_plates: HashSet::new(),
            previous_statuses: HashMap::new(),
            config,
            should_quit: false,
            show_sound_settings: false,
            sound_settings_row: 0,
            show_auth_banner: !has_api_key && !banner_dismissed,
            closed_expanded: false,
            status_message: None,
        }
    }

    pub fn dismiss_auth_banner(&mut self) {
        self.show_auth_banner = false;
        let dismiss_path = get_data_dir().join(".auth_banner_dismissed");
        let _ = std::fs::write(&dismiss_path, "");
    }

    pub fn open_plates(&self) -> Vec<&Plate> {
        let mut open: Vec<_> = self
            .plates
            .iter()
            .filter(|s| s.status != PlateStatus::Closed)
            .collect();

        open.sort_by(|a, b| {
            let a_needs = a.status.needs_attention();
            let b_needs = b.status.needs_attention();
            b_needs.cmp(&a_needs)
        });

        open
    }

    pub fn closed_plates(&self) -> Vec<&Plate> {
        self.plates
            .iter()
            .filter(|s| s.status == PlateStatus::Closed)
            .collect()
    }

    pub fn display_order(&self) -> Vec<&Plate> {
        let mut open = self.open_plates();
        if self.closed_expanded {
            let mut closed = self.closed_plates();
            open.append(&mut closed);
        }
        open
    }

    pub fn is_on_closed_header(&self) -> bool {
        let open_count = self.open_plates().len();
        let closed_count = self.closed_plates().len();
        closed_count > 0 && self.selected_index == Some(open_count)
    }

    pub fn toggle_closed(&mut self) {
        self.closed_expanded = !self.closed_expanded;
    }

    pub fn max_selectable_index(&self) -> usize {
        let open_count = self.open_plates().len();
        let closed_count = self.closed_plates().len();
        if closed_count == 0 {
            open_count.saturating_sub(1)
        } else if self.closed_expanded {
            open_count + closed_count
        } else {
            open_count
        }
    }

    pub fn move_up(&mut self) {
        match self.selected_index {
            Some(idx) if idx > 0 => self.selected_index = Some(idx - 1),
            None => {
                let max = self.max_selectable_index();
                if max > 0 || !self.open_plates().is_empty() || self.closed_plates().len() > 0 {
                    self.selected_index = Some(max);
                }
            }
            _ => {}
        }
    }

    pub fn move_down(&mut self) {
        let max = self.max_selectable_index();
        match self.selected_index {
            Some(idx) if idx < max => self.selected_index = Some(idx + 1),
            None if max > 0 || !self.open_plates().is_empty() || self.closed_plates().len() > 0 => {
                self.selected_index = Some(0)
            }
            _ => {}
        }
    }

    pub fn deselect(&mut self) {
        self.selected_index = None;
    }

    pub fn select(&mut self) {
        if self.is_on_closed_header() {
            self.toggle_closed();
            return;
        }

        if !self.config.tmux_mode {
            return;
        }

        if let Some(plate) = self.selected_plate() {
            if let Some(ref target) = plate.tmux_target {
                let window = target.split(':').last().unwrap_or(target);
                let _ = Command::new("tmux")
                    .args(["select-window", "-t", window])
                    .status();
            } else {
                self.status_message = Some("No tmux window for this plate".to_string());
            }
        }
    }

    pub fn selected_plate(&self) -> Option<&Plate> {
        let idx = self.selected_index?;
        let open_count = self.open_plates().len();

        if idx < open_count {
            self.open_plates().get(idx).copied()
        } else if self.closed_expanded && idx > open_count {
            let closed_idx = idx - open_count - 1;
            self.closed_plates().get(closed_idx).copied()
        } else {
            None
        }
    }

    pub fn jump(&mut self, index: usize) {
        let max = self.max_selectable_index();
        if index <= max {
            self.selected_index = Some(index);
        }
    }

    pub fn mark_seen(&mut self) {
        if let Some(plate) = self.selected_plate() {
            self.seen_plates.insert(plate.session_id.clone());
        }
    }

    pub fn is_unseen(&self, session_id: &str) -> bool {
        !self.seen_plates.contains(session_id)
    }

    pub fn attention_count(&self) -> usize {
        self.plates
            .iter()
            .filter(|s| s.status.needs_attention() && self.is_unseen(&s.session_id))
            .count()
    }
}
