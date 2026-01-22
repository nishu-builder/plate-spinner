use std::collections::{HashMap, HashSet};

use crate::config::Config;
use crate::models::{Session, SessionStatus};

pub struct App {
    pub sessions: Vec<Session>,
    pub selected_index: usize,
    pub seen_sessions: HashSet<String>,
    pub previous_statuses: HashMap<String, SessionStatus>,
    pub config: Config,
    pub should_quit: bool,
    pub resume_session: Option<(String, String)>,
    pub show_sound_settings: bool,
    pub sound_settings_row: usize,
}

impl App {
    pub fn new(config: Config) -> Self {
        Self {
            sessions: Vec::new(),
            selected_index: 0,
            seen_sessions: HashSet::new(),
            previous_statuses: HashMap::new(),
            config,
            should_quit: false,
            resume_session: None,
            show_sound_settings: false,
            sound_settings_row: 0,
        }
    }

    pub fn display_order(&self) -> Vec<&Session> {
        let mut open: Vec<_> = self
            .sessions
            .iter()
            .filter(|s| s.status != SessionStatus::Closed)
            .collect();
        let mut closed: Vec<_> = self
            .sessions
            .iter()
            .filter(|s| s.status == SessionStatus::Closed)
            .collect();

        open.sort_by(|a, b| {
            let a_needs = a.status.needs_attention();
            let b_needs = b.status.needs_attention();
            b_needs.cmp(&a_needs)
        });

        open.extend(closed.drain(..));
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
        let sessions = self.display_order();
        if let Some(session) = sessions.get(self.selected_index) {
            self.resume_session = Some((session.session_id.clone(), session.project_path.clone()));
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
        let sessions = self.display_order();
        if let Some(session) = sessions.get(self.selected_index) {
            self.seen_sessions.insert(session.session_id.clone());
        }
    }

    pub fn is_unseen(&self, session_id: &str) -> bool {
        !self.seen_sessions.contains(session_id)
    }

    pub fn attention_count(&self) -> usize {
        self.sessions
            .iter()
            .filter(|s| s.status.needs_attention() && self.is_unseen(&s.session_id))
            .count()
    }
}
