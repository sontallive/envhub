use envhub_core::{
    InstallMode, State, get_launcher_path, install_shim, is_shim_installed, load_state,
    set_active_profile,
};
use std::io;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Apps,
    Profiles,
    EnvVars,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    AppsList,
    AppDetail,
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
    pub selection_index: usize,
}

impl InputState {
    fn new() -> Self {
        Self {
            mode: InputMode::Normal,
            step: InputStep::First,
            buf: String::new(),
            first: String::new(),
            second: String::new(),
            selection_index: 0,
        }
    }

    fn reset(&mut self) {
        self.mode = InputMode::Normal;
        self.step = InputStep::First;
        self.buf.clear();
        self.first.clear();
        self.second.clear();
        self.selection_index = 0;
    }
}

#[derive(Debug, Clone)]
pub struct AppEntry {
    pub name: String,
    pub active_profile: Option<String>,
    pub profiles: Vec<String>,
    pub is_installed: bool,
}

#[derive(Debug)]
pub struct App {
    pub entries: Vec<AppEntry>,
    pub selected_app: usize,
    pub selected_profile: usize,
    pub selected_env_var: usize,
    pub page: Page,
    pub focus: Focus,
    pub status: String,
    pub input: InputState,
    pub state: State,
    pub is_launcher_installed: bool,
    pub is_path_configured: bool,
}

impl App {
    pub fn load() -> io::Result<Self> {
        let state =
            load_state().map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;
        Ok(Self::from_state(&state))
    }

    pub fn handle_install(&mut self) {
        if let Some(app_name) = self.current_app_name() {
            if let Some(launcher_path) = get_launcher_path() {
                match install_shim(&app_name, InstallMode::User, &launcher_path) {
                    Ok(_) => {
                        self.status = format!("Installed shim for {}", app_name);
                        // Update status
                        if let Ok(state) = load_state() {
                            self.update_from_state(state);
                        }
                    }
                    Err(e) => {
                        self.status = format!("Installation failed: {}", e);
                    }
                }
            } else {
                self.status = "Launcher not found!".to_string();
            }
        }
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
                    is_installed: is_shim_installed(name, InstallMode::User),
                });
            }
        }

        let mut app = Self {
            entries,
            selected_app: 0,
            selected_profile: 0,
            selected_env_var: 0,
            page: Page::AppsList,
            focus: Focus::Apps,
            status: "Ready".to_string(),
            input: InputState::new(),
            state: state.clone(),
            is_launcher_installed: envhub_core::is_launcher_installed(),
            is_path_configured: envhub_core::is_user_path_configured(),
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
                    is_installed: is_shim_installed(name, InstallMode::User),
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
        let env_len = self.current_env_list().len();
        if self.selected_env_var >= env_len {
            self.selected_env_var = env_len.saturating_sub(1);
        }
        self.snap_to_active_profile();
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> io::Result<bool> {
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
            KeyCode::Esc | KeyCode::Backspace => {
                if self.page == Page::AppDetail {
                    self.page = Page::AppsList;
                    self.focus = Focus::Apps;
                    self.status = "Apps List".to_string();
                }
            }
            KeyCode::Char('a') => {
                if self.page == Page::AppsList {
                    self.input.mode = InputMode::AddApp;
                    self.input.step = InputStep::First;
                    self.input.buf.clear();
                    self.status = "Add app: enter name".to_string();
                } else if self.page == Page::AppDetail && self.focus == Focus::EnvVars {
                    self.input.mode = InputMode::SetEnv;
                    self.input.step = InputStep::First;
                    self.input.buf.clear();
                    let profile = self.current_profile_name().unwrap_or_default();
                    self.status = format!("Add env for {profile}: enter key");
                }
            }
            KeyCode::Char('i') => {
                if self.page == Page::AppsList {
                    self.handle_install();
                }
            }
            KeyCode::Char('p') => {
                if self.page == Page::AppDetail {
                    self.input.mode = InputMode::AddProfile;
                    self.input.step = InputStep::First;
                    self.input.buf.clear();
                    self.status = "Add profile: enter name".to_string();
                }
            }
            KeyCode::Char('d') => {
                if self.focus == Focus::EnvVars {
                    // Delete current env var
                    if let Some((key, _)) = self.current_env_pair() {
                        if let (Some(app), Some(profile)) =
                            (self.current_app_name(), self.current_profile_name())
                        {
                            match envhub_core::remove_profile_env(&app, &profile, &key) {
                                Ok(()) => {
                                    self.status = format!("Removed {key}");
                                    if let Ok(state) = load_state() {
                                        self.update_from_state(state);
                                    }
                                }
                                Err(e) => self.status = format!("Failed to remove: {e}"),
                            }
                        }
                    }
                }
            }
            KeyCode::Char('e') => {
                if self.page == Page::AppDetail {
                    self.input.mode = InputMode::SetEnv;
                    self.input.step = InputStep::First;
                    self.input.buf.clear();

                    // Pre-fill key if editing
                    if self.focus == Focus::EnvVars {
                        if let Some((key, value)) = self.current_env_pair() {
                            self.input.first = key.clone();
                            self.input.buf = value.clone(); // Pre-fill value? Logic below expects buf to be KEY first.
                            // Actually SetEnv flow is: Step 1 Enter Key, Step 2 Enter Value.
                            // If we want to edit, we probably want to pre-fill Key in Step 1.
                            self.input.buf = key;
                            self.status = "Edit env: confirm key".to_string();
                        }
                    } else {
                        let profile = self.current_profile_name().unwrap_or_default();
                        self.status = format!("Set env for profile {profile}: enter key");
                    }
                }
            }
            // Tab is less useful now with pages, but maybe switch focus between Profiles and EnvVars later?
            // For now, removing Tab switching or keeping it no-op if on AppsList
            KeyCode::Tab => {
                if self.page == Page::AppDetail {
                    self.focus = match self.focus {
                        Focus::Profiles => Focus::EnvVars,
                        Focus::EnvVars => Focus::Profiles,
                        _ => Focus::Profiles,
                    };
                }
            }
            KeyCode::Up => self.move_selection(-1),
            KeyCode::Down => self.move_selection(1),
            KeyCode::Enter => {
                if self.page == Page::AppsList {
                    if !self.entries.is_empty() {
                        self.page = Page::AppDetail;
                        self.focus = Focus::Profiles;
                        self.status =
                            format!("Selected {}", self.current_app_name().unwrap_or_default());
                    }
                } else if self.page == Page::AppDetail {
                    if self.focus == Focus::Profiles {
                        self.activate_profile()?;
                    } else if self.focus == Focus::EnvVars {
                        // Edit on Enter? Same as 'e' but maybe pre-set?
                        // Let's reuse the 'e' logic or just trigger SetEnv
                        if let Some((key, val)) = self.current_env_pair() {
                            self.input.mode = InputMode::SetEnv;
                            self.input.step = InputStep::First;
                            self.input.buf = key;
                            self.status = format!("Edit {}: confirm key", val);
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(false)
    }

    fn handle_input(&mut self, key: KeyEvent) -> io::Result<bool> {
        // Special handling for Clone Profile Selection (Step 2 of AddProfile)
        if self.input.mode == InputMode::AddProfile && self.input.step == InputStep::Second {
            match key.code {
                KeyCode::Esc => {
                    self.input.reset();
                    self.status = "Cancelled".to_string();
                }
                KeyCode::Up => {
                    if self.input.selection_index > 0 {
                        self.input.selection_index -= 1;
                    }
                }
                KeyCode::Down => {
                    // limit depends on how many profiles + 1 (None)
                    // We need correct count.
                    let count = self.current_profiles().len() + 1;
                    if self.input.selection_index < count - 1 {
                        self.input.selection_index += 1;
                    }
                }
                KeyCode::Enter => {
                    self.commit_input()?;
                }
                _ => {}
            }
            return Ok(false);
        }

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
                // Step 2: Select Clone source
                // Validate if profile exists
                let new_profile = value.clone();
                let profiles = self.current_profiles();
                if profiles.contains(&new_profile) {
                    self.status = format!("Profile '{}' already exists", new_profile);
                    return Ok(());
                }

                self.input.first = value;
                self.input.buf.clear();
                self.input.step = InputStep::Second;
                self.input.selection_index = 0;
                self.status = "Select profile to copy from".to_string();
            }
            (InputMode::AddProfile, InputStep::Second) => {
                let app = self.current_app_name();
                let new_profile = self.input.first.clone();

                // Determine selected source
                let profiles = self.current_profiles();
                // Index 0 is "None", Index 1..=len as profiles[i-1]
                let source_profile = if self.input.selection_index == 0 {
                    None
                } else {
                    profiles.get(self.input.selection_index - 1).cloned()
                };

                if let Some(app) = app {
                    let res = match source_profile {
                        Some(src) => envhub_core::clone_profile(&app, &src, &new_profile),
                        None => envhub_core::add_profile(&app, &new_profile),
                    };

                    match res {
                        Ok(()) => {
                            self.status = format!("profile {new_profile} added to {app}");
                            if let Ok(state) = load_state() {
                                self.update_from_state(state);
                            }
                        }
                        Err(err) => self.status = format!("Failed: {err}"),
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
        match self.page {
            Page::AppsList => {
                let len = self.entries.len();
                self.selected_app = next_index(self.selected_app, len, delta);
                self.snap_to_active_profile();
            }
            Page::AppDetail => {
                match self.focus {
                    Focus::Profiles => {
                        let len = self.current_profiles().len();
                        self.selected_profile = next_index(self.selected_profile, len, delta);
                        // When changing profile, maybe reset selected env var?
                        self.selected_env_var = 0;
                    }
                    Focus::EnvVars => {
                        let len = self.current_env_list().len();
                        self.selected_env_var = next_index(self.selected_env_var, len, delta);
                    }
                    _ => {}
                }
            }
        }
    }

    pub fn current_env_list(&self) -> Vec<(String, String)> {
        let Some(app_name) = self.current_app_name() else {
            return vec![];
        };
        let Some(profile) = self.current_profile_name() else {
            return vec![];
        };

        self.state
            .apps
            .get(&app_name)
            .and_then(|a| a.profiles.get(&profile))
            .map(|m| m.iter().map(|(k, v)| (k.clone(), v.clone())).collect())
            .unwrap_or_default()
    }

    pub fn current_env_pair(&self) -> Option<(String, String)> {
        self.current_env_list().get(self.selected_env_var).cloned()
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
