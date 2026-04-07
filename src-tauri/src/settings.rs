use serde::{Deserialize, Serialize};
use std::fs;
use tauri::{AppHandle, Manager, Runtime};

const SETTINGS_FILE_NAME: &str = "terminal-settings.json";

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TerminalPreference {
    Auto,
    Terminal,
    Iterm,
    Ghostty,
    Tabby,
    Custom,
}

impl Default for TerminalPreference {
    fn default() -> Self {
        Self::Auto
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSettings {
    pub preferred_terminal: TerminalPreference,
    pub custom_command: String,
}

impl Default for TerminalSettings {
    fn default() -> Self {
        Self {
            preferred_terminal: TerminalPreference::Auto,
            custom_command: String::new(),
        }
    }
}

impl TerminalSettings {
    pub fn validated(mut self) -> Result<Self, String> {
        self.custom_command = self.custom_command.trim().to_string();

        if self.preferred_terminal == TerminalPreference::Custom && self.custom_command.is_empty() {
            return Err(
                "Custom command is required when the preferred terminal is set to Custom."
                    .to_string(),
            );
        }

        Ok(self)
    }

    pub fn preferred_terminal_label(&self) -> &'static str {
        match self.preferred_terminal {
            TerminalPreference::Auto => "Auto",
            TerminalPreference::Terminal => "Terminal",
            TerminalPreference::Iterm => "iTerm",
            TerminalPreference::Ghostty => "Ghostty",
            TerminalPreference::Tabby => "Tabby",
            TerminalPreference::Custom => "Custom command",
        }
    }
}

pub fn load<R: Runtime>(app: &AppHandle<R>) -> Result<TerminalSettings, String> {
    let settings_path = settings_path(app)?;
    if !settings_path.exists() {
        return Ok(TerminalSettings::default());
    }

    let contents = fs::read_to_string(&settings_path)
        .map_err(|error| format!("failed to read terminal settings: {error}"))?;
    let settings: TerminalSettings = serde_json::from_str(&contents)
        .map_err(|error| format!("failed to parse terminal settings: {error}"))?;

    settings.validated()
}

pub fn save<R: Runtime>(app: &AppHandle<R>, settings: TerminalSettings) -> Result<TerminalSettings, String> {
    let settings = settings.validated()?;
    let settings_path = settings_path(app)?;
    let payload = serde_json::to_string_pretty(&settings)
        .map_err(|error| format!("failed to serialize terminal settings: {error}"))?;

    fs::write(&settings_path, payload)
        .map_err(|error| format!("failed to write terminal settings: {error}"))?;

    Ok(settings)
}

fn settings_path<R: Runtime>(app: &AppHandle<R>) -> Result<std::path::PathBuf, String> {
    let app_config_dir = app
        .path()
        .app_config_dir()
        .map_err(|error| format!("failed to resolve app config directory: {error}"))?;

    fs::create_dir_all(&app_config_dir)
        .map_err(|error| format!("failed to create app config directory: {error}"))?;

    Ok(app_config_dir.join(SETTINGS_FILE_NAME))
}
