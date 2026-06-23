use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Gauge, List, ListItem, Paragraph, Wrap},
    Frame,
};

use crate::app::{ActiveModal, App, MenuItem, PaneFocus, RunnerState};
use crate::game::GameStatus;

// Gruvbox / Everforest Inspired Palette
const COLOR_BG: Color = Color::Rgb(29, 32, 33); // Dark0
const COLOR_BORDER_ACTIVE: Color = Color::Rgb(250, 189, 47); // Yellow/Orange
const COLOR_BORDER_INACTIVE: Color = Color::Rgb(102, 92, 84); // Gray
const COLOR_TEXT: Color = Color::Rgb(213, 196, 161); // Light4
const COLOR_HIGHLIGHT_BG: Color = Color::Rgb(167, 192, 128); // Green highlight
const COLOR_HIGHLIGHT_FG: Color = Color::Rgb(40, 44, 52); // Dark highlight text
const COLOR_SUCCESS: Color = Color::Rgb(184, 187, 38); // Green
const COLOR_INFO: Color = Color::Rgb(131, 165, 152); // Blue/Teal
const COLOR_WARN: Color = Color::Rgb(254, 128, 25); // Orange
const COLOR_MUTED: Color = Color::Rgb(146, 131, 116); // Gray muted

fn is_right_focused(app: &App) -> bool {
    (app.focus == PaneFocus::LibraryList || app.focus == PaneFocus::MetadataPanel)
        && app.active_modal.is_none()
}

pub fn render(f: &mut Frame, app: &mut App) {
    // Background style
    let main_block = Block::default().style(Style::default().bg(COLOR_BG).fg(COLOR_TEXT));
    f.render_widget(main_block, f.area());

    // Main layout: Body + Footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(f.area());

    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(chunks[0]);

    render_left_pane(f, app, body_chunks[0]);
    render_right_pane(f, app, body_chunks[1]);
    render_footer(f, app, chunks[1]);

    // Draw centered popup input modal if active
    if let Some(ref modal) = app.active_modal {
        match modal {
            ActiveModal::CreatePrefix(ref prefix_modal) => {
                render_new_prefix_modal(f, prefix_modal, f.area());
            }
            ActiveModal::AddGame(ref add_game_modal) => {
                render_add_game_modal(f, app, add_game_modal, f.area());
            }
            ActiveModal::DeleteGame {
                game_idx,
                prompt_prefix,
                ..
            } => {
                if let Some(game) = app.config.games.get(*game_idx) {
                    render_delete_game_modal(
                        f,
                        &game.name,
                        &game.wineprefix,
                        *prompt_prefix,
                        f.area(),
                    );
                }
            }
            ActiveModal::DeletePrefix {
                prefix_idx,
                prompt_delete_files,
                ..
            } => {
                if let Some(prefix) = app.custom_prefixes.get(*prefix_idx) {
                    render_delete_prefix_modal(
                        f,
                        &prefix.name,
                        &prefix.path,
                        *prompt_delete_files,
                        f.area(),
                    );
                }
            }
            ActiveModal::Help => {
                render_help_modal(f, f.area());
            }
        }
    }

    // Render onboarding welcome overlay on top of everything if active
    if let Some(ref onboarding) = app.onboarding {
        render_onboarding(f, onboarding, f.area());
    }
}

fn render_left_pane(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.focus == PaneFocus::LeftMenu && app.active_modal.is_none();
    let border_style = Style::default().fg(if is_focused {
        COLOR_BORDER_ACTIVE
    } else {
        COLOR_BORDER_INACTIVE
    });

    let menu_block = Block::default()
        .title(" 󰊖 LGTUI NAVIGATOR ")
        .borders(Borders::ALL)
        .border_style(border_style);

    // Build the list of menu items
    let mut items = Vec::new();

    // Section GAMES
    items.push(ListItem::new(Line::from(vec![Span::styled(
        " GAMES",
        Style::default()
            .fg(COLOR_MUTED)
            .add_modifier(Modifier::BOLD),
    )])));

    // Item: Library
    let lib_style = if app.menu_index == 0 {
        Style::default()
            .bg(COLOR_HIGHLIGHT_BG)
            .fg(COLOR_HIGHLIGHT_FG)
    } else {
        Style::default().fg(COLOR_TEXT)
    };
    items.push(ListItem::new(Line::from(vec![Span::styled(
        "     Library",
        lib_style,
    )])));

    // Spacer
    items.push(ListItem::new(Line::from("")));

    // Section MANAGEMENT
    items.push(ListItem::new(Line::from(vec![Span::styled(
        " MANAGEMENT",
        Style::default()
            .fg(COLOR_MUTED)
            .add_modifier(Modifier::BOLD),
    )])));

    // Item: Runner Manager
    let rm_style = if app.menu_index == 1 {
        Style::default()
            .bg(COLOR_HIGHLIGHT_BG)
            .fg(COLOR_HIGHLIGHT_FG)
    } else {
        Style::default().fg(COLOR_TEXT)
    };
    items.push(ListItem::new(Line::from(vec![Span::styled(
        "   󰘚  Wine/Proton Manager",
        rm_style,
    )])));

    // Item: Prefix Manager
    let pm_style = if app.menu_index == 2 {
        Style::default()
            .bg(COLOR_HIGHLIGHT_BG)
            .fg(COLOR_HIGHLIGHT_FG)
    } else {
        Style::default().fg(COLOR_TEXT)
    };
    items.push(ListItem::new(Line::from(vec![Span::styled(
        "   󰓦  Wine Prefix Manager",
        pm_style,
    )])));

    // Item: Winetricks
    let wt_style = if app.menu_index == 3 {
        Style::default()
            .bg(COLOR_HIGHLIGHT_BG)
            .fg(COLOR_HIGHLIGHT_FG)
    } else {
        Style::default().fg(COLOR_TEXT)
    };
    items.push(ListItem::new(Line::from(vec![Span::styled(
        "     Winetricks Utilities",
        wt_style,
    )])));

    // Item: System Dependencies
    let sd_style = if app.menu_index == 4 {
        Style::default()
            .bg(COLOR_HIGHLIGHT_BG)
            .fg(COLOR_HIGHLIGHT_FG)
    } else {
        Style::default().fg(COLOR_TEXT)
    };
    items.push(ListItem::new(Line::from(vec![Span::styled(
        "     System Dependencies",
        sd_style,
    )])));

    // Item: Settings
    let st_style = if app.menu_index == 5 {
        Style::default()
            .bg(COLOR_HIGHLIGHT_BG)
            .fg(COLOR_HIGHLIGHT_FG)
    } else {
        Style::default().fg(COLOR_TEXT)
    };
    items.push(ListItem::new(Line::from(vec![Span::styled(
        "     Global Settings",
        st_style,
    )])));

    let list = List::new(items).block(menu_block);
    f.render_widget(list, area);
}

fn render_right_pane(f: &mut Frame, app: &mut App, area: Rect) {
    let is_focused = is_right_focused(app);
    let border_style = Style::default().fg(if is_focused {
        COLOR_BORDER_ACTIVE
    } else {
        COLOR_BORDER_INACTIVE
    });

    match app.current_menu() {
        MenuItem::Library => render_library(f, app, area, border_style),
        MenuItem::RunnerManager => render_runner_manager(f, app, area, border_style),
        MenuItem::PrefixManager => render_prefix_manager(f, app, area, border_style),
        MenuItem::Winetricks => render_winetricks(f, app, area, border_style),
        MenuItem::DependencyInstaller => render_dependency_installer(f, app, area, border_style),
        MenuItem::Settings => render_settings(f, app, area, border_style),
    }
}

fn render_library(f: &mut Frame, app: &App, area: Rect, _border_style: Style) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    let is_dimmed = app.active_modal.is_some();

    // Left half: Game List
    let list_border_style =
        Style::default().fg(if app.focus == PaneFocus::LibraryList && !is_dimmed {
            COLOR_BORDER_ACTIVE
        } else {
            COLOR_BORDER_INACTIVE
        });
    let list_block = Block::default()
        .title("  GAME LIBRARY ")
        .borders(Borders::ALL)
        .border_style(list_border_style);

    let items: Vec<ListItem> = app
        .config
        .games
        .iter()
        .enumerate()
        .map(|(idx, game)| {
            let is_selected = app.selected_game_idx == idx;
            let is_focused_selected =
                is_selected && app.focus == PaneFocus::LibraryList && !is_dimmed;

            let (status_char, status_style) = match game.status {
                GameStatus::Ready => (" ", Style::default().fg(COLOR_MUTED)),
                GameStatus::Running { .. } => (" ", Style::default().fg(COLOR_SUCCESS)),
                GameStatus::Error(_) => (" ", Style::default().fg(COLOR_WARN)),
            };

            let name_span = if is_focused_selected {
                Span::styled(
                    &game.name,
                    Style::default()
                        .bg(COLOR_HIGHLIGHT_BG)
                        .fg(COLOR_HIGHLIGHT_FG)
                        .add_modifier(Modifier::BOLD),
                )
            } else if is_selected {
                Span::styled(
                    &game.name,
                    Style::default()
                        .fg(COLOR_BORDER_ACTIVE)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(&game.name, Style::default().fg(COLOR_TEXT))
            };

            ListItem::new(Line::from(vec![
                Span::styled(status_char, status_style),
                name_span,
            ]))
        })
        .collect();

    let list = List::new(items).block(list_block);
    f.render_widget(list, layout[0]);

    // Right half: Game Details
    let details_border_style =
        Style::default().fg(if app.focus == PaneFocus::MetadataPanel && !is_dimmed {
            COLOR_BORDER_ACTIVE
        } else {
            COLOR_BORDER_INACTIVE
        });
    let details_block = Block::default()
        .title(" 󰏖 METADATA & CONFIG ")
        .borders(Borders::ALL)
        .border_style(details_border_style);

    if app.config.games.is_empty() {
        let empty_msg = Paragraph::new("No games found in the library.")
            .block(details_block)
            .style(Style::default().fg(COLOR_MUTED));
        f.render_widget(empty_msg, layout[1]);
        return;
    }

    let game = &app.config.games[app.selected_game_idx];

    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                "Title: ",
                Style::default().fg(COLOR_INFO).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &game.name,
                Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(COLOR_INFO)),
            match &game.status {
                GameStatus::Ready => {
                    Span::styled("Ready to Play", Style::default().fg(COLOR_SUCCESS))
                }
                GameStatus::Running { pid, .. } => Span::styled(
                    format!("Running (PID: {})", pid),
                    Style::default().fg(COLOR_BORDER_ACTIVE),
                ),
                GameStatus::Error(err) => {
                    Span::styled(format!("Error: {}", err), Style::default().fg(COLOR_WARN))
                }
            },
        ]),
        Line::from(vec![
            Span::styled("Runner: ", Style::default().fg(COLOR_INFO)),
            Span::styled(
                game.runner_id.as_deref().unwrap_or("System default (Wine)"),
                Style::default().fg(COLOR_TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("Playtime: ", Style::default().fg(COLOR_INFO)),
            Span::styled(
                format_playtime(game.playtime_secs),
                Style::default().fg(COLOR_TEXT),
            ),
        ]),
        Line::from(vec![
            Span::styled("Wine Prefix: ", Style::default().fg(COLOR_INFO)),
            Span::styled(
                game.wineprefix
                    .as_deref()
                    .unwrap_or("Default shared prefix"),
                Style::default().fg(COLOR_MUTED),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "ENVIRONMENT WRAPPERS & FLAGS",
            Style::default()
                .fg(COLOR_MUTED)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ];

    // Toggle grid UI representation
    let toggle_symbol = |enabled: bool| {
        if enabled {
            Span::styled(
                " [●] ",
                Style::default()
                    .fg(COLOR_SUCCESS)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled(" [○] ", Style::default().fg(COLOR_MUTED))
        }
    };

    let render_option_line = |idx: usize, enabled: bool, label: &str| {
        let is_selected =
            app.focus == PaneFocus::MetadataPanel && app.selected_metadata_idx == idx && !is_dimmed;
        let mut spans = Vec::new();
        spans.push(toggle_symbol(enabled));

        let label_style = if is_selected {
            Style::default()
                .bg(COLOR_HIGHLIGHT_BG)
                .fg(COLOR_HIGHLIGHT_FG)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(COLOR_TEXT)
        };
        spans.push(Span::styled(label.to_string(), label_style));
        Line::from(spans)
    };

    lines.push(render_option_line(
        0,
        game.dxvk,
        "DXVK (Direct3D 9/10/11 -> Vulkan)",
    ));
    lines.push(render_option_line(
        1,
        game.vkd3d,
        "VKD3D (Direct3D 12 -> Vulkan)",
    ));
    lines.push(render_option_line(
        2,
        game.mangohud,
        "MangoHud (Performance Overlay)",
    ));
    lines.push(render_option_line(
        3,
        game.gamemode,
        "Feral GameMode (CPU/GPU Optimizations)",
    ));

    // Fetch and display game statistics from SQLite database
    let stats = if let Ok(conn) = crate::database::init_db() {
        crate::database::get_game_stats(&conn, &game.id).unwrap_or_default()
    } else {
        crate::database::GameStats::default()
    };

    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        "GAME STATISTICS & INSIGHTS",
        Style::default()
            .fg(COLOR_MUTED)
            .add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(""));

    let last_played_str = if let Some(timestamp) = stats.last_played {
        format_timestamp(timestamp)
    } else {
        "Never".to_string()
    };

    lines.push(Line::from(vec![
        Span::styled("  Last Played:    ", Style::default().fg(COLOR_INFO)),
        Span::styled(last_played_str, Style::default().fg(COLOR_TEXT)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Total Launches: ", Style::default().fg(COLOR_INFO)),
        Span::styled(
            format!("{} sessions", stats.session_count),
            Style::default().fg(COLOR_TEXT),
        ),
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Average FPS:    ", Style::default().fg(COLOR_INFO)),
        if stats.avg_fps > 0 {
            Span::styled(
                format!("{} FPS", stats.avg_fps),
                Style::default()
                    .fg(COLOR_SUCCESS)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled("N/A", Style::default().fg(COLOR_MUTED))
        },
    ]));
    lines.push(Line::from(vec![
        Span::styled("  Maximum FPS:    ", Style::default().fg(COLOR_INFO)),
        if stats.max_fps > 0 {
            Span::styled(
                format!("{} FPS", stats.max_fps),
                Style::default()
                    .fg(COLOR_SUCCESS)
                    .add_modifier(Modifier::BOLD),
            )
        } else {
            Span::styled("N/A", Style::default().fg(COLOR_MUTED))
        },
    ]));

    let details = Paragraph::new(lines)
        .block(details_block)
        .wrap(Wrap { trim: false });
    f.render_widget(details, layout[1]);
}

fn format_timestamp(secs: u64) -> String {
    let days = secs / 86400;
    let seconds_in_day = secs % 86400;
    let hours = seconds_in_day / 3600;
    let minutes = (seconds_in_day % 3600) / 60;

    let mut year = 1970;
    let mut days_left = days;

    loop {
        let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
        let days_in_year = if is_leap { 366 } else { 365 };
        if days_left < days_in_year {
            break;
        }
        days_left -= days_in_year;
        year += 1;
    }

    let is_leap = (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0);
    let month_days = if is_leap {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for &d in &month_days {
        if days_left < d {
            break;
        }
        days_left -= d;
        month += 1;
    }

    let day = days_left + 1;
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        year, month, day, hours, minutes
    )
}

fn render_runner_manager(f: &mut Frame, app: &App, area: Rect, border_style: Style) {
    let has_bottom_bar = matches!(
        &app.runner_state,
        RunnerState::Downloading(_) | RunnerState::Extracting | RunnerState::Error(_)
    );

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(if has_bottom_bar { 3 } else { 0 }),
        ])
        .split(area);

    let runners_block = Block::default()
        .title(" 󰘚 WINE/PROTON RUNNER ECOSYSTEM ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let items: Vec<ListItem> = if let RunnerState::FetchingReleases = app.runner_state {
        vec![ListItem::new(Line::from(vec![Span::styled(
            "  󰏔 Fetching latest releases from GitHub API...",
            Style::default().fg(COLOR_BORDER_ACTIVE),
        )]))]
    } else if app.config.runners.is_empty() {
        vec![ListItem::new(Line::from(vec![Span::styled(
            "  No runners available. Restart the app or check your internet connection.",
            Style::default().fg(COLOR_MUTED),
        )]))]
    } else {
        app.config
            .runners
            .iter()
            .enumerate()
            .map(|(idx, runner)| {
                let is_selected = app.selected_runner_idx == idx && is_right_focused(app);

                let status_span = if runner.installed {
                    Span::styled("󰄬 Installed ", Style::default().fg(COLOR_SUCCESS))
                } else if app.downloading_runner_id.as_ref() == Some(&runner.id) {
                    match app.runner_state {
                        RunnerState::Extracting => {
                            Span::styled("󰏔 Extracting... ", Style::default().fg(COLOR_WARN))
                        }
                        _ => Span::styled("󰏔 Downloading... ", Style::default().fg(COLOR_WARN)),
                    }
                } else {
                    Span::styled("󰏕 Available ", Style::default().fg(COLOR_INFO))
                };

                let mut name_span = Span::styled(
                    format!("{} ({})", runner.name, runner.version),
                    Style::default().fg(COLOR_TEXT),
                );

                if is_selected {
                    name_span = Span::styled(
                        format!("{} ({})", runner.name, runner.version),
                        Style::default()
                            .bg(COLOR_HIGHLIGHT_BG)
                            .fg(COLOR_HIGHLIGHT_FG)
                            .add_modifier(Modifier::BOLD),
                    );
                }

                ListItem::new(Line::from(vec![status_span, name_span]))
            })
            .collect()
    };

    let list = List::new(items).block(runners_block);
    f.render_widget(list, layout[0]);

    // Bottom status/progress bar
    match &app.runner_state {
        RunnerState::Downloading(pct) => {
            let r_id = app.downloading_runner_id.as_deref().unwrap_or_default();
            let runner_name = app
                .config
                .runners
                .iter()
                .find(|r| r.id == r_id)
                .map(|r| r.version.clone())
                .unwrap_or_default();

            let gauge_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(COLOR_BORDER_ACTIVE))
                .title(format!(" Downloading {} ", runner_name));

            let gauge = Gauge::default()
                .block(gauge_block)
                .gauge_style(Style::default().fg(COLOR_BORDER_ACTIVE).bg(COLOR_MUTED))
                .percent(*pct as u16);

            f.render_widget(gauge, layout[1]);
        }
        RunnerState::Extracting => {
            let r_id = app.downloading_runner_id.as_deref().unwrap_or_default();
            let runner_name = app
                .config
                .runners
                .iter()
                .find(|r| r.id == r_id)
                .map(|r| r.version.clone())
                .unwrap_or_default();

            let extract_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(COLOR_WARN))
                .title(" Extraction ");

            let text = Paragraph::new(Line::from(vec![Span::styled(
                format!(" 󰏔 Decompressing {} assets... Please wait", runner_name),
                Style::default().fg(COLOR_WARN).add_modifier(Modifier::BOLD),
            )]))
            .block(extract_block);

            f.render_widget(text, layout[1]);
        }
        RunnerState::Error(err) => {
            let error_block = Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(COLOR_WARN))
                .title(" Download Error ");

            let text = Paragraph::new(Line::from(vec![Span::styled(
                format!("  Error: {}", err),
                Style::default().fg(COLOR_WARN),
            )]))
            .block(error_block);

            f.render_widget(text, layout[1]);
        }
        _ => {}
    }
}

fn render_winetricks(f: &mut Frame, app: &App, area: Rect, border_style: Style) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let tools_block = Block::default()
        .title("  SELECT WINETRICKS OPERATION ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let items: Vec<ListItem> = app
        .winetricks_items
        .iter()
        .enumerate()
        .map(|(idx, (verb, desc))| {
            let is_selected = app.selected_winetricks_idx == idx && is_right_focused(app);

            let label = format!("  {:<16} - {}", verb, desc);
            let content = if is_selected {
                Span::styled(
                    label,
                    Style::default()
                        .bg(COLOR_HIGHLIGHT_BG)
                        .fg(COLOR_HIGHLIGHT_FG)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(label, Style::default().fg(COLOR_TEXT))
            };

            ListItem::new(Line::from(vec![content]))
        })
        .collect();

    let list = List::new(items).block(tools_block);
    f.render_widget(list, layout[0]);

    // Winetricks console log
    let console_block = Block::default()
        .title(" 󰆍 EXECUTION LOGS ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let logs_to_display = if app.winetricks_logs.len() > 10 {
        &app.winetricks_logs[app.winetricks_logs.len() - 10..]
    } else {
        &app.winetricks_logs
    };

    let log_lines: Vec<Line> = logs_to_display
        .iter()
        .map(|log| {
            if log.contains("Error") {
                Line::from(Span::styled(log, Style::default().fg(COLOR_WARN)))
            } else if log.contains("Executing") {
                Line::from(Span::styled(log, Style::default().fg(COLOR_INFO)))
            } else if log.contains("applied successfully") {
                Line::from(Span::styled(log, Style::default().fg(COLOR_SUCCESS)))
            } else {
                Line::from(Span::styled(log, Style::default().fg(COLOR_MUTED)))
            }
        })
        .collect();

    let console = Paragraph::new(log_lines).block(console_block);
    f.render_widget(console, layout[1]);
}

fn render_dependency_installer(f: &mut Frame, app: &App, area: Rect, border_style: Style) {
    let block = Block::default()
        .title("  SYSTEM DEPENDENCY INSTALLER ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let logs_to_display = if app.script_logs.len() > 20 {
        &app.script_logs[app.script_logs.len() - 20..]
    } else {
        &app.script_logs
    };

    let log_lines: Vec<Line> = logs_to_display
        .iter()
        .map(|log| {
            if log.contains("ERROR") || log.contains("errors") {
                Line::from(Span::styled(log, Style::default().fg(COLOR_WARN)))
            } else if log.contains("Executing") || log.contains("started") {
                Line::from(Span::styled(log, Style::default().fg(COLOR_INFO)))
            } else if log.contains("completed successfully") || log.contains("Successfully") {
                Line::from(Span::styled(log, Style::default().fg(COLOR_SUCCESS)))
            } else {
                Line::from(Span::styled(log, Style::default().fg(COLOR_TEXT)))
            }
        })
        .collect();

    let console = Paragraph::new(log_lines).block(block);
    f.render_widget(console, area);
}

fn render_settings(f: &mut Frame, app: &App, area: Rect, border_style: Style) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let settings_block = Block::default()
        .title("  GLOBAL CONFIGURATION ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let items: Vec<ListItem> = app
        .settings_items
        .iter()
        .enumerate()
        .map(|(idx, (label, val, is_bool))| {
            let is_selected = app.selected_settings_idx == idx && is_right_focused(app);

            let display_val = if *is_bool {
                if val == "Enabled" {
                    Span::styled(format!(" [✔] {}", val), Style::default().fg(COLOR_SUCCESS))
                } else {
                    Span::styled(format!(" [ ] {}", val), Style::default().fg(COLOR_MUTED))
                }
            } else {
                Span::styled(format!(" {}", val), Style::default().fg(COLOR_INFO))
            };

            let mut label_span = Span::styled(
                format!("  {:<32} : ", label),
                Style::default().fg(COLOR_TEXT),
            );
            if is_selected {
                label_span = Span::styled(
                    format!("  {:<32} : ", label),
                    Style::default()
                        .bg(COLOR_HIGHLIGHT_BG)
                        .fg(COLOR_HIGHLIGHT_FG)
                        .add_modifier(Modifier::BOLD),
                );
            }

            ListItem::new(Line::from(vec![label_span, display_val]))
        })
        .collect();

    let list = List::new(items).block(settings_block);
    f.render_widget(list, layout[0]);

    // Custom prefixes list
    let prefix_block = Block::default()
        .title(" 󰘚 CUSTOM WINE PREFIXES (Press [n] for New, [Enter] to Initialize) ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let prefix_items: Vec<ListItem> = if app.custom_prefixes.is_empty() {
        vec![ListItem::new(Line::from(vec![Span::styled(
            "  No custom prefixes configured. Press [n] to create one.",
            Style::default().fg(COLOR_MUTED),
        )]))]
    } else {
        app.custom_prefixes
            .iter()
            .enumerate()
            .map(|(idx, p)| {
                let global_idx = app.settings_items.len() + idx;
                let is_selected = app.selected_settings_idx == global_idx && is_right_focused(app);

                let status_span = match p.status.as_str() {
                    "Ready" => Span::styled("󰄬 Ready ", Style::default().fg(COLOR_SUCCESS)),
                    "Booting..." => Span::styled("󰏔 Booting... ", Style::default().fg(COLOR_WARN)),
                    _ => Span::styled("󰏕 Not Initialized ", Style::default().fg(COLOR_INFO)),
                };

                let label = format!(
                    "  {:<12} [{:<5}] -> {}",
                    p.name,
                    p.architecture,
                    p.path.to_string_lossy()
                );
                let content_span = if is_selected {
                    Span::styled(
                        label,
                        Style::default()
                            .bg(COLOR_HIGHLIGHT_BG)
                            .fg(COLOR_HIGHLIGHT_FG)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled(label, Style::default().fg(COLOR_TEXT))
                };

                ListItem::new(Line::from(vec![status_span, content_span]))
            })
            .collect()
    };

    let prefix_list = List::new(prefix_items).block(prefix_block);
    f.render_widget(prefix_list, layout[1]);
}

fn render_prefix_manager(f: &mut Frame, app: &App, area: Rect, border_style: Style) {
    let layout = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let list_focused = app.focus == PaneFocus::LibraryList;

    let prefix_block = Block::default()
        .title(" 󰘚 WINE PREFIXES (Press [n] for New, [x] to Delete) ")
        .borders(Borders::ALL)
        .border_style(if list_focused {
            Style::default().fg(COLOR_BORDER_ACTIVE)
        } else {
            border_style
        });

    let prefix_items: Vec<ListItem> = if app.custom_prefixes.is_empty() {
        vec![ListItem::new(Line::from(vec![Span::styled(
            "  No custom prefixes configured. Press [n] to create one.",
            Style::default().fg(COLOR_MUTED),
        )]))]
    } else {
        app.custom_prefixes
            .iter()
            .enumerate()
            .map(|(idx, p)| {
                let is_selected = app.selected_prefix_idx == idx && list_focused;

                let status_span = match p.status.as_str() {
                    "Ready" => Span::styled("󰄬 Ready ", Style::default().fg(COLOR_SUCCESS)),
                    "Booting..." => Span::styled("󰏔 Booting... ", Style::default().fg(COLOR_WARN)),
                    _ => Span::styled("󰏕 Not Initialized ", Style::default().fg(COLOR_INFO)),
                };

                let label = format!("  {:<16} [{:<5}]", p.name, p.architecture,);
                let content_span = if is_selected {
                    Span::styled(
                        label,
                        Style::default()
                            .bg(COLOR_HIGHLIGHT_BG)
                            .fg(COLOR_HIGHLIGHT_FG)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Span::styled(label, Style::default().fg(COLOR_TEXT))
                };

                ListItem::new(Line::from(vec![status_span, content_span]))
            })
            .collect()
    };

    let prefix_list = List::new(prefix_items).block(prefix_block);
    f.render_widget(prefix_list, layout[0]);

    // Right details pane
    let details_block = Block::default()
        .title(" 󰘚 PREFIX DETAILS ")
        .borders(Borders::ALL)
        .border_style(border_style);

    let details_area = layout[1];
    f.render_widget(details_block.clone(), details_area);

    let inner_details_area = details_block.inner(details_area);

    if !app.custom_prefixes.is_empty() && app.selected_prefix_idx < app.custom_prefixes.len() {
        let p = &app.custom_prefixes[app.selected_prefix_idx];

        let details_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Spacer
                Constraint::Length(2), // Name
                Constraint::Length(2), // Architecture
                Constraint::Length(2), // Status
                Constraint::Length(3), // Path
                Constraint::Min(0),    // Operations Help
            ])
            .split(inner_details_area);

        let name_p = Paragraph::new(Line::from(vec![
            Span::styled(" Name:          ", Style::default().fg(COLOR_MUTED)),
            Span::styled(
                &p.name,
                Style::default().fg(COLOR_TEXT).add_modifier(Modifier::BOLD),
            ),
        ]));

        let arch_p = Paragraph::new(Line::from(vec![
            Span::styled(" Architecture:  ", Style::default().fg(COLOR_MUTED)),
            Span::styled(&p.architecture, Style::default().fg(COLOR_INFO)),
        ]));

        let status_text = match p.status.as_str() {
            "Ready" => Span::styled(
                "Ready (Initial Boot Complete)",
                Style::default().fg(COLOR_SUCCESS),
            ),
            "Booting..." => Span::styled(
                "Booting / Updating registry overrides...",
                Style::default().fg(COLOR_WARN),
            ),
            _ => Span::styled(
                "Not Initialized (Requires initial boot)",
                Style::default().fg(COLOR_INFO),
            ),
        };

        let status_p = Paragraph::new(Line::from(vec![
            Span::styled(" Status:        ", Style::default().fg(COLOR_MUTED)),
            status_text,
        ]));

        let path_p = Paragraph::new(vec![
            Line::from(Span::styled(
                " Path:          ",
                Style::default().fg(COLOR_MUTED),
            )),
            Line::from(Span::styled(
                p.path.to_string_lossy().to_string(),
                Style::default().fg(COLOR_TEXT),
            )),
        ]);

        let operations_box = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(COLOR_MUTED))
            .title(" AVAILABLE OPERATIONS ");

        let operations_text = vec![
            Line::from(Span::styled(
                " [Enter]  Boot / Initialize Prefix",
                Style::default().fg(COLOR_TEXT),
            )),
            Line::from(Span::styled(
                " [x]      Delete / Deregister Prefix",
                Style::default().fg(COLOR_WARN),
            )),
            Line::from(Span::styled(
                " [n]      Create New Custom Prefix",
                Style::default().fg(COLOR_INFO),
            )),
        ];
        let operations_p = Paragraph::new(operations_text).block(operations_box);

        f.render_widget(name_p, details_layout[1]);
        f.render_widget(arch_p, details_layout[2]);
        f.render_widget(status_p, details_layout[3]);
        f.render_widget(path_p, details_layout[4]);
        f.render_widget(operations_p, details_layout[5]);
    } else {
        let empty_p = Paragraph::new(Line::from(Span::styled(
            " Select or create a prefix to view details.",
            Style::default().fg(COLOR_MUTED),
        )));
        f.render_widget(empty_p, inner_details_area);
    }
}

fn render_delete_prefix_modal(
    f: &mut Frame,
    prefix_name: &str,
    prefix_path: &std::path::Path,
    prompt_delete_files: bool,
    area: Rect,
) {
    let popup_area = centered_rect(50, 30, area);

    // Clear background
    f.render_widget(Clear, popup_area);

    let popup_block = Block::default()
        .title("  CONFIRM PREFIX DELETION ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_WARN))
        .style(Style::default().bg(COLOR_BG).fg(COLOR_TEXT));

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Spacer
            Constraint::Length(2), // Message line 1
            Constraint::Length(2), // Message line 2
            Constraint::Min(0),    // Action keys
        ])
        .split(popup_block.inner(popup_area));

    f.render_widget(popup_block, popup_area);

    if !prompt_delete_files {
        let msg1 = Paragraph::new(Line::from(vec![
            Span::styled(
                " Are you sure you want to remove ",
                Style::default().fg(COLOR_TEXT),
            ),
            Span::styled(
                prefix_name,
                Style::default()
                    .fg(COLOR_BORDER_ACTIVE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" from configuration?", Style::default().fg(COLOR_TEXT)),
        ]));
        let msg2 = Paragraph::new(" This will unregister the Wine prefix from LGTUI.");
        let footer = Paragraph::new(Line::from(vec![Span::styled(
            " [Enter/y] Confirm   [Esc/n] Cancel",
            Style::default().fg(COLOR_MUTED),
        )]));
        f.render_widget(msg1, inner_chunks[1]);
        f.render_widget(msg2, inner_chunks[2]);
        f.render_widget(footer, inner_chunks[3]);
    } else {
        let msg1 = Paragraph::new(" Do you also want to delete all prefix files on disk?");
        let path_str = prefix_path.to_string_lossy();
        let msg2 = Paragraph::new(Line::from(vec![
            Span::styled(" Path: ", Style::default().fg(COLOR_MUTED)),
            Span::styled(path_str, Style::default().fg(COLOR_WARN)),
        ]))
        .wrap(Wrap { trim: false });
        let footer = Paragraph::new(Line::from(vec![Span::styled(
            " [Enter/y] Yes, Delete Files   [n] No, Keep Files",
            Style::default().fg(COLOR_MUTED),
        )]));
        f.render_widget(msg1, inner_chunks[1]);
        f.render_widget(msg2, inner_chunks[2]);
        f.render_widget(footer, inner_chunks[3]);
    }
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let shortcuts = match app.focus {
        PaneFocus::LeftMenu => {
            "[Tab/l] Select Menu   [j/k] Navigate Menu   [a] Add Game   [i] Install Deps   [?] Help   [q] Quit"
        }
        PaneFocus::LibraryList => match app.current_menu() {
            MenuItem::Library => {
                "[Enter] Launch Game   [Tab] Cycle Focus   [a] Add Game   [x] Delete Game   [?] Help   [q] Quit"
            }
            MenuItem::RunnerManager => {
                "[Enter] Install Runner   [j/k] Select   [Tab] Menu   [?] Help   [q] Quit"
            }
            MenuItem::PrefixManager => {
                "[Enter] Init Prefix   [x] Delete Prefix   [n] New Prefix   [j/k] Select   [Tab] Menu   [?] Help   [q] Quit"
            }
            MenuItem::Winetricks => {
                "[Enter] Apply Verb   [j/k] Select   [Tab] Menu   [?] Help   [q] Quit"
            }
            MenuItem::DependencyInstaller => {
                "[Enter] Run Installer Script   [Tab] Menu   [?] Help   [q] Quit"
            }
            MenuItem::Settings => {
                "[Space] Toggle   [Enter] Init Prefix   [n] New Prefix   [j/k] Navigate   [Tab] Menu   [?] Help   [q] Quit"
            }
        },
        PaneFocus::MetadataPanel => match app.current_menu() {
            MenuItem::Library => {
                "[Space] Toggle Option   [j/k] Navigate Config   [Tab] Cycle Focus   [?] Help   [q] Quit"
            }
            _ => "[Tab] Cycle Focus   [?] Help   [q] Quit",
        },
    };

    let footer = Paragraph::new(Line::from(vec![Span::styled(
        shortcuts,
        Style::default().bg(COLOR_MUTED).fg(COLOR_HIGHLIGHT_FG),
    )]));
    f.render_widget(footer, area);
}

fn format_playtime(secs: u64) -> String {
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;
    format!("{}h {}m {}s", hours, minutes, seconds)
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn render_new_prefix_modal(f: &mut Frame, modal: &crate::app::ModalState, area: Rect) {
    use crate::app::ModalFocus;
    let popup_area = centered_rect(50, 40, area);

    // Clear background
    f.render_widget(Clear, popup_area);

    let popup_block = Block::default()
        .title(" 󰘚 CREATE NEW WINE PREFIX ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BORDER_ACTIVE))
        .style(Style::default().bg(COLOR_BG).fg(COLOR_TEXT));

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
        ])
        .split(popup_block.inner(popup_area));

    f.render_widget(popup_block, popup_area);

    // Prefix Name Field
    let name_focused = modal.focus == ModalFocus::NameInput;
    let name_border_color = if name_focused {
        COLOR_BORDER_ACTIVE
    } else {
        COLOR_BORDER_INACTIVE
    };
    let name_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(name_border_color))
        .title(" Prefix Name ");
    let name_para = Paragraph::new(format!("  {}", modal.name_input)).block(name_block);
    f.render_widget(name_para, inner_chunks[1]);

    // Wine Architecture Toggle Field
    let arch_focused = modal.focus == ModalFocus::ArchToggle;
    let arch_border_color = if arch_focused {
        COLOR_BORDER_ACTIVE
    } else {
        COLOR_BORDER_INACTIVE
    };
    let arch_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(arch_border_color))
        .title(" Wine Architecture ");
    let arch_label = if modal.is_64bit {
        "   < 64-bit (win64) >"
    } else {
        "   < 32-bit (win32) >"
    };
    let arch_para = Paragraph::new(Span::styled(
        arch_label,
        Style::default()
            .fg(COLOR_SUCCESS)
            .add_modifier(Modifier::BOLD),
    ))
    .block(arch_block);
    f.render_widget(arch_para, inner_chunks[3]);

    // Bottom Action keys hints
    let footer_hints = Paragraph::new(Line::from(vec![Span::styled(
        " [Tab] Toggle Focus   [Enter] Create   [Esc] Cancel",
        Style::default().fg(COLOR_MUTED),
    )]));
    f.render_widget(footer_hints, inner_chunks[5]);
}

fn validate_exec_path(path: &str) -> bool {
    if path.trim().is_empty() {
        return false;
    }
    let invalid_chars = ['*', '?', '"', '<', '>', '|'];
    !path.chars().any(|c| invalid_chars.contains(&c))
}

fn render_add_game_modal(
    f: &mut Frame,
    app: &App,
    modal: &crate::app::AddGameModalState,
    area: Rect,
) {
    use crate::app::AddGameFocus;
    let popup_area = centered_rect(65, 80, area);

    f.render_widget(Clear, popup_area);

    let popup_block = Block::default()
        .title(" 󰊖 ADD NEW GAME TO LIBRARY ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BORDER_ACTIVE))
        .style(Style::default().bg(COLOR_BG).fg(COLOR_TEXT));

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Game Title
            Constraint::Length(3), // Exec Path
            Constraint::Length(3), // WINEPREFIX
            Constraint::Length(3), // Runner Selection
            Constraint::Length(3), // Toggles Row 1 (DXVK, VKD3D, MangoHud)
            Constraint::Length(3), // Toggles Row 2 (GameMode, Installer)
            Constraint::Min(0),    // Error/validation messages
            Constraint::Length(1), // Action hints
        ])
        .split(popup_block.inner(popup_area));

    f.render_widget(popup_block, popup_area);

    // Game Title Field
    let title_focused = modal.focus == AddGameFocus::Title;
    let title_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if title_focused {
            COLOR_BORDER_ACTIVE
        } else {
            COLOR_BORDER_INACTIVE
        }))
        .title(" Game Title ");
    let title_para = Paragraph::new(format!("  {}", modal.title)).block(title_block);
    f.render_widget(title_para, inner_chunks[0]);

    // Executable Path Field
    let exec_focused = modal.focus == AddGameFocus::ExecPath;
    let path_valid = validate_exec_path(&modal.exec_path);
    let exec_border_color = if exec_focused {
        COLOR_BORDER_ACTIVE
    } else if !modal.exec_path.is_empty() && !path_valid {
        COLOR_WARN
    } else {
        COLOR_BORDER_INACTIVE
    };
    let exec_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(exec_border_color))
        .title(" Executable Path (Auto-detects setup.exe / install.exe) ");
    let exec_para = Paragraph::new(format!("  {}", modal.exec_path)).block(exec_block);
    f.render_widget(exec_para, inner_chunks[1]);

    // WINEPREFIX Name Field
    let prefix_focused = modal.focus == AddGameFocus::PrefixName;
    let prefix_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if prefix_focused {
            COLOR_BORDER_ACTIVE
        } else {
            COLOR_BORDER_INACTIVE
        }))
        .title(" Custom WINEPREFIX Name ");
    let prefix_para = Paragraph::new(format!("  {}", modal.prefix_name)).block(prefix_block);
    f.render_widget(prefix_para, inner_chunks[2]);

    // Runner Selection Field
    let runner_focused = modal.focus == AddGameFocus::Runner;
    let runner_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if runner_focused {
            COLOR_BORDER_ACTIVE
        } else {
            COLOR_BORDER_INACTIVE
        }))
        .title(" Runner / Compatibility Layer (Use Left/Right to Cycle) ");

    let runner_name = if modal.selected_runner_idx == 0 {
        "System Wine (Default)".to_string()
    } else if let Some(r) = app.config.runners.get(modal.selected_runner_idx - 1) {
        format!("{} ({})", r.name, r.version)
    } else {
        "Unknown Runner".to_string()
    };
    let runner_para = Paragraph::new(format!("  {}", runner_name)).block(runner_block);
    f.render_widget(runner_para, inner_chunks[3]);

    // Row 1 Toggles (DXVK, VKD3D, MangoHud)
    let r1_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(33),
            Constraint::Percentage(34),
        ])
        .split(inner_chunks[4]);

    // DXVK
    let dxvk_focused = modal.focus == AddGameFocus::Dxvk;
    let dxvk_text = if modal.dxvk {
        "[x] DXVK Wrapper"
    } else {
        "[ ] DXVK Wrapper"
    };
    let dxvk_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if dxvk_focused {
            COLOR_BORDER_ACTIVE
        } else {
            COLOR_BORDER_INACTIVE
        }))
        .title(" DXVK ");
    let dxvk_para = Paragraph::new(format!("  {}", dxvk_text)).block(dxvk_block);
    f.render_widget(dxvk_para, r1_chunks[0]);

    // VKD3D
    let vkd3d_focused = modal.focus == AddGameFocus::Vkd3d;
    let vkd3d_text = if modal.vkd3d {
        "[x] VKD3D Wrapper"
    } else {
        "[ ] VKD3D Wrapper"
    };
    let vkd3d_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if vkd3d_focused {
            COLOR_BORDER_ACTIVE
        } else {
            COLOR_BORDER_INACTIVE
        }))
        .title(" VKD3D ");
    let vkd3d_para = Paragraph::new(format!("  {}", vkd3d_text)).block(vkd3d_block);
    f.render_widget(vkd3d_para, r1_chunks[1]);

    // MangoHud
    let mangohud_focused = modal.focus == AddGameFocus::MangoHud;
    let mangohud_text = if modal.mangohud {
        "[x] MangoHud Overlay"
    } else {
        "[ ] MangoHud Overlay"
    };
    let mangohud_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if mangohud_focused {
            COLOR_BORDER_ACTIVE
        } else {
            COLOR_BORDER_INACTIVE
        }))
        .title(" MangoHud ");
    let mangohud_para = Paragraph::new(format!("  {}", mangohud_text)).block(mangohud_block);
    f.render_widget(mangohud_para, r1_chunks[2]);

    // Row 2 Toggles (GameMode, Installer Pipeline)
    let r2_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(inner_chunks[5]);

    // GameMode
    let gamemode_focused = modal.focus == AddGameFocus::GameMode;
    let gamemode_text = if modal.gamemode {
        "[x] GameMode Optimizer"
    } else {
        "[ ] GameMode Optimizer"
    };
    let gamemode_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if gamemode_focused {
            COLOR_BORDER_ACTIVE
        } else {
            COLOR_BORDER_INACTIVE
        }))
        .title(" GameMode ");
    let gamemode_para = Paragraph::new(format!("  {}", gamemode_text)).block(gamemode_block);
    f.render_widget(gamemode_para, r2_chunks[0]);

    // Installer Pipeline
    let installer_focused = modal.focus == AddGameFocus::IsInstaller;
    let installer_text = if modal.is_installer {
        "[x] Installer Pipeline"
    } else {
        "[ ] Installer Pipeline"
    };
    let installer_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(if installer_focused {
            COLOR_BORDER_ACTIVE
        } else {
            COLOR_BORDER_INACTIVE
        }))
        .title(" Setup/Installer Mode ");
    let installer_para = Paragraph::new(format!("  {}", installer_text)).block(installer_block);
    f.render_widget(installer_para, r2_chunks[1]);

    // Error / validation messages
    let mut validation_msg = Vec::new();
    if !modal.exec_path.is_empty() && !path_valid {
        validation_msg.push(Line::from(vec![Span::styled(
            "  Invalid path: executable path cannot contain *, ?, \", <, >, |",
            Style::default().fg(COLOR_WARN),
        )]));
    } else if modal.title.trim().is_empty() {
        validation_msg.push(Line::from(vec![Span::styled(
            "  Enter game title to continue.",
            Style::default().fg(COLOR_MUTED),
        )]));
    } else {
        validation_msg.push(Line::from(vec![Span::styled(
            "  Input settings are valid.",
            Style::default().fg(COLOR_SUCCESS),
        )]));
    }
    let validation_para = Paragraph::new(validation_msg);
    f.render_widget(validation_para, inner_chunks[6]);

    // Footer hints
    let footer_hints = Paragraph::new(Line::from(vec![Span::styled(
        " [Tab] Cycle Focus   [Space/Arrows] Toggle   [Enter] Add Game   [Esc] Cancel",
        Style::default().fg(COLOR_MUTED),
    )]));
    f.render_widget(footer_hints, inner_chunks[7]);
}

fn render_onboarding(f: &mut Frame, onboarding: &crate::onboarding::OnboardingState, area: Rect) {
    use crate::onboarding::OnboardingStep;

    let popup_area = centered_rect(65, 60, area);

    f.render_widget(Clear, popup_area);

    let border_style = Style::default().fg(COLOR_BORDER_ACTIVE);
    let popup_block = Block::default()
        .title(" 󰊖 LGTUI ONBOARDING & DIAGNOSTICS ")
        .borders(Borders::ALL)
        .border_style(border_style)
        .style(Style::default().bg(COLOR_BG).fg(COLOR_TEXT));

    let inner_area = popup_block.inner(popup_area);
    f.render_widget(popup_block, popup_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Banner / title
            Constraint::Min(0),    // Main content
            Constraint::Length(3), // Footer / action
        ])
        .split(inner_area);

    let banner_text = vec![
        Line::from(vec![Span::styled(
            " 󰊖 Welcome to LGTUI Onboarding & Setup System ",
            Style::default().fg(COLOR_WARN).add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            " Let's configure your high-performance Linux gaming ecosystem.",
            Style::default().fg(COLOR_MUTED),
        )]),
    ];
    let banner_paragraph =
        Paragraph::new(banner_text).alignment(ratatui::layout::Alignment::Center);
    f.render_widget(banner_paragraph, chunks[0]);

    match &onboarding.step {
        OnboardingStep::Welcome => {
            let welcome_text = vec![
                Line::from(""),
                Line::from(vec![Span::styled(
                    "  Initializing Silent System Diagnostics Scan...",
                    Style::default().fg(COLOR_INFO).add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
                Line::from("  Checking essential packages on your system:"),
                Line::from("    • Wine Staging / Wine"),
                Line::from("    • Winetricks Utilities"),
                Line::from("    • MangoHud Overlay"),
                Line::from("    • GameMode Daemon"),
                Line::from(""),
                Line::from(vec![Span::styled(
                    "  Please wait a brief moment for the check to complete.",
                    Style::default().fg(COLOR_MUTED),
                )]),
            ];
            let welcome_paragraph = Paragraph::new(welcome_text);
            f.render_widget(welcome_paragraph, chunks[1]);

            let footer_text = vec![Line::from(vec![Span::styled(
                " [Scanning system...] ",
                Style::default().fg(COLOR_MUTED),
            )])];
            let footer_paragraph =
                Paragraph::new(footer_text).alignment(ratatui::layout::Alignment::Center);
            f.render_widget(footer_paragraph, chunks[2]);
        }
        OnboardingStep::ScanResult(res) => {
            let mut list_items = vec![Line::from("  Diagnostic results:"), Line::from("")];

            let check_icon = Span::styled(
                " [✓] ",
                Style::default()
                    .fg(COLOR_SUCCESS)
                    .add_modifier(Modifier::BOLD),
            );
            let warn_icon = Span::styled(
                " [✗] ",
                Style::default().fg(COLOR_WARN).add_modifier(Modifier::BOLD),
            );

            let wine_status = if res.wine {
                vec![
                    check_icon.clone(),
                    Span::raw("Wine Layer: "),
                    Span::styled("Found", Style::default().fg(COLOR_SUCCESS)),
                ]
            } else {
                vec![
                    warn_icon.clone(),
                    Span::raw("Wine Layer: "),
                    Span::styled("Missing", Style::default().fg(COLOR_WARN)),
                ]
            };
            list_items.push(Line::from(wine_status));

            let winetricks_status = if res.winetricks {
                vec![
                    check_icon.clone(),
                    Span::raw("Winetricks: "),
                    Span::styled("Found", Style::default().fg(COLOR_SUCCESS)),
                ]
            } else {
                vec![
                    warn_icon.clone(),
                    Span::raw("Winetricks: "),
                    Span::styled("Missing", Style::default().fg(COLOR_WARN)),
                ]
            };
            list_items.push(Line::from(winetricks_status));

            let mangohud_status = if res.mangohud {
                vec![
                    check_icon.clone(),
                    Span::raw("MangoHud Overlay: "),
                    Span::styled("Found", Style::default().fg(COLOR_SUCCESS)),
                ]
            } else {
                vec![
                    warn_icon.clone(),
                    Span::raw("MangoHud Overlay: "),
                    Span::styled("Missing", Style::default().fg(COLOR_WARN)),
                ]
            };
            list_items.push(Line::from(mangohud_status));

            let gamemode_status = if res.gamemode {
                vec![
                    check_icon.clone(),
                    Span::raw("GameMode Daemon: "),
                    Span::styled("Found", Style::default().fg(COLOR_SUCCESS)),
                ]
            } else {
                vec![
                    warn_icon.clone(),
                    Span::raw("GameMode Daemon: "),
                    Span::styled("Missing", Style::default().fg(COLOR_WARN)),
                ]
            };
            list_items.push(Line::from(gamemode_status));

            list_items.push(Line::from(""));

            if res.has_missing() {
                list_items.push(Line::from(vec![Span::styled(
                    "  Hey, we detected some missing gaming layers on your system.",
                    Style::default().fg(COLOR_WARN),
                )]));
                list_items.push(Line::from(vec![
                    Span::styled("  Can I install all the necessary dependencies to your PC for the best performance experience?", Style::default().fg(COLOR_TEXT)),
                ]));

                let scan_paragraph = Paragraph::new(list_items);
                f.render_widget(scan_paragraph, chunks[1]);

                let yes_style = if onboarding.selected_yes {
                    Style::default()
                        .bg(COLOR_SUCCESS)
                        .fg(COLOR_HIGHLIGHT_FG)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().bg(COLOR_MUTED).fg(COLOR_TEXT)
                };
                let no_style = if !onboarding.selected_yes {
                    Style::default()
                        .bg(COLOR_WARN)
                        .fg(COLOR_HIGHLIGHT_FG)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().bg(COLOR_MUTED).fg(COLOR_TEXT)
                };

                let button_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([
                        Constraint::Percentage(30),
                        Constraint::Percentage(20),
                        Constraint::Percentage(20),
                        Constraint::Percentage(30),
                    ])
                    .split(chunks[2]);

                let yes_btn = Paragraph::new(" [ Yes ] ")
                    .style(yes_style)
                    .alignment(ratatui::layout::Alignment::Center);
                let no_btn = Paragraph::new(" [ No ] ")
                    .style(no_style)
                    .alignment(ratatui::layout::Alignment::Center);
                f.render_widget(yes_btn, button_chunks[1]);
                f.render_widget(no_btn, button_chunks[2]);
            } else {
                list_items.push(Line::from(vec![Span::styled(
                    "  All required gaming components and libraries are successfully configured!",
                    Style::default()
                        .fg(COLOR_SUCCESS)
                        .add_modifier(Modifier::BOLD),
                )]));
                let scan_paragraph = Paragraph::new(list_items);
                f.render_widget(scan_paragraph, chunks[1]);

                let footer_text = vec![Line::from(vec![
                    Span::styled(" Press ", Style::default().fg(COLOR_TEXT)),
                    Span::styled(
                        "[Enter]",
                        Style::default()
                            .fg(COLOR_BORDER_ACTIVE)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        " to launch the LGTUI Library ",
                        Style::default().fg(COLOR_TEXT),
                    ),
                ])];
                let footer_paragraph =
                    Paragraph::new(footer_text).alignment(ratatui::layout::Alignment::Center);
                f.render_widget(footer_paragraph, chunks[2]);
            }
        }
        OnboardingStep::Installing {
            logs,
            completed,
            success,
        } => {
            let mut list_items = vec![
                Line::from(vec![Span::styled(
                    "  Dependency Installer execution logs:",
                    Style::default().fg(COLOR_INFO).add_modifier(Modifier::BOLD),
                )]),
                Line::from(""),
            ];

            let max_lines = (chunks[1].height as usize).saturating_sub(4);
            let start_idx = logs.len().saturating_sub(max_lines);
            for log in &logs[start_idx..] {
                list_items.push(Line::from(format!("    {}", log)));
            }

            let logs_paragraph = Paragraph::new(list_items).wrap(Wrap { trim: false });
            f.render_widget(logs_paragraph, chunks[1]);

            if *completed {
                let footer_line = if *success {
                    vec![
                        Span::styled(
                            " Installation Succeeded! Press ",
                            Style::default().fg(COLOR_SUCCESS),
                        ),
                        Span::styled(
                            "[Enter]",
                            Style::default()
                                .fg(COLOR_BORDER_ACTIVE)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            " to proceed to LGTUI Library ",
                            Style::default().fg(COLOR_TEXT),
                        ),
                    ]
                } else {
                    vec![
                        Span::styled(
                            " Installation Failed. Press ",
                            Style::default().fg(COLOR_WARN),
                        ),
                        Span::styled(
                            "[Enter]",
                            Style::default()
                                .fg(COLOR_BORDER_ACTIVE)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(" to continue anyway ", Style::default().fg(COLOR_TEXT)),
                    ]
                };
                let footer_paragraph = Paragraph::new(vec![Line::from(footer_line)])
                    .alignment(ratatui::layout::Alignment::Center);
                f.render_widget(footer_paragraph, chunks[2]);
            } else {
                let footer_paragraph = Paragraph::new(vec![Line::from(vec![Span::styled(
                    " [Installing dependencies, please wait...] ",
                    Style::default().fg(COLOR_MUTED),
                )])])
                .alignment(ratatui::layout::Alignment::Center);
                f.render_widget(footer_paragraph, chunks[2]);
            }
        }
    }
}

fn render_delete_game_modal(
    f: &mut Frame,
    game_name: &str,
    wineprefix: &Option<String>,
    prompt_prefix: bool,
    area: Rect,
) {
    let popup_area = centered_rect(50, 30, area);

    // Clear background
    f.render_widget(Clear, popup_area);

    let popup_block = Block::default()
        .title("  CONFIRM GAME DELETION ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_WARN))
        .style(Style::default().bg(COLOR_BG).fg(COLOR_TEXT));

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Spacer
            Constraint::Length(2), // Message line 1
            Constraint::Length(2), // Message line 2
            Constraint::Min(0),    // Action keys
        ])
        .split(popup_block.inner(popup_area));

    f.render_widget(popup_block, popup_area);

    if !prompt_prefix {
        let msg1 = Paragraph::new(Line::from(vec![
            Span::styled(
                " Are you sure you want to delete ",
                Style::default().fg(COLOR_TEXT),
            ),
            Span::styled(
                game_name,
                Style::default()
                    .fg(COLOR_BORDER_ACTIVE)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("?", Style::default().fg(COLOR_TEXT)),
        ]));
        let msg2 = Paragraph::new(" This will remove the game configuration from library.");
        let footer = Paragraph::new(Line::from(vec![Span::styled(
            " [Enter/y] Confirm   [Esc/n] Cancel",
            Style::default().fg(COLOR_MUTED),
        )]));
        f.render_widget(msg1, inner_chunks[1]);
        f.render_widget(msg2, inner_chunks[2]);
        f.render_widget(footer, inner_chunks[3]);
    } else {
        let msg1 =
            Paragraph::new(" Do you also want to destroy the corresponding WINEPREFIX directory?");
        let prefix_path = wineprefix.as_deref().unwrap_or("N/A");
        let msg2 = Paragraph::new(Line::from(vec![
            Span::styled(" Path: ", Style::default().fg(COLOR_MUTED)),
            Span::styled(prefix_path, Style::default().fg(COLOR_WARN)),
        ]))
        .wrap(Wrap { trim: false });
        let footer = Paragraph::new(Line::from(vec![Span::styled(
            " [Enter/y] Yes, Delete   [Esc/n] No, Keep Prefix",
            Style::default().fg(COLOR_MUTED),
        )]));
        f.render_widget(msg1, inner_chunks[1]);
        f.render_widget(msg2, inner_chunks[2]);
        f.render_widget(footer, inner_chunks[3]);
    }
}

fn render_help_modal(f: &mut Frame, area: Rect) {
    let popup_area = centered_rect(60, 55, area);

    // Clear background
    f.render_widget(Clear, popup_area);

    let popup_block = Block::default()
        .title(" 󰘥 LGTUI KEYBINDINGS & SHORTCUTS HELP ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_BORDER_ACTIVE))
        .style(Style::default().bg(COLOR_BG).fg(COLOR_TEXT));

    let inner_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Spacer
            Constraint::Min(0),    // Help content table / paragraph
            Constraint::Length(1), // Action hints
        ])
        .split(popup_block.inner(popup_area));

    f.render_widget(popup_block, popup_area);

    let help_items = vec![
        Line::from(vec![Span::styled(
            "  Key     Action",
            Style::default().add_modifier(Modifier::BOLD).fg(COLOR_WARN),
        )]),
        Line::from("  -------------------------------------------------------------"),
        Line::from(vec![
            Span::styled(
                "  Tab     ",
                Style::default().fg(COLOR_INFO).add_modifier(Modifier::BOLD),
            ),
            Span::raw("Cycle Focus (Left Navigator, Game List, Details)"),
        ]),
        Line::from(vec![
            Span::styled(
                "  a       ",
                Style::default().fg(COLOR_INFO).add_modifier(Modifier::BOLD),
            ),
            Span::raw("Add custom Windows Game (or setup.exe Installer)"),
        ]),
        Line::from(vec![
            Span::styled(
                "  x       ",
                Style::default().fg(COLOR_INFO).add_modifier(Modifier::BOLD),
            ),
            Span::raw("Delete selected game (with WINEPREFIX purge options)"),
        ]),
        Line::from(vec![
            Span::styled(
                "  n       ",
                Style::default().fg(COLOR_INFO).add_modifier(Modifier::BOLD),
            ),
            Span::raw("Create new custom Wine Prefix directory"),
        ]),
        Line::from(vec![
            Span::styled(
                "  i       ",
                Style::default().fg(COLOR_INFO).add_modifier(Modifier::BOLD),
            ),
            Span::raw("Switch to tab & install Linux gaming dependencies"),
        ]),
        Line::from(vec![
            Span::styled(
                "  Enter   ",
                Style::default().fg(COLOR_INFO).add_modifier(Modifier::BOLD),
            ),
            Span::raw("Launch game / download runner / apply custom options"),
        ]),
        Line::from(vec![
            Span::styled(
                "  Esc     ",
                Style::default().fg(COLOR_INFO).add_modifier(Modifier::BOLD),
            ),
            Span::raw("Close active dialog or cancel overlay screen"),
        ]),
        Line::from(vec![
            Span::styled(
                "  ?       ",
                Style::default().fg(COLOR_INFO).add_modifier(Modifier::BOLD),
            ),
            Span::raw("Toggle this keyboard shortcuts help modal"),
        ]),
        Line::from(vec![
            Span::styled(
                "  q       ",
                Style::default().fg(COLOR_INFO).add_modifier(Modifier::BOLD),
            ),
            Span::raw("Quit LGTUI application cleanly"),
        ]),
    ];

    let help_para = Paragraph::new(help_items);
    f.render_widget(help_para, inner_chunks[1]);

    let footer_hints = Paragraph::new(Line::from(vec![Span::styled(
        " Press any key to close help overlay ",
        Style::default().fg(COLOR_MUTED),
    )]))
    .alignment(ratatui::layout::Alignment::Center);
    f.render_widget(footer_hints, inner_chunks[2]);
}
