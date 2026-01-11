use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Row, Table, Wrap,
    },
};

use crate::app::{App, Focus, InputMode, InputStep};

// Theme configuration
struct Theme {
    primary: Color,
    secondary: Color,
    accent: Color,
    background: Color,
    text: Color,
    text_dim: Color,
    success: Color,
    error: Color,
}

const THEME: Theme = Theme {
    primary: Color::Cyan,      // Cyan
    secondary: Color::Magenta, // Pink/Purple
    accent: Color::Yellow,
    background: Color::Reset, // Transparent/Term Default
    text: Color::White,
    text_dim: Color::DarkGray,
    success: Color::Green,
    error: Color::Red,
};

pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Set global background
    frame.render_widget(
        Block::default().style(Style::default().bg(THEME.background)),
        area,
    );

    // Main Layout: Header, Content, Footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(0),    // Content
            Constraint::Length(1), // Footer/Status
        ])
        .split(area);

    render_header(frame, chunks[0]);
    render_content(frame, chunks[1], app);
    render_status_bar(frame, chunks[2], app);

    if app.input.mode != InputMode::Normal {
        render_input_modal(frame, area, app);
    }
}

fn render_header(frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .borders(Borders::BOTTOM)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(THEME.text_dim));

    let inner_area = block.inner(area);

    let title_text = vec![
        Span::styled(
            " Env",
            Style::default()
                .fg(THEME.primary)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "Hub",
            Style::default().fg(THEME.text).add_modifier(Modifier::BOLD),
        ),
    ];

    let instructions = Line::from(vec![
        Span::styled(
            "Q",
            Style::default()
                .fg(THEME.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("uit | "),
        Span::styled(
            "R",
            Style::default()
                .fg(THEME.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("eload | "),
        Span::styled(
            "Tab",
            Style::default()
                .fg(THEME.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Switch Focus"),
    ]);

    frame.render_widget(block, area);

    // Left aligned Title
    frame.render_widget(
        Paragraph::new(Line::from(title_text)).alignment(Alignment::Left),
        inner_area,
    );

    // Right aligned Global Hints
    frame.render_widget(
        Paragraph::new(instructions).alignment(Alignment::Right),
        inner_area,
    );
}

fn render_content(frame: &mut Frame, area: Rect, app: &App) {
    // 3-Column Layout: Apps | Profiles | Env Vars
    // Or 2-Column: Apps+Profiles | Env Vars?
    // Let's stick to 2 major columns, but split the left one for Apps/Profiles list.
    // Left: 30% width (Apps List top, Profiles List bottom? Or Side-by-Side?)
    // The previous layout was Apps | Profiles.
    // Let's try:
    // Left Panel (40%): Apps List (top 50%), Profiles List (bottom 50%)
    // Right Panel (60%): Environment Variables Table

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[0]);

    render_apps_list(frame, left_chunks[0], app);
    render_profiles_list(frame, left_chunks[1], app);
    render_env_details(frame, main_chunks[1], app);
}

fn draw_block(title: &str, is_focused: bool) -> Block<'_> {
    let border_color = if is_focused {
        THEME.primary
    } else {
        THEME.text_dim
    };
    let text_color = if is_focused {
        THEME.primary
    } else {
        THEME.text_dim
    };

    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(text_color).add_modifier(Modifier::BOLD),
        ))
}

fn render_apps_list(frame: &mut Frame, area: Rect, app: &App) {
    let focus = app.focus == Focus::Apps;
    let items: Vec<ListItem> = app
        .entries
        .iter()
        .map(|entry| {
            let is_active_somewhere = entry.active_profile.is_some();
            let marker = if is_active_somewhere { "● " } else { "  " };
            let marker_style = if is_active_somewhere {
                Style::default().fg(THEME.success)
            } else {
                Style::default()
            };

            let subtext = if let Some(p) = &entry.active_profile {
                format!(" ({})", p)
            } else {
                String::new()
            };

            let content = Line::from(vec![
                Span::styled(marker, marker_style),
                Span::raw(&entry.name),
                Span::styled(subtext, Style::default().fg(THEME.text_dim)),
            ]);

            ListItem::new(content)
        })
        .collect();

    let list = List::new(items)
        .block(draw_block("Applications (A:Add)", focus))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▎ ");

    let mut state = ListState::default();
    state.select(Some(app.selected_app));
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_profiles_list(frame: &mut Frame, area: Rect, app: &App) {
    let focus = app.focus == Focus::Profiles;
    let current_app = app.entries.get(app.selected_app);
    let active_profile = current_app.and_then(|a| a.active_profile.as_ref());

    let items: Vec<ListItem> = app
        .current_profiles()
        .into_iter()
        .map(|profile| {
            let is_active = Some(&profile) == active_profile;
            let icon = if is_active { "✓ " } else { "  " };
            let style = if is_active {
                Style::default().fg(THEME.success)
            } else {
                Style::default()
            };

            ListItem::new(Line::from(vec![
                Span::styled(icon, style),
                Span::raw(profile),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(draw_block("Profiles (P:Add, Enter:Activate)", focus))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▎ ");

    let mut state = ListState::default();
    state.select(Some(app.selected_profile));
    frame.render_stateful_widget(list, area, &mut state);
}

fn render_env_details(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(THEME.text_dim))
        .title(" Environment Variables (E:Set) ");

    let (rows, empty_msg) = get_env_rows(app);

    if let Some(msg) = empty_msg {
        let p = Paragraph::new(msg)
            .block(block)
            .style(Style::default().fg(THEME.text_dim))
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true });
        frame.render_widget(p, area);
    } else {
        let table = Table::new(
            rows,
            [Constraint::Percentage(30), Constraint::Percentage(70)],
        )
        .header(
            Row::new(vec!["Key", "Value"]).style(
                Style::default()
                    .fg(THEME.primary)
                    .add_modifier(Modifier::BOLD)
                    .add_modifier(Modifier::UNDERLINED),
            ),
        )
        .block(block)
        .column_spacing(2);
        frame.render_widget(table, area);
    }
}

fn get_env_rows(app: &App) -> (Vec<Row<'_>>, Option<String>) {
    let Some(app_entry) = app.entries.get(app.selected_app) else {
        return (vec![], Some("No application selected".to_string()));
    };
    let Some(profile) = app_entry.profiles.get(app.selected_profile) else {
        return (vec![], Some("No profile selected".to_string()));
    };
    let Some(app_cfg) = app.state.apps.get(&app_entry.name) else {
        return (vec![], Some("App configuration not found".to_string()));
    };
    let Some(env) = app_cfg.profiles.get(profile) else {
        return (vec![], Some("Profile configuration not found".to_string()));
    };

    if env.is_empty() {
        return (vec![], Some("No environment variables set".to_string()));
    }

    let rows = env
        .iter()
        .map(|(k, v)| {
            Row::new(vec![
                Span::styled(k.clone(), Style::default().fg(THEME.secondary)),
                Span::raw(v.clone()),
            ])
        })
        .collect();

    (rows, None)
}

fn render_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let status_style = if app.status.starts_with("Failed") {
        Style::default().fg(THEME.error)
    } else {
        Style::default().fg(THEME.text)
    };

    let text = Span::styled(format!(" {}", app.status), status_style);

    // We can't set bg easily on exact line without a block covering full width,
    // so let's just use a Paragraph with a styled block or style.

    frame.render_widget(
        Paragraph::new(text).style(Style::default().bg(Color::DarkGray)),
        area,
    );
}

fn render_input_modal(frame: &mut Frame, area: Rect, app: &App) {
    let modal_area = centered_rect(60, 25, area);

    frame.render_widget(Clear, modal_area); // Clear background

    let title = match app.input.mode {
        InputMode::AddApp => " Add Application ",
        InputMode::AddProfile => " Add Profile ",
        InputMode::SetEnv => " Set Environment Variable ",
        InputMode::Normal => "",
    };

    let block = Block::default()
        .title(Span::styled(
            title,
            Style::default()
                .fg(THEME.accent)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Thick)
        .border_style(Style::default().fg(THEME.accent));

    let inner_area = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    // Layout inside modal
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(2), Constraint::Length(3)]) // Prompt+Input, Hints
        .split(inner_area);

    let prompt = match (app.input.mode, app.input.step) {
        (InputMode::AddApp, InputStep::First) => "Enter application name:",
        (InputMode::AddApp, InputStep::Second) => "Enter target binary path:",
        (InputMode::AddProfile, _) => "Enter new profile name:",
        (InputMode::SetEnv, InputStep::First) => "Enter variable KEY:",
        (InputMode::SetEnv, InputStep::Second) => "Enter variable VALUE:",
        _ => "",
    };

    let input_text = vec![
        Line::from(Span::styled(prompt, Style::default().fg(THEME.text_dim))),
        Line::from(vec![
            Span::raw(" > "),
            Span::styled(
                &app.input.buf,
                Style::default().fg(THEME.text).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                "_",
                Style::default()
                    .fg(THEME.accent)
                    .add_modifier(Modifier::SLOW_BLINK),
            ), // Cursor
        ]),
    ];

    frame.render_widget(
        Paragraph::new(input_text).wrap(Wrap { trim: false }),
        layout[0],
    );

    let hints = Line::from(vec![
        Span::styled("Enter", Style::default().fg(THEME.primary)),
        Span::raw(" Confirm  "),
        Span::styled("Esc", Style::default().fg(THEME.error)),
        Span::raw(" Cancel"),
    ]);

    frame.render_widget(
        Paragraph::new(hints).alignment(Alignment::Center),
        layout[1],
    );
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    let vertical = popup_layout[1];
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical);

    horizontal[1]
}
