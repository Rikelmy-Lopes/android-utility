//! Application startup helpers: per-user data locations.

pub mod labels;
pub mod settings;

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// Directory created below the platform application-data location.
const APPLICATION_DIRECTORY: &str = "Android Utility";

/// Per-user directories used for configuration, logs, and caches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    data_dir: PathBuf,
}

impl AppPaths {
    /// Resolves the per-user application data directory for the current platform.
    ///
    /// On Windows this is `%LOCALAPPDATA%`; on Linux it is
    /// `$XDG_DATA_HOME` or `$HOME/.local/share`.
    pub fn for_current_user() -> Result<Self, PathError> {
        Ok(Self::under(platform_data_directory()?))
    }

    fn under(base: PathBuf) -> Self {
        Self {
            data_dir: base.join(APPLICATION_DIRECTORY),
        }
    }

    /// Root directory for all application-owned files.
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    /// Directory for cached data such as pulled APKs.
    pub fn cache_dir(&self) -> PathBuf {
        self.data_dir().join("cache")
    }

    /// Directory for diagnostic log files.
    pub fn log_dir(&self) -> PathBuf {
        self.data_dir().join("logs")
    }

    /// File storing user preferences.
    pub fn settings_file(&self) -> PathBuf {
        self.data_dir().join("settings.json")
    }

    /// Creates the cache directory and returns it.
    pub fn ensure_cache_dir(&self) -> Result<PathBuf, PathError> {
        let directory = self.cache_dir();
        fs::create_dir_all(&directory).map_err(|error| PathError::Io {
            message: error.to_string(),
        })?;
        Ok(directory)
    }

    /// Creates the log directory and returns it.
    pub fn ensure_log_dir(&self) -> Result<PathBuf, PathError> {
        let directory = self.log_dir();
        fs::create_dir_all(&directory).map_err(|error| PathError::Io {
            message: error.to_string(),
        })?;
        Ok(directory)
    }
}

/// Failure while resolving or creating application directories.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathError {
    /// A required environment variable is not set.
    EnvironmentNotSet(&'static str),
    /// The operating system has no defined data directory.
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    UnsupportedPlatform,
    /// The directory could not be created.
    Io {
        /// Operating-system error message.
        message: String,
    },
}

impl fmt::Display for PathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EnvironmentNotSet(name) => {
                write!(formatter, "environment variable {name} is not set")
            }
            #[cfg(not(any(target_os = "windows", target_os = "linux")))]
            Self::UnsupportedPlatform => {
                formatter.write_str("per-user data directory is not supported on this platform")
            }
            Self::Io { message } => write!(formatter, "could not create directory: {message}"),
        }
    }
}

impl std::error::Error for PathError {}

#[cfg(target_os = "windows")]
fn platform_data_directory() -> Result<PathBuf, PathError> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or(PathError::EnvironmentNotSet("LOCALAPPDATA"))
}

#[cfg(target_os = "linux")]
fn platform_data_directory() -> Result<PathBuf, PathError> {
    if let Some(value) = std::env::var_os("XDG_DATA_HOME") {
        return Ok(PathBuf::from(value));
    }

    std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(".local/share"))
        .ok_or(PathError::EnvironmentNotSet("HOME"))
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn platform_data_directory() -> Result<PathBuf, PathError> {
    Err(PathError::UnsupportedPlatform)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places_cache_below_application_directory() {
        let paths = AppPaths::under(PathBuf::from("/base"));
        assert_eq!(
            paths.data_dir(),
            Path::new("/base").join(APPLICATION_DIRECTORY)
        );
        assert_eq!(paths.cache_dir(), paths.data_dir().join("cache"));
        assert_eq!(paths.log_dir(), paths.data_dir().join("logs"));
        assert_eq!(
            paths.settings_file(),
            paths.data_dir().join("settings.json")
        );
    }

    #[test]
    fn reports_missing_environment_variable() {
        let error = PathError::EnvironmentNotSet("LOCALAPPDATA");
        assert!(error.to_string().contains("LOCALAPPDATA"));
    }
}
