use std::collections::{HashMap, HashSet};

use crate::config::Config;
use crate::models::{Plate, PlateStatus};

pub struct App {
    pub plates: Vec<Plate>,
    pub selected_index: usize,
    pub seen_plates: HashSet<String>,
    pub previous_statuses: HashMap<String, PlateStatus>,
    pub config: Config,
    pub should_quit: bool,
    pub resume_plate: Option<(String, String)>,
    pub show_sound_settings: bool,
    pub sound_settings_row: usize,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            plates: Vec::new(),
            selected_index: 0,
            seen_plates: HashSet::new(),
            previous_statuses: HashMap::new(),
            config,
            should_quit: false,
            resume_plate: None,
            show_sound_settings: false,
            sound_settings_row: 0,
        }
    }

    pub fn display_order(&self) -> Vec<&Plate> {
        let mut open: Vec<_> = self
            .plates
            .iter()
            .filter(|s| s.status != PlateStatus::Closed)
            .collect();
        let mut closed: Vec<_> = self
            .plates
            .iter()
            .filter(|s| s.status == PlateStatus::Closed)
            .collect();

        open.sort_by(|a, b| {
            let a_needs = a.status.needs_attention();
            let b_needs = b.status.needs_attention();
            b_needs.cmp(&a_needs)
        });

        open.append(&mut closed);
        open
    }

    pub fn move_up(&mut self) {
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let max = self.display_order().len().saturating_sub(1);
        if self.selected_index < max {
            self.selected_index += 1;
        }
    }

    pub fn select(&mut self) {
        let plates = self.display_order();
        if let Some(plate) = plates.get(self.selected_index) {
            self.resume_plate = Some((plate.session_id.clone(), plate.project_path.clone()));
            self.should_quit = true;
        }
    }

    pub fn jump(&mut self, index: usize) {
        let max = self.display_order().len();
        if index < max {
            self.selected_index = index;
        }
    }

    pub fn mark_seen(&mut self) {
        let plates = self.display_order();
        if let Some(plate) = plates.get(self.selected_index) {
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
