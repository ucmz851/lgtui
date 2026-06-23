use tokio::sync::mpsc;

use crate::game::{Config, GameStatus};
use crate::prefix::{initialize_prefix, WinePrefix};
use crate::runner::{AppEvent, RunnerManager};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaneFocus {
    LeftMenu,
    LibraryList,
    MetadataPanel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuItem {
    Library = 0,
    RunnerManager = 1,
    PrefixManager = 2,
    Winetricks = 3,
    DependencyInstaller = 4,
    Settings = 5,
}

impl MenuItem {
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => MenuItem::Library,
            1 => MenuItem::RunnerManager,
            2 => MenuItem::PrefixManager,
            3 => MenuItem::Winetricks,
            4 => MenuItem::DependencyInstaller,
            _ => MenuItem::Settings,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunnerState {
    Idle,
    FetchingReleases,
    Downloading(u8),
    Extracting,
    Error(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModalFocus {
    NameInput,
    ArchToggle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModalState {
    pub name_input: String,
    pub is_64bit: bool,
    pub focus: ModalFocus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddGameFocus {
    Title,
    ExecPath,
    PrefixName,
    Runner,
    Dxvk,
    Vkd3d,
    MangoHud,
    GameMode,
    IsInstaller,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddGameModalState {
    pub title: String,
    pub exec_path: String,
    pub prefix_name: String,
    pub prefix_edited: bool,
    pub selected_runner_idx: usize,
    pub dxvk: bool,
    pub vkd3d: bool,
    pub mangohud: bool,
    pub gamemode: bool,
    pub is_installer: bool,
    pub focus: AddGameFocus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActiveModal {
    CreatePrefix(ModalState),
    AddGame(AddGameModalState),
    DeleteGame {
        game_idx: usize,
        delete_prefix: bool,
        prompt_prefix: bool,
    },
    DeletePrefix {
        prefix_idx: usize,
        delete_files: bool,
        prompt_delete_files: bool,
    },
    Help,
}

pub struct App {
    pub config: Config,
    pub focus: PaneFocus,
    pub menu_index: usize,

    // Sub-pane selections
    pub selected_game_idx: usize,
    pub selected_runner_idx: usize,
    pub selected_prefix_idx: usize,
    pub selected_winetricks_idx: usize,
    pub selected_settings_idx: usize,
    pub selected_metadata_idx: usize,

    // Background task reporting states
    pub downloading_runner_id: Option<String>,
    pub download_progress: u32,

    // Winetricks simulation
    pub winetricks_items: Vec<(&'static str, &'static str)>,
    pub winetricks_logs: Vec<String>,
    pub winetricks_running: bool,
    pub winetricks_timer: Option<u32>,

    // Global settings states
    pub settings_items: Vec<(String, String, bool)>, // (Label, Value, IsBoolean)

    // Prefix management states
    pub custom_prefixes: Vec<WinePrefix>,
    pub active_modal: Option<ActiveModal>,

    // System dependency script runner states
    pub script_logs: Vec<String>,
    pub script_running: bool,
    pub script_path: std::path::PathBuf,

    pub should_quit: bool,
    pub runner_state: RunnerState,
    pub event_tx: mpsc::Sender<AppEvent>,
    pub onboarding: Option<crate::onboarding::OnboardingState>,
}

impl App {
    pub fn new(event_tx: mpsc::Sender<AppEvent>) -> Self {
        let config = Config::load_or_create();

        let winetricks_items = vec![
            ("winecfg", "Configure Wine prefix settings"),
            ("winemine", "Launch WineMine"),
            ("dxvk", "Install/Update DXVK in prefix"),
            ("vkd3d", "Install/Update VKD3D-Proton in prefix"),
            ("d3dcompiler_47", "Install d3dcompiler_47 DLL overrides"),
            ("vcrun2015", "Install MSVC 2015 Redistributable"),
        ];

        let settings_items = vec![
            (
                "Default Wineprefix Directory".to_string(),
                config.default_wineprefix.clone(),
                false,
            ),
            (
                "Runner Download Cache".to_string(),
                config.runner_download_dir.clone(),
                false,
            ),
            (
                "Enable MangoHud Globally".to_string(),
                "Enabled".to_string(),
                true,
            ),
            (
                "Enable GameMode Globally".to_string(),
                "Disabled".to_string(),
                true,
            ),
        ];

        let conn = crate::database::init_db().expect("Failed to initialize database");
        let custom_prefixes = crate::database::get_all_prefixes(&conn).unwrap_or_default();

        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let script_path = std::path::PathBuf::from(home).join(".config/lgtui/install_deps.sh");

        let onboarding = if crate::onboarding::needs_onboarding() {
            let tx = event_tx.clone();
            tokio::spawn(async move {
                let res = crate::onboarding::DiagnosticsResult::perform();
                let _ = tx.send(AppEvent::OnboardingScanFinished(res)).await;
            });
            Some(crate::onboarding::OnboardingState {
                step: crate::onboarding::OnboardingStep::Welcome,
                selected_yes: true,
            })
        } else {
            None
        };

        let app = App {
            config,
            focus: PaneFocus::LeftMenu,
            menu_index: 0,
            selected_game_idx: 0,
            selected_runner_idx: 0,
            selected_prefix_idx: 0,
            selected_winetricks_idx: 0,
            selected_settings_idx: 0,
            selected_metadata_idx: 0,
            downloading_runner_id: None,
            download_progress: 0,
            winetricks_items,
            winetricks_logs: vec!["LGTUI Winetricks console initialized...".to_string()],
            winetricks_running: false,
            winetricks_timer: None,
            settings_items,
            custom_prefixes,
            active_modal: None,
            script_logs: vec![
                "LGTUI Dependency Installer ready. Press [Enter] to run script.".to_string(),
            ],
            script_running: false,
            script_path,
            should_quit: false,
            runner_state: RunnerState::FetchingReleases,
            event_tx: event_tx.clone(),
            onboarding,
        };

        RunnerManager::fetch_releases(event_tx);

        app
    }

    pub fn current_menu(&self) -> MenuItem {
        MenuItem::from_index(self.menu_index)
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        use crossterm::event::KeyCode;

        // 0. Intercept keys if onboarding is active
        if let Some(ref mut ob) = self.onboarding {
            match &mut ob.step {
                crate::onboarding::OnboardingStep::Welcome => {}
                crate::onboarding::OnboardingStep::ScanResult(res) => {
                    if res.has_missing() {
                        match key.code {
                            KeyCode::Left
                            | KeyCode::Right
                            | KeyCode::Tab
                            | KeyCode::Up
                            | KeyCode::Down => {
                                ob.selected_yes = !ob.selected_yes;
                            }
                            KeyCode::Enter => {
                                if ob.selected_yes {
                                    ob.step = crate::onboarding::OnboardingStep::Installing {
                                        logs: vec!["Starting installation...".to_string()],
                                        completed: false,
                                        success: false,
                                    };
                                    self.run_dependency_script();
                                } else {
                                    crate::onboarding::complete_onboarding();
                                    self.onboarding = None;
                                }
                            }
                            _ => {}
                        }
                    } else if key.code == KeyCode::Enter {
                        crate::onboarding::complete_onboarding();
                        self.onboarding = None;
                    }
                }
                crate::onboarding::OnboardingStep::Installing { completed, .. } => {
                    if *completed && key.code == KeyCode::Enter {
                        crate::onboarding::complete_onboarding();
                        self.onboarding = None;
                    }
                }
            }
            return;
        }

        // 1. Intercept keys if a popup input modal is active
        if let Some(ref mut active) = self.active_modal {
            match active {
                ActiveModal::CreatePrefix(ref mut modal) => match key.code {
                    KeyCode::Esc => {
                        self.active_modal = None;
                    }
                    KeyCode::Tab | KeyCode::Up | KeyCode::Down => {
                        modal.focus = match modal.focus {
                            ModalFocus::NameInput => ModalFocus::ArchToggle,
                            ModalFocus::ArchToggle => ModalFocus::NameInput,
                        };
                    }
                    KeyCode::Char(' ') => {
                        if modal.focus == ModalFocus::ArchToggle {
                            modal.is_64bit = !modal.is_64bit;
                        }
                    }
                    KeyCode::Char(c) => {
                        if modal.focus == ModalFocus::NameInput {
                            modal.name_input.push(c);
                        }
                    }
                    KeyCode::Backspace => {
                        if modal.focus == ModalFocus::NameInput {
                            modal.name_input.pop();
                        }
                    }
                    KeyCode::Enter if !modal.name_input.trim().is_empty() => {
                        let name = modal.name_input.trim().to_string();
                        let arch = if modal.is_64bit {
                            "win64".to_string()
                        } else {
                            "win32".to_string()
                        };

                        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
                        let path = std::path::PathBuf::from(home)
                            .join(".local/share/lgtui/prefixes")
                            .join(&name);

                        let new_prefix = WinePrefix {
                            name,
                            path,
                            architecture: arch,
                            status: "Not Initialized".to_string(),
                        };

                        if let Ok(conn) = crate::database::init_db() {
                            let _ = crate::database::save_prefix(&conn, &new_prefix);
                        }

                        self.custom_prefixes.push(new_prefix);
                        self.active_modal = None;
                    }
                    _ => {}
                },
                ActiveModal::AddGame(ref mut modal) => match key.code {
                    KeyCode::Esc => {
                        self.active_modal = None;
                    }
                    KeyCode::Tab | KeyCode::Down => {
                        modal.focus = match modal.focus {
                            AddGameFocus::Title => AddGameFocus::ExecPath,
                            AddGameFocus::ExecPath => AddGameFocus::PrefixName,
                            AddGameFocus::PrefixName => AddGameFocus::Runner,
                            AddGameFocus::Runner => AddGameFocus::Dxvk,
                            AddGameFocus::Dxvk => AddGameFocus::Vkd3d,
                            AddGameFocus::Vkd3d => AddGameFocus::MangoHud,
                            AddGameFocus::MangoHud => AddGameFocus::GameMode,
                            AddGameFocus::GameMode => AddGameFocus::IsInstaller,
                            AddGameFocus::IsInstaller => AddGameFocus::Title,
                        };
                    }
                    KeyCode::Up => {
                        modal.focus = match modal.focus {
                            AddGameFocus::Title => AddGameFocus::IsInstaller,
                            AddGameFocus::ExecPath => AddGameFocus::Title,
                            AddGameFocus::PrefixName => AddGameFocus::ExecPath,
                            AddGameFocus::Runner => AddGameFocus::PrefixName,
                            AddGameFocus::Dxvk => AddGameFocus::Runner,
                            AddGameFocus::Vkd3d => AddGameFocus::Dxvk,
                            AddGameFocus::MangoHud => AddGameFocus::Vkd3d,
                            AddGameFocus::GameMode => AddGameFocus::MangoHud,
                            AddGameFocus::IsInstaller => AddGameFocus::GameMode,
                        };
                    }
                    KeyCode::Left => match modal.focus {
                        AddGameFocus::Runner => {
                            let total = 1 + self.config.runners.len();
                            modal.selected_runner_idx =
                                (modal.selected_runner_idx + total - 1) % total;
                        }
                        _ => {}
                    },
                    KeyCode::Right => match modal.focus {
                        AddGameFocus::Runner => {
                            let total = 1 + self.config.runners.len();
                            modal.selected_runner_idx = (modal.selected_runner_idx + 1) % total;
                        }
                        _ => {}
                    },
                    KeyCode::Char(' ') => match modal.focus {
                        AddGameFocus::Runner => {
                            let total = 1 + self.config.runners.len();
                            modal.selected_runner_idx = (modal.selected_runner_idx + 1) % total;
                        }
                        AddGameFocus::Dxvk => modal.dxvk = !modal.dxvk,
                        AddGameFocus::Vkd3d => modal.vkd3d = !modal.vkd3d,
                        AddGameFocus::MangoHud => modal.mangohud = !modal.mangohud,
                        AddGameFocus::GameMode => modal.gamemode = !modal.gamemode,
                        AddGameFocus::IsInstaller => modal.is_installer = !modal.is_installer,
                        _ => {}
                    },
                    KeyCode::Char(c) => match modal.focus {
                        AddGameFocus::Title => {
                            modal.title.push(c);
                            if !modal.prefix_edited {
                                modal.prefix_name = slugify(&modal.title);
                            }
                        }
                        AddGameFocus::ExecPath => {
                            modal.exec_path.push(c);
                            let lower = modal.exec_path.to_lowercase();
                            if lower.ends_with("setup.exe") || lower.ends_with("install.exe") {
                                modal.is_installer = true;
                            }
                        }
                        AddGameFocus::PrefixName => {
                            modal.prefix_name.push(c);
                            modal.prefix_edited = true;
                        }
                        _ => {}
                    },
                    KeyCode::Backspace => match modal.focus {
                        AddGameFocus::Title => {
                            modal.title.pop();
                            if !modal.prefix_edited {
                                modal.prefix_name = slugify(&modal.title);
                            }
                        }
                        AddGameFocus::ExecPath => {
                            modal.exec_path.pop();
                            let lower = modal.exec_path.to_lowercase();
                            modal.is_installer =
                                lower.ends_with("setup.exe") || lower.ends_with("install.exe");
                        }
                        AddGameFocus::PrefixName => {
                            modal.prefix_name.pop();
                            modal.prefix_edited = true;
                        }
                        _ => {}
                    },
                    KeyCode::Enter => {
                        let title_valid = !modal.title.trim().is_empty();
                        let path_valid = validate_exec_path(&modal.exec_path);
                        if title_valid && path_valid {
                            let modal_clone = modal.clone();
                            self.add_game_from_modal(modal_clone);
                        }
                    }
                    _ => {}
                },
                ActiveModal::DeleteGame {
                    game_idx,
                    ref mut prompt_prefix,
                    ..
                } => {
                    let idx = *game_idx;
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                            self.active_modal = None;
                        }
                        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                            if !*prompt_prefix {
                                let game = &self.config.games[idx];
                                if let Some(ref prefix) = game.wineprefix {
                                    if prefix != &self.config.default_wineprefix {
                                        *prompt_prefix = true;
                                    } else {
                                        self.execute_game_deletion(idx, false);
                                    }
                                } else {
                                    self.execute_game_deletion(idx, false);
                                }
                            } else {
                                self.execute_game_deletion(idx, true);
                            }
                        }
                        _ => {}
                    }
                }
                ActiveModal::DeletePrefix {
                    prefix_idx,
                    delete_files: _,
                    prompt_delete_files,
                } => {
                    let idx = *prefix_idx;
                    match key.code {
                        KeyCode::Esc => {
                            self.active_modal = None;
                        }
                        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                            if !*prompt_delete_files {
                                let prefix = &self.custom_prefixes[idx];
                                if prefix.path.exists() && prefix.path.is_dir() {
                                    *prompt_delete_files = true;
                                } else {
                                    self.execute_prefix_deletion(idx, false);
                                }
                            } else {
                                self.execute_prefix_deletion(idx, true);
                            }
                        }
                        KeyCode::Char('n') | KeyCode::Char('N') => {
                            if !*prompt_delete_files {
                                self.active_modal = None;
                            } else {
                                self.execute_prefix_deletion(idx, false);
                            }
                        }
                        _ => {}
                    }
                }
                ActiveModal::Help => {
                    self.active_modal = None;
                }
            }
            return;
        }

        // Global keybindings (when modal is NOT active)
        if self.active_modal.is_none() {
            match key.code {
                KeyCode::Char('q') => {
                    self.should_quit = true;
                    return;
                }
                KeyCode::Char('a') => {
                    self.active_modal = Some(ActiveModal::AddGame(AddGameModalState {
                        title: String::new(),
                        exec_path: String::new(),
                        prefix_name: String::new(),
                        prefix_edited: false,
                        selected_runner_idx: 0,
                        dxvk: true,
                        vkd3d: false,
                        mangohud: false,
                        gamemode: false,
                        is_installer: false,
                        focus: AddGameFocus::Title,
                    }));
                    return;
                }
                KeyCode::Char('i') => {
                    self.menu_index = MenuItem::DependencyInstaller as usize;
                    self.focus = PaneFocus::LibraryList;
                    self.run_dependency_script();
                    return;
                }
                KeyCode::Char('?') => {
                    self.active_modal = Some(ActiveModal::Help);
                    return;
                }
                KeyCode::Char('x') => {
                    if self.current_menu() == MenuItem::Library && !self.config.games.is_empty() {
                        self.active_modal = Some(ActiveModal::DeleteGame {
                            game_idx: self.selected_game_idx,
                            delete_prefix: false,
                            prompt_prefix: false,
                        });
                    } else if self.current_menu() == MenuItem::PrefixManager
                        && !self.custom_prefixes.is_empty()
                    {
                        self.active_modal = Some(ActiveModal::DeletePrefix {
                            prefix_idx: self.selected_prefix_idx,
                            delete_files: false,
                            prompt_delete_files: false,
                        });
                    }
                    return;
                }
                KeyCode::Char('n') => {
                    self.active_modal = Some(ActiveModal::CreatePrefix(ModalState {
                        name_input: String::new(),
                        is_64bit: true,
                        focus: ModalFocus::NameInput,
                    }));
                    return;
                }
                KeyCode::Tab => {
                    self.focus = match self.focus {
                        PaneFocus::LeftMenu => PaneFocus::LibraryList,
                        PaneFocus::LibraryList => {
                            if self.current_menu() == MenuItem::Library {
                                PaneFocus::MetadataPanel
                            } else {
                                PaneFocus::LeftMenu
                            }
                        }
                        PaneFocus::MetadataPanel => PaneFocus::LeftMenu,
                    };
                    return;
                }
                _ => {}
            }
        }

        match self.focus {
            PaneFocus::LeftMenu => match key.code {
                KeyCode::Up | KeyCode::Char('k') => {
                    if self.menu_index > 0 {
                        self.menu_index -= 1;
                    } else {
                        self.menu_index = 5; // Wrap around (6 items)
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if self.menu_index < 5 {
                        self.menu_index += 1;
                    } else {
                        self.menu_index = 0; // Wrap around (6 items)
                    }
                }
                KeyCode::Right | KeyCode::Char('l') => {
                    self.focus = PaneFocus::LibraryList;
                }
                _ => {}
            },
            PaneFocus::LibraryList => match self.current_menu() {
                MenuItem::Library => match key.code {
                    KeyCode::Left | KeyCode::Char('h') => {
                        self.focus = PaneFocus::LeftMenu;
                    }
                    KeyCode::Right | KeyCode::Char('l') => {
                        self.focus = PaneFocus::MetadataPanel;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if !self.config.games.is_empty() {
                            if self.selected_game_idx > 0 {
                                self.selected_game_idx -= 1;
                            } else {
                                self.selected_game_idx = self.config.games.len() - 1;
                            }
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if !self.config.games.is_empty() {
                            if self.selected_game_idx < self.config.games.len() - 1 {
                                self.selected_game_idx += 1;
                            } else {
                                self.selected_game_idx = 0;
                            }
                        }
                    }
                    KeyCode::Enter => {
                        self.launch_selected_game();
                    }
                    _ => {}
                },
                MenuItem::RunnerManager => match key.code {
                    KeyCode::Left | KeyCode::Char('h') => {
                        self.focus = PaneFocus::LeftMenu;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if !self.config.runners.is_empty() {
                            if self.selected_runner_idx > 0 {
                                self.selected_runner_idx -= 1;
                            } else {
                                self.selected_runner_idx = self.config.runners.len() - 1;
                            }
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if !self.config.runners.is_empty() {
                            if self.selected_runner_idx < self.config.runners.len() - 1 {
                                self.selected_runner_idx += 1;
                            } else {
                                self.selected_runner_idx = 0;
                            }
                        }
                    }
                    KeyCode::Enter => {
                        self.download_selected_runner();
                    }
                    _ => {}
                },
                MenuItem::PrefixManager => match key.code {
                    KeyCode::Left | KeyCode::Char('h') => {
                        self.focus = PaneFocus::LeftMenu;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if !self.custom_prefixes.is_empty() {
                            if self.selected_prefix_idx > 0 {
                                self.selected_prefix_idx -= 1;
                            } else {
                                self.selected_prefix_idx = self.custom_prefixes.len() - 1;
                            }
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if !self.custom_prefixes.is_empty() {
                            if self.selected_prefix_idx < self.custom_prefixes.len() - 1 {
                                self.selected_prefix_idx += 1;
                            } else {
                                self.selected_prefix_idx = 0;
                            }
                        }
                    }
                    KeyCode::Enter => {
                        self.initialize_custom_prefix(self.selected_prefix_idx);
                    }
                    _ => {}
                },
                MenuItem::Winetricks => match key.code {
                    KeyCode::Left | KeyCode::Char('h') => {
                        self.focus = PaneFocus::LeftMenu;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        if self.selected_winetricks_idx > 0 {
                            self.selected_winetricks_idx -= 1;
                        } else {
                            self.selected_winetricks_idx = self.winetricks_items.len() - 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        if self.selected_winetricks_idx < self.winetricks_items.len() - 1 {
                            self.selected_winetricks_idx += 1;
                        } else {
                            self.selected_winetricks_idx = 0;
                        }
                    }
                    KeyCode::Enter => {
                        self.run_selected_winetricks();
                    }
                    _ => {}
                },
                MenuItem::DependencyInstaller => match key.code {
                    KeyCode::Left | KeyCode::Char('h') => {
                        self.focus = PaneFocus::LeftMenu;
                    }
                    KeyCode::Enter => {
                        self.run_dependency_script();
                    }
                    _ => {}
                },
                MenuItem::Settings => match key.code {
                    KeyCode::Left | KeyCode::Char('h') => {
                        self.focus = PaneFocus::LeftMenu;
                    }
                    KeyCode::Up | KeyCode::Char('k') => {
                        let total_settings = self.settings_items.len();
                        let total_prefixes = self.custom_prefixes.len();
                        let total_items = total_settings + total_prefixes;

                        if total_items > 0 {
                            if self.selected_settings_idx > 0 {
                                self.selected_settings_idx -= 1;
                            } else {
                                self.selected_settings_idx = total_items - 1;
                            }
                        }
                    }
                    KeyCode::Down | KeyCode::Char('j') => {
                        let total_settings = self.settings_items.len();
                        let total_prefixes = self.custom_prefixes.len();
                        let total_items = total_settings + total_prefixes;

                        if total_items > 0 {
                            if self.selected_settings_idx < total_items - 1 {
                                self.selected_settings_idx += 1;
                            } else {
                                self.selected_settings_idx = 0;
                            }
                        }
                    }
                    KeyCode::Enter => {
                        let total_settings = self.settings_items.len();
                        if self.selected_settings_idx >= total_settings {
                            let prefix_idx = self.selected_settings_idx - total_settings;
                            self.initialize_custom_prefix(prefix_idx);
                        }
                    }
                    KeyCode::Char(' ') => {
                        let total_settings = self.settings_items.len();
                        if self.selected_settings_idx < total_settings {
                            self.toggle_selected_setting();
                        }
                    }
                    _ => {}
                },
            },
            PaneFocus::MetadataPanel => {
                if self.current_menu() == MenuItem::Library {
                    match key.code {
                        KeyCode::Left | KeyCode::Char('h') => {
                            self.focus = PaneFocus::LibraryList;
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            if self.selected_metadata_idx > 0 {
                                self.selected_metadata_idx -= 1;
                            } else {
                                self.selected_metadata_idx = 3;
                            }
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            if self.selected_metadata_idx < 3 {
                                self.selected_metadata_idx += 1;
                            } else {
                                self.selected_metadata_idx = 0;
                            }
                        }
                        KeyCode::Char(' ') => {
                            if let Some(game) = self.config.games.get_mut(self.selected_game_idx) {
                                match self.selected_metadata_idx {
                                    0 => game.dxvk = !game.dxvk,
                                    1 => game.vkd3d = !game.vkd3d,
                                    2 => game.mangohud = !game.mangohud,
                                    3 => game.gamemode = !game.gamemode,
                                    _ => {}
                                }
                                let _ = game.save_toml();
                                let _ = self.config.save();
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    pub fn tick(&mut self) {
        // Increment playtime for running games
        for game in &mut self.config.games {
            if let GameStatus::Running { .. } = game.status {
                game.playtime_secs += 1;
            }
        }

        // Process winetricks timer
        if self.winetricks_running {
            if let Some(timer) = self.winetricks_timer {
                if timer > 1 {
                    self.winetricks_timer = Some(timer - 1);
                    if timer % 2 == 0 {
                        self.winetricks_logs
                            .push(format!("Applying config override: block-{}...", timer));
                    }
                } else {
                    self.winetricks_running = false;
                    self.winetricks_timer = None;
                    let active_verb = self.winetricks_items[self.selected_winetricks_idx].0;
                    self.winetricks_logs.push(format!(
                        "Winetricks: Verb '{}' applied successfully to prefix.",
                        active_verb
                    ));
                }
            }
        }
    }

    // Action Handlers

    fn launch_selected_game(&mut self) {
        if self.config.games.is_empty() {
            return;
        }

        let game = &self.config.games[self.selected_game_idx];

        if let GameStatus::Running { .. } = game.status {
            return;
        }

        let runner = game
            .runner_id
            .as_ref()
            .and_then(|r_id| self.config.runners.iter().find(|r| r.id == *r_id).cloned());

        RunnerManager::launch(game.clone(), runner, self.event_tx.clone());
    }

    fn download_selected_runner(&mut self) {
        if self.config.runners.is_empty() {
            return;
        }

        let runner = &self.config.runners[self.selected_runner_idx];
        if runner.installed {
            return;
        }

        if let RunnerState::Downloading(_) | RunnerState::Extracting = self.runner_state {
            return;
        }

        let url = match &runner.download_url {
            Some(u) => u.clone(),
            None => {
                self.runner_state =
                    RunnerState::Error("No download URL available for this runner".to_string());
                return;
            }
        };

        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let dest_dir = std::path::PathBuf::from(home)
            .join(".local/share/steam/compatibilitytools.d")
            .join(&runner.version);

        self.downloading_runner_id = Some(runner.id.clone());
        self.download_progress = 0;
        self.runner_state = RunnerState::Downloading(0);

        RunnerManager::download_and_extract_runner(
            runner.id.clone(),
            url,
            dest_dir,
            self.event_tx.clone(),
        );
    }

    fn run_selected_winetricks(&mut self) {
        if self.winetricks_running {
            return;
        }

        if self.config.games.is_empty() {
            self.winetricks_logs
                .push("Winetricks Error: No games in library to configure.".to_string());
            return;
        }

        let game = &self.config.games[self.selected_game_idx];
        let verb = self.winetricks_items[self.selected_winetricks_idx].0;
        let prefix = game
            .wineprefix
            .as_deref()
            .unwrap_or(&self.config.default_wineprefix);

        self.winetricks_running = true;
        self.winetricks_timer = Some(5);
        self.winetricks_logs.push(format!(
            "Executing: winetricks {} (Prefix: {})",
            verb, prefix
        ));
    }

    fn toggle_selected_setting(&mut self) {
        if self.selected_settings_idx >= self.settings_items.len() {
            return;
        }

        let item = &mut self.settings_items[self.selected_settings_idx];
        if item.2 {
            if item.1 == "Enabled" {
                item.1 = "Disabled".to_string();
            } else {
                item.1 = "Enabled".to_string();
            }
        }
    }

    fn initialize_custom_prefix(&mut self, idx: usize) {
        if idx >= self.custom_prefixes.len() {
            return;
        }

        let prefix = &self.custom_prefixes[idx];
        if prefix.status == "Booting..." {
            return;
        }

        // Find standard wine path, or fall back to /usr/bin/wine
        let wine_path = self
            .config
            .runners
            .iter()
            .find(|r| r.installed && r.path.contains("wine"))
            .map(|r| std::path::PathBuf::from(&r.path))
            .unwrap_or_else(|| std::path::PathBuf::from("/usr/bin/wine"));

        initialize_prefix(prefix.clone(), wine_path, self.event_tx.clone());
    }

    fn run_dependency_script(&mut self) {
        if self.script_running {
            return;
        }

        self.script_running = true;
        self.script_logs.clear();
        self.script_logs
            .push("Executing: sh lgtui_install_deps.sh".to_string());

        RunnerManager::run_dependency_script(self.script_path.clone(), self.event_tx.clone());
    }

    fn add_game_from_modal(&mut self, state: AddGameModalState) {
        let game_id = slugify(&state.title);
        let mut final_id = game_id.clone();
        let mut count = 1;
        while self.config.games.iter().any(|g| g.id == final_id) {
            final_id = format!("{}-{}", game_id, count);
            count += 1;
        }

        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        let base_dir = std::path::PathBuf::from(&home).join(".local/share/lgtui");
        let wineprefix = if state.prefix_name.trim().is_empty() {
            None
        } else {
            Some(
                base_dir
                    .join("prefixes")
                    .join(state.prefix_name.trim())
                    .to_string_lossy()
                    .to_string(),
            )
        };

        let runner_id = if state.selected_runner_idx == 0 {
            None
        } else {
            self.config
                .runners
                .get(state.selected_runner_idx - 1)
                .map(|r| r.id.clone())
        };

        let new_game = crate::game::Game {
            id: final_id,
            name: state.title.trim().to_string(),
            exec_path: state.exec_path.trim().to_string(),
            args: Vec::new(),
            wineprefix,
            runner_id,
            playtime_secs: 0,
            dxvk: state.dxvk,
            vkd3d: state.vkd3d,
            mangohud: state.mangohud,
            gamemode: state.gamemode,
            is_installer: state.is_installer,
            status: GameStatus::Ready,
        };

        let _ = new_game.save_toml();

        self.config.games.push(new_game);
        let _ = self.config.save();
        self.active_modal = None;
        self.focus = PaneFocus::LibraryList;
    }

    fn execute_game_deletion(&mut self, game_idx: usize, delete_prefix_dir: bool) {
        if game_idx >= self.config.games.len() {
            self.active_modal = None;
            return;
        }

        let game = self.config.games.remove(game_idx);
        let _ = game.delete_toml();
        let _ = self.config.save();

        if delete_prefix_dir {
            if let Some(prefix_path_str) = game.wineprefix {
                let prefix_path = std::path::PathBuf::from(prefix_path_str);
                tokio::spawn(async move {
                    if prefix_path.exists() && prefix_path.is_dir() {
                        let _ = tokio::fs::remove_dir_all(prefix_path).await;
                    }
                });
            }
        }

        if self.selected_game_idx >= self.config.games.len() && !self.config.games.is_empty() {
            self.selected_game_idx = self.config.games.len() - 1;
        } else if self.config.games.is_empty() {
            self.selected_game_idx = 0;
        }

        self.active_modal = None;
        self.focus = PaneFocus::LibraryList;
    }

    fn execute_prefix_deletion(&mut self, prefix_idx: usize, delete_prefix_dir: bool) {
        if prefix_idx >= self.custom_prefixes.len() {
            self.active_modal = None;
            return;
        }

        let prefix = self.custom_prefixes.remove(prefix_idx);

        if let Ok(conn) = crate::database::init_db() {
            let _ = crate::database::delete_prefix(&conn, &prefix.name);
        }

        if delete_prefix_dir {
            let path = prefix.path.clone();
            tokio::spawn(async move {
                if path.exists() && path.is_dir() {
                    let _ = tokio::fs::remove_dir_all(path).await;
                }
            });
        }

        if self.selected_prefix_idx >= self.custom_prefixes.len()
            && !self.custom_prefixes.is_empty()
        {
            self.selected_prefix_idx = self.custom_prefixes.len() - 1;
        } else if self.custom_prefixes.is_empty() {
            self.selected_prefix_idx = 0;
        }

        self.active_modal = None;
        self.focus = PaneFocus::LibraryList;
    }
}

fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn validate_exec_path(path: &str) -> bool {
    if path.trim().is_empty() {
        return false;
    }
    let invalid_chars = ['*', '?', '"', '<', '>', '|'];
    !path.chars().any(|c| invalid_chars.contains(&c))
}
