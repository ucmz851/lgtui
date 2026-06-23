use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticsResult {
    pub wine: bool,
    pub winetricks: bool,
    pub mangohud: bool,
    pub gamemode: bool,
}

impl DiagnosticsResult {
    pub fn perform() -> Self {
        DiagnosticsResult {
            wine: has_command("wine"),
            winetricks: has_command("winetricks"),
            mangohud: has_command("mangohud"),
            gamemode: has_command("gamemoded") || has_command("gamemoderun"),
        }
    }

    pub fn has_missing(&self) -> bool {
        !self.wine || !self.winetricks || !self.mangohud || !self.gamemode
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnboardingStep {
    Welcome,
    ScanResult(DiagnosticsResult),
    Installing {
        logs: Vec<String>,
        completed: bool,
        success: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnboardingState {
    pub step: OnboardingStep,
    pub selected_yes: bool,
}

fn has_command(cmd: &str) -> bool {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("command -v {}", cmd))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn needs_onboarding() -> bool {
    false
}

pub fn complete_onboarding() {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let path = PathBuf::from(home).join(".config/lgui/config.toml");
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(path, "onboarding_completed = true\n");
}
