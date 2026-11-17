//! Persisted user preferences.
//!
//! Settings are stored as JSON below the per-user application data directory.
//! Missing or unreadable settings fall back to the defaults.

use std::fmt;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Color theme selected by the user.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    /// Dark background with light text.
    #[default]
    Dark,
    /// Light background with dark text.
    Light,
}

/// User preferences persisted between runs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Selected color theme.
    pub theme: ThemeMode,
}

impl Settings {
    /// Loads settings from `path`, falling back to the defaults on failure.
    pub fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .ok()
            .and_then(|source| serde_json::from_str(&source).ok())
            .unwrap_or_default()
    }

    /// Writes the settings to `path`, creating the parent directory if needed.
    pub fn save(&self, path: &Path) -> Result<(), SettingsError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| SettingsError {
                message: error.to_string(),
            })?;
        }

        let source = serde_json::to_string_pretty(self).map_err(|error| SettingsError {
            message: error.to_string(),
        })?;

        fs::write(path, source).map_err(|error| SettingsError {
            message: error.to_string(),
        })
    }
}

/// Failure while saving user settings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsError {
    /// Operating-system or serialization error message.
    pub message: String,
}

impl fmt::Display for SettingsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "could not save settings: {}", self.message)
    }
}

impl std::error::Error for SettingsError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_use_the_dark_theme() {
        assert_eq!(Settings::default().theme, ThemeMode::Dark);
    }

    #[test]
    fn missing_settings_file_falls_back_to_defaults() {
        let path = Path::new("this-file-does-not-exist").join("settings.json");
        assert_eq!(Settings::load(&path), Settings::default());
    }

    #[test]
    fn malformed_settings_fall_back_to_defaults() {
        let directory = std::env::temp_dir().join("android-utility-settings-test");
        fs::create_dir_all(&directory).expect("create temp directory");
        let path = directory.join("settings.json");
        fs::write(&path, "not valid json").expect("write settings");

        assert_eq!(Settings::load(&path), Settings::default());

        let _ = fs::remove_file(&path);
    }

    #[test]
    fn saved_settings_round_trip() {
        let directory = std::env::temp_dir().join("android-utility-settings-round-trip");
        let path = directory.join("settings.json");
        let settings = Settings {
            theme: ThemeMode::Light,
        };

        settings.save(&path).expect("save settings");
        assert_eq!(Settings::load(&path), settings);

        let _ = fs::remove_dir_all(&directory);
    }
}
