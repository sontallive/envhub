use std::io;
use std::time::{Duration, Instant};

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph};
use ratatui::Terminal;

use envhub_core::{load_state, set_active_profile, CoreError, State};

fn main() -> Result<(), CoreError> {
    run_tui().map_err(|err| CoreError::new(envhub_core::ErrorCode::Io, err.to_string()))
}

fn run_tui() -> io::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::load()?;
    let mut last_tick = Instant::now();

    loop {
        terminal.draw(|frame| app.render(frame.area(), frame))?;

        let timeout = Duration::from_millis(200);
        let waited = timeout.saturating_sub(last_tick.elapsed());
        if event::poll(waited)? {
            if let Event::Key(key) = event::read()? {
                if app.handle_key(key)? {
                    break;
                }
            }
        }
        if last_tick.elapsed() >= timeout {
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), crossterm::terminal::LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Focus {
    Apps,
    Profiles,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    Normal,
    AddApp,
    AddProfile,
    SetEnv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InputStep {
    First,
    Second,
}

#[derive(Debug, Clone)]
struct InputState {
    mode: InputMode,
    step: InputStep,
    buf: String,
    first: String,
    second: String,
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
struct AppEntry {
    name: String,
    active_profile: Option<String>,
    profiles: Vec<String>,
}

#[derive(Debug)]
struct App {
    entries: Vec<AppEntry>,
    selected_app: usize,
    selected_profile: usize,
    focus: Focus,
    status: String,
    input: InputState,
    state: State,
}

impl App {
    fn load() -> io::Result<Self> {
        let state = load_state()
            .map_err(|err| io::Error::new(io::ErrorKind::Other, err.to_string()))?;
        Ok(Self::from_state(&state))
    }

    fn from_state(state: &State) -> Self {
        let mut entries = Vec::new();
        for (name, app) in &state.apps {
            let profiles = app.profiles.keys().cloned().collect();
            entries.push(AppEntry {
                name: name.clone(),
                active_profile: app.active_profile.clone(),
                profiles,
            });
        }
        Self {
            entries,
            selected_app: 0,
            selected_profile: 0,
            focus: Focus::Apps,
            status: "Ready".to_string(),
            input: InputState::new(),
            state: state.clone(),
        }
    }

    fn update_from_state(&mut self, state: State) {
        let mut entries = Vec::new();
        for (name, app) in &state.apps {
            let profiles = app.profiles.keys().cloned().collect();
            entries.push(AppEntry {
                name: name.clone(),
                active_profile: app.active_profile.clone(),
                profiles,
            });
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
    }

    fn handle_key(&mut self, key: KeyEvent) -> io::Result<bool> {
        if self.entries.is_empty() && self.input.mode == InputMode::Normal {
            if matches!(key.code, KeyCode::Char('a') | KeyCode::Char('q')) {
                // fall through to normal handling
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
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                return Ok(true)
            }
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
                self.selected_profile = 0;
            }
            Focus::Profiles => {
                let len = self.current_profiles().len();
                self.selected_profile = next_index(self.selected_profile, len, delta);
            }
        }
    }

    fn current_profiles(&self) -> Vec<String> {
        self.entries
            .get(self.selected_app)
            .map(|entry| entry.profiles.clone())
            .unwrap_or_default()
    }

    fn current_app_name(&self) -> Option<String> {
        self.entries.get(self.selected_app).map(|entry| entry.name.clone())
    }

    fn current_profile_name(&self) -> Option<String> {
        self.entries
            .get(self.selected_app)
            .and_then(|entry| entry.profiles.get(self.selected_profile))
            .cloned()
    }

    fn activate_profile(&mut self) -> io::Result<()> {
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

    fn render(&self, area: Rect, frame: &mut ratatui::Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(3), Constraint::Length(2)].as_ref())
            .split(area);
        let header = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(2), Constraint::Min(2)].as_ref())
            .split(chunks[0]);
        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)].as_ref())
            .split(header[1]);

        self.render_header(header[0], frame);
        if self.entries.is_empty() {
            self.render_empty(body[0], body[1], frame);
        } else {
            self.render_apps(body[0], frame);
            self.render_profiles(body[1], frame);
        }

        let status = Paragraph::new(self.status.clone())
            .block(Block::default().borders(Borders::TOP));
        frame.render_widget(status, chunks[1]);

        if self.input.mode != InputMode::Normal {
            self.render_input_modal(area, frame);
        }
    }

    fn render_apps(&self, area: Rect, frame: &mut ratatui::Frame) {
        let focused = self.focus == Focus::Apps;
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let title_style = if focused {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let items: Vec<ListItem> = self
            .entries
            .iter()
            .map(|entry| {
                let active = entry
                    .active_profile
                    .as_ref()
                    .map(|p| format!(" (active: {p})"))
                    .unwrap_or_default();
                ListItem::new(Line::from(vec![
                    Span::raw(&entry.name),
                    Span::styled(active, Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect();

        let title = match self.focus {
            Focus::Apps => "[Apps]  a:add  tab:focus",
            Focus::Profiles => "Apps  a:add",
        };
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title(title)
                    .title_style(title_style),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ");

        frame.render_stateful_widget(list, area, &mut list_state(self.selected_app));
    }

    fn render_profiles(&self, area: Rect, frame: &mut ratatui::Frame) {
        let focused = self.focus == Focus::Profiles;
        let border_style = if focused {
            Style::default().fg(Color::Cyan)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let title_style = if focused {
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        let active = self
            .entries
            .get(self.selected_app)
            .and_then(|entry| entry.active_profile.as_ref());
        let profiles = self.current_profiles();
        let items: Vec<ListItem> = profiles
            .iter()
            .map(|profile| {
                let style = if Some(profile) == active {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(vec![Span::styled(profile.as_str(), style)]))
            })
            .collect();

        let title = match self.focus {
            Focus::Profiles => "[Profiles]  enter:activate  p:add",
            Focus::Apps => "Profiles  p:add",
        };
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(border_style)
                    .title(title)
                    .title_style(title_style),
            )
            .highlight_style(Style::default().add_modifier(Modifier::BOLD))
            .highlight_symbol(">> ");

        let parts = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)].as_ref())
            .split(area);

        frame.render_stateful_widget(list, parts[0], &mut list_state(self.selected_profile));

        let env_lines = self.render_env_preview();
        let env_block = Paragraph::new(env_lines)
            .block(Block::default().borders(Borders::ALL).title("Env  e:set"));
        frame.render_widget(env_block, parts[1]);
    }

    fn render_env_preview(&self) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let Some(app) = self.entries.get(self.selected_app) else {
            lines.push(Line::from("No app selected"));
            return lines;
        };
        let Some(profile) = app.profiles.get(self.selected_profile) else {
            lines.push(Line::from("No profile selected"));
            return lines;
        };
        let Some(app_cfg) = self.state.apps.get(&app.name) else {
            lines.push(Line::from("App not found"));
            return lines;
        };
        let Some(env) = app_cfg.profiles.get(profile) else {
            lines.push(Line::from("Profile not found"));
            return lines;
        };
        if env.is_empty() {
            lines.push(Line::from("No env vars"));
            return lines;
        }
        for (key, value) in env.iter() {
            lines.push(Line::from(vec![
                Span::styled(key.clone(), Style::default().fg(Color::Cyan)),
                Span::raw(" = "),
                Span::raw(value.clone()),
            ]));
        }
        lines
    }

    fn render_header(&self, area: Rect, frame: &mut ratatui::Frame) {
        let title = Line::from(vec![
            Span::styled("EnvHub", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw("  "),
            Span::styled("TUI", Style::default().fg(Color::DarkGray)),
        ]);
        let hint = Line::from(Span::styled(
            "q:quit  r:reload  tab:focus",
            Style::default().fg(Color::DarkGray),
        ));
        let block = Block::default().borders(Borders::BOTTOM);
        let header = Paragraph::new(vec![title, hint]).block(block);
        frame.render_widget(header, area);
    }

    fn render_empty(&self, left: Rect, right: Rect, frame: &mut ratatui::Frame) {
        let left_block = Block::default().borders(Borders::ALL).title("Apps");
        frame.render_widget(left_block, left);
        let text = vec![
            Line::from("No apps registered"),
            Line::from("Press 'a' to add one"),
        ];
        let right_block = Paragraph::new(text).block(Block::default().borders(Borders::ALL).title("Profiles"));
        frame.render_widget(right_block, right);
    }

    fn render_input_modal(&self, area: Rect, frame: &mut ratatui::Frame) {
        let modal = centered_rect(60, 18, area);
        let title = match self.input.mode {
            InputMode::AddApp => "Add App",
            InputMode::AddProfile => "Add Profile",
            InputMode::SetEnv => "Set Env",
            InputMode::Normal => "",
        };
        let hint = match (self.input.mode, self.input.step) {
            (InputMode::AddApp, InputStep::First) => "App name",
            (InputMode::AddApp, InputStep::Second) => "Target binary",
            (InputMode::AddProfile, _) => "Profile name",
            (InputMode::SetEnv, InputStep::First) => "Env key",
            (InputMode::SetEnv, InputStep::Second) => "Env value",
            _ => "",
        };
        frame.render_widget(Clear, modal);
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black));
        let text = vec![
            Line::from(vec![
                Span::styled(hint, Style::default().fg(Color::Yellow)),
                Span::raw(": "),
                Span::raw(&self.input.buf),
            ]),
            Line::from(""),
            Line::from(Span::styled("Enter to confirm, Esc to cancel", Style::default().fg(Color::DarkGray))),
        ];
        let paragraph = Paragraph::new(text).block(block);
        frame.render_widget(paragraph, modal);
    }

}

fn list_state(selected: usize) -> ratatui::widgets::ListState {
    let mut state = ratatui::widgets::ListState::default();
    state.select(Some(selected));
    state
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

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(area);
    let vertical = popup_layout[1];
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(vertical);
    horizontal[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use envhub_core::{AppConfig, EnvProfile};
    use indexmap::IndexMap;

    #[test]
    fn next_index_wraps() {
        assert_eq!(next_index(0, 3, -1), 2);
        assert_eq!(next_index(2, 3, 1), 0);
        assert_eq!(next_index(1, 3, 1), 2);
    }

    #[test]
    fn from_state_maps_profiles() {
        let mut state = State::default();
        let mut profiles = IndexMap::new();
        let mut env = EnvProfile::new();
        env.insert("KEY".to_string(), "VALUE".to_string());
        profiles.insert("work".to_string(), env);
        state.apps.insert(
            "tool".to_string(),
            AppConfig {
                target_binary: "tool-bin".to_string(),
                active_profile: Some("work".to_string()),
                profiles,
                ..AppConfig::default()
            },
        );

        let app = App::from_state(&state);
        assert_eq!(app.entries.len(), 1);
        assert_eq!(app.entries[0].name, "tool");
        assert_eq!(app.entries[0].profiles, vec!["work".to_string()]);
    }
}
