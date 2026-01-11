use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use envhub_core::{State, load_state, set_active_profile};
use std::io;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Apps,
    Profiles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    AddApp,
    AddProfile,
    SetEnv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputStep {
    First,
    Second,
}

#[derive(Debug, Clone)]
pub struct InputState {
    pub mode: InputMode,
    pub step: InputStep,
    pub buf: String,
    pub first: String,
    pub second: String,
}

impl InputState {
    fn new() -> Self {
        Self {
            mode: InputMode::Normal,
            step: InputStep::First,
            buf: String::new(),
            first: String::new(),
            second: String::new(),
        }
    }

    fn reset(&mut self) {
        self.mode = InputMode::Normal;
        self.step = InputStep::First;
        self.buf.clear();
        self.first.clear();
        self.second.clear();
    }
}

#[derive(Debug, Clone)]
pub struct AppEntry {
    pub name: String,
    pub active_profile: Option<String>,
    pub profiles: Vec<String>,
}

#[derive(Debug)]
pub struct App {
    pub entries: Vec<AppEntry>,
    pub selected_app: usize,
    pub selected_profile: usize,
    pub focus: Focus,
    pub status: String,
    pub input: InputState,
    pub state: State,
}

impl App {
    pub fn load() -> io::Result<Self> {
        let state =
            load_state().map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;
        Ok(Self::from_state(&state))
    }

    pub fn from_state(state: &State) -> Self {
        let mut entries = Vec::new();
        // Sort keys for consistent order
        let mut app_names: Vec<_> = state.apps.keys().collect();
        app_names.sort();

        for name in app_names {
            if let Some(app) = state.apps.get(name) {
                let mut profiles: Vec<_> = app.profiles.keys().cloned().collect();
                profiles.sort();

                entries.push(AppEntry {
                    name: name.clone(),
                    active_profile: app.active_profile.clone(),
                    profiles,
                });
            }
        }

        let mut app = Self {
            entries,
            selected_app: 0,
            selected_profile: 0,
            focus: Focus::Apps,
            status: "Ready".to_string(),
            input: InputState::new(),
            state: state.clone(),
        };
        app.snap_to_active_profile();
        app
    }

    pub fn update_from_state(&mut self, state: State) {
        let mut entries = Vec::new();
        let mut app_names: Vec<_> = state.apps.keys().collect();
        app_names.sort();

        for name in app_names {
            if let Some(app) = state.apps.get(name) {
                let mut profiles: Vec<_> = app.profiles.keys().cloned().collect();
                profiles.sort();

                entries.push(AppEntry {
                    name: name.clone(),
                    active_profile: app.active_profile.clone(),
                    profiles,
                });
            }
        }

        self.state = state;
        self.entries = entries;
        if self.selected_app >= self.entries.len() {
            self.selected_app = self.entries.len().saturating_sub(1);
            self.selected_profile = 0;
        }
        let profile_len = self.current_profiles().len();
        if self.selected_profile >= profile_len {
            self.selected_profile = profile_len.saturating_sub(1);
        }
        self.snap_to_active_profile();
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> io::Result<bool> {
        if self.entries.is_empty() && self.input.mode == InputMode::Normal {
            if matches!(key.code, KeyCode::Char('a') | KeyCode::Char('q')) {
                // fall through
            } else if key.code == KeyCode::Tab {
                return Ok(false);
            } else if matches!(key.code, KeyCode::Up | KeyCode::Down | KeyCode::Enter) {
                return Ok(false);
            }
        }
        if self.input.mode != InputMode::Normal {
            return self.handle_input(key);
        }
        match key.code {
            KeyCode::Char('q') => return Ok(true),
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => return Ok(true),
            KeyCode::Char('r') => {
                let state = load_state()
                    .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;
                self.update_from_state(state);
                self.status = "Reloaded".to_string();
            }
            KeyCode::Char('a') => {
                self.input.mode = InputMode::AddApp;
                self.input.step = InputStep::First;
                self.input.buf.clear();
                self.status = "Add app: enter name".to_string();
            }
            KeyCode::Char('p') => {
                if self.current_app_name().is_none() {
                    self.status = "Select an app first".to_string();
                } else {
                    self.input.mode = InputMode::AddProfile;
                    self.input.step = InputStep::First;
                    self.input.buf.clear();
                    self.status = "Add profile: enter name".to_string();
                }
            }
            KeyCode::Char('e') => {
                if self.current_profile_name().is_none() {
                    self.status = "Select a profile first".to_string();
                } else {
                    self.input.mode = InputMode::SetEnv;
                    self.input.step = InputStep::First;
                    self.input.buf.clear();
                    let profile = self.current_profile_name().unwrap_or_default();
                    self.status = format!("Set env for profile {profile}: enter key");
                }
            }
            KeyCode::Tab => {
                self.focus = match self.focus {
                    Focus::Apps => Focus::Profiles,
                    Focus::Profiles => Focus::Apps,
                };
            }
            KeyCode::Up => self.move_selection(-1),
            KeyCode::Down => self.move_selection(1),
            KeyCode::Enter => self.activate_profile()?,
            _ => {}
        }
        Ok(false)
    }

    fn handle_input(&mut self, key: KeyEvent) -> io::Result<bool> {
        match key.code {
            KeyCode::Esc => {
                self.input.reset();
                self.status = "Cancelled".to_string();
            }
            KeyCode::Backspace => {
                self.input.buf.pop();
            }
            KeyCode::Enter => {
                self.commit_input()?;
            }
            KeyCode::Char(ch) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    return Ok(false);
                }
                self.input.buf.push(ch);
            }
            _ => {}
        }
        Ok(false)
    }

    fn commit_input(&mut self) -> io::Result<()> {
        let value = self.input.buf.trim().to_string();
        if value.is_empty() {
            self.status = "Input cannot be empty".to_string();
            return Ok(());
        }

        match (self.input.mode, self.input.step) {
            (InputMode::AddApp, InputStep::First) => {
                self.input.first = value;
                self.input.buf.clear();
                self.input.step = InputStep::Second;
                self.status = "Add app: enter target binary".to_string();
            }
            (InputMode::AddApp, InputStep::Second) => {
                self.input.second = value;
                let name = self.input.first.clone();
                let target = self.input.second.clone();
                match envhub_core::register_app(&name, &target) {
                    Ok(()) => {
                        self.status = format!("registered {name} -> {target}");
                        if let Ok(state) = load_state() {
                            self.update_from_state(state);
                        }
                    }
                    Err(err) => self.status = format!("Failed to register: {err}"),
                }
                self.input.reset();
            }
            (InputMode::AddProfile, InputStep::First) => {
                if let Some(app) = self.current_app_name() {
                    match envhub_core::add_profile(&app, &value) {
                        Ok(()) => {
                            self.status = format!("profile {value} added to {app}");
                            if let Ok(state) = load_state() {
                                self.update_from_state(state);
                            }
                        }
                        Err(err) => self.status = format!("Failed to add profile: {err}"),
                    }
                }
                self.input.reset();
            }
            (InputMode::SetEnv, InputStep::First) => {
                self.input.first = value;
                self.input.buf.clear();
                self.input.step = InputStep::Second;
                self.status = "Set env: enter value".to_string();
            }
            (InputMode::SetEnv, InputStep::Second) => {
                let app = self.current_app_name();
                let profile_name = self.current_profile_name();
                let key = self.input.first.clone();
                let env_value = value;
                if let (Some(app), Some(profile_name)) = (app, profile_name) {
                    match envhub_core::set_profile_env(&app, &profile_name, &key, &env_value) {
                        Ok(()) => {
                            self.status = format!("env {key} set for {app}:{profile_name}");
                            if let Ok(state) = load_state() {
                                self.update_from_state(state);
                            }
                        }
                        Err(err) => self.status = format!("Failed to set env: {err}"),
                    }
                }
                self.input.reset();
            }
            _ => {
                self.input.reset();
            }
        }
        Ok(())
    }

    fn move_selection(&mut self, delta: isize) {
        match self.focus {
            Focus::Apps => {
                let len = self.entries.len();
                self.selected_app = next_index(self.selected_app, len, delta);
                self.snap_to_active_profile();
            }
            Focus::Profiles => {
                let len = self.current_profiles().len();
                self.selected_profile = next_index(self.selected_profile, len, delta);
            }
        }
    }

    pub fn current_profiles(&self) -> Vec<String> {
        self.entries
            .get(self.selected_app)
            .map(|entry| entry.profiles.clone())
            .unwrap_or_default()
    }

    pub fn current_app_name(&self) -> Option<String> {
        self.entries
            .get(self.selected_app)
            .map(|entry| entry.name.clone())
    }

    pub fn current_profile_name(&self) -> Option<String> {
        self.entries
            .get(self.selected_app)
            .and_then(|entry| entry.profiles.get(self.selected_profile))
            .cloned()
    }

    pub fn activate_profile(&mut self) -> io::Result<()> {
        if self.focus != Focus::Profiles {
            return Ok(());
        }
        let Some(entry) = self.entries.get(self.selected_app) else {
            return Ok(());
        };
        let Some(profile) = entry.profiles.get(self.selected_profile) else {
            return Ok(());
        };
        let result = set_active_profile(&entry.name, profile);
        match result {
            Ok(()) => {
                self.status = format!("Active profile for {} -> {}", entry.name, profile);
                if let Ok(state) = load_state() {
                    self.update_from_state(state);
                }
            }
            Err(err) => {
                self.status = format!("Failed to set profile: {}", err);
            }
        }
        Ok(())
    }
    fn snap_to_active_profile(&mut self) {
        if let Some(entry) = self.entries.get(self.selected_app) {
            self.selected_profile = 0; // Default
            if let Some(active) = &entry.active_profile {
                if let Some(idx) = entry.profiles.iter().position(|p| p == active) {
                    self.selected_profile = idx;
                }
            }
        }
    }
}

fn next_index(current: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    let next = current as isize + delta;
    if next < 0 {
        return len - 1;
    }
    if next as usize >= len {
        return 0;
    }
    next as usize
}
