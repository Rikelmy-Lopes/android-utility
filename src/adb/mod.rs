//! ADB integration: process invocation, output parsing, and device operations.
//!
//! Process execution stays inside this module. Parsed values are handed to the
//! application layer as domain types.

mod error;
mod parse;
mod process;

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use android_utility_core::{InstalledApp, PackageName, sort_apps};

pub use error::AdbError;
pub use process::ProcessOutput;

use parse::{
    merge_package_paths, parse_devices, parse_package_entries, parse_package_names,
    permission_denied, pm_failure, uninstall_succeeded,
};

/// ADB operations required by application commands.
///
/// Implementations are substituted in tests so that no device or process is
/// needed.
pub trait DeviceService: Send + Sync {
    /// Lists devices reported by the ADB server.
    fn devices(&self) -> Result<Vec<Device>, AdbError>;

    /// Lists installed packages with system and disabled state.
    fn list_apps(&self, serial: &str) -> Result<Vec<InstalledApp>, AdbError>;

    /// Reads the application label from the APK at `apk_path`.
    fn application_label(&self, serial: &str, apk_path: &str) -> Result<Option<String>, AdbError>;

    /// Disables a package for the current user.
    fn disable_app(&self, serial: &str, package: &PackageName) -> Result<(), AdbError>;

    /// Re-enables a disabled package.
    fn enable_app(&self, serial: &str, package: &PackageName) -> Result<(), AdbError>;

    /// Uninstalls a package, optionally keeping data for the current user only.
    fn uninstall_app(
        &self,
        serial: &str,
        package: &PackageName,
        user_only: bool,
    ) -> Result<(), AdbError>;
}

/// Connection state of a device reported by ADB.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceState {
    /// Ready for commands.
    Online,
    /// Connected but not responding.
    Offline,
    /// Connected but not authorized for debugging.
    Unauthorized,
    /// USB access needs host permission changes.
    NoPermissions,
    /// Unrecognized state reported by ADB.
    Unknown,
}

/// A device reported by the ADB server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Device {
    /// Serial or emulator identifier.
    pub serial: String,
    /// Reported connection state.
    pub state: DeviceState,
    /// Device model, when reported.
    pub model: Option<String>,
}

impl Device {
    /// Returns the model name when known, otherwise the serial.
    pub fn name(&self) -> &str {
        match self.model.as_deref() {
            Some(model) if !model.trim().is_empty() => model,
            _ => &self.serial,
        }
    }

    /// Returns `true` when commands can be sent to the device.
    pub fn is_online(&self) -> bool {
        self.state == DeviceState::Online
    }
}

/// Default timeout for short ADB commands.
const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(20);

/// Timeout for pulling an APK from the device.
const APK_PULL_TIMEOUT: Duration = Duration::from_secs(120);

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// ADB client that invokes the `adb` executable.
#[derive(Debug, Clone)]
pub struct Adb {
    program: String,
    command_timeout: Duration,
    pull_timeout: Duration,
    cache_dir: Option<PathBuf>,
}

impl Default for Adb {
    fn default() -> Self {
        Self::new()
    }
}

impl Adb {
    /// Creates a client that runs `adb` from the process search path.
    pub fn new() -> Self {
        Self {
            program: "adb".to_owned(),
            command_timeout: DEFAULT_COMMAND_TIMEOUT,
            pull_timeout: APK_PULL_TIMEOUT,
            cache_dir: None,
        }
    }

    /// Stores pulled APKs in `directory` instead of the system temporary directory.
    pub fn with_cache_dir(mut self, directory: PathBuf) -> Self {
        self.cache_dir = Some(directory);
        self
    }

    fn run(&self, arguments: &[String], timeout: Duration) -> Result<ProcessOutput, AdbError> {
        process::run(&self.program, arguments, timeout)
    }

    fn shell(&self, serial: &str, command: &[&str]) -> Result<ProcessOutput, AdbError> {
        let mut arguments = vec!["-s".to_owned(), serial.to_owned(), "shell".to_owned()];
        arguments.extend(command.iter().map(|part| (*part).to_owned()));

        self.run(&arguments, self.command_timeout)
    }

    fn ensure_device_ready(&self, serial: &str) -> Result<(), AdbError> {
        let output = self.run(
            &["-s".to_owned(), serial.to_owned(), "get-state".to_owned()],
            self.command_timeout,
        )?;

        if output.succeeded() && output.stdout.trim() == "device" {
            Ok(())
        } else {
            Err(AdbError::command_failed(output.diagnostics()))
        }
    }

    fn temporary_apk_path(&self) -> Result<PathBuf, AdbError> {
        let sequence = TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let file_name = format!("android-utility-{}-{sequence}.apk", std::process::id());

        let directory = match &self.cache_dir {
            Some(directory) => {
                fs::create_dir_all(directory).map_err(|error| AdbError::Io {
                    message: error.to_string(),
                })?;
                directory.clone()
            }
            None => std::env::temp_dir(),
        };

        Ok(directory.join(file_name))
    }
}

impl DeviceService for Adb {
    fn devices(&self) -> Result<Vec<Device>, AdbError> {
        let output = self.run(
            &["devices".to_owned(), "-l".to_owned()],
            self.command_timeout,
        )?;

        if !output.succeeded() {
            return Err(AdbError::command_failed(output.diagnostics()));
        }

        Ok(parse_devices(&output.stdout)
            .into_iter()
            .map(|entry| Device {
                serial: entry.serial,
                state: entry.state,
                model: entry.model,
            })
            .collect())
    }

    fn list_apps(&self, serial: &str) -> Result<Vec<InstalledApp>, AdbError> {
        let all = self.shell(serial, &["pm", "list", "packages", "-f"])?;
        let system = self.shell(serial, &["pm", "list", "packages", "-s"])?;
        let disabled = self.shell(serial, &["pm", "list", "packages", "-d"])?;

        for output in [&all, &system, &disabled] {
            if let Some(message) = pm_failure(output) {
                return Err(AdbError::command_failed(message));
            }
        }

        let system_packages = parse_package_names(&system.stdout);
        let disabled_packages: HashSet<PackageName> = parse_package_names(&disabled.stdout);

        let mut apps: Vec<InstalledApp> = merge_package_paths(&parse_package_entries(&all.stdout))
            .into_iter()
            .map(|(package, apk_path)| InstalledApp {
                is_system: system_packages.contains(&package),
                is_disabled: disabled_packages.contains(&package),
                package,
                label: None,
                apk_path,
            })
            .collect();

        sort_apps(&mut apps);
        Ok(apps)
    }

    fn application_label(&self, serial: &str, apk_path: &str) -> Result<Option<String>, AdbError> {
        let local_path = self.temporary_apk_path()?;
        let local_argument = local_path.to_string_lossy().into_owned();

        let arguments = vec![
            "-s".to_owned(),
            serial.to_owned(),
            "pull".to_owned(),
            apk_path.to_owned(),
            local_argument,
        ];

        let pull_result = self.run(&arguments, self.pull_timeout);
        let label: Result<Option<String>, AdbError> = match pull_result {
            Ok(output) if output.succeeded() => Ok(apk_info::Apk::new(&local_path)
                .ok()
                .and_then(|apk| apk.get_application_label())),
            Ok(output) => Err(AdbError::command_failed(output.diagnostics())),
            Err(error) => Err(error),
        };

        let _ = fs::remove_file(&local_path);

        label.map(|value| {
            value
                .map(|text| text.trim().to_owned())
                .filter(|text| !text.is_empty())
        })
    }

    fn disable_app(&self, serial: &str, package: &PackageName) -> Result<(), AdbError> {
        self.ensure_device_ready(serial)?;

        let output = self.shell(
            serial,
            &["pm", "disable-user", "--user", "0", package.as_str()],
        )?;

        match pm_failure(&output) {
            Some(_) => Err(pm_error(&output)),
            None => Ok(()),
        }
    }

    fn enable_app(&self, serial: &str, package: &PackageName) -> Result<(), AdbError> {
        self.ensure_device_ready(serial)?;

        let output = self.shell(serial, &["pm", "enable", package.as_str()])?;

        match pm_failure(&output) {
            Some(_) => Err(pm_error(&output)),
            None => Ok(()),
        }
    }

    fn uninstall_app(
        &self,
        serial: &str,
        package: &PackageName,
        user_only: bool,
    ) -> Result<(), AdbError> {
        self.ensure_device_ready(serial)?;

        let mut command = vec!["pm", "uninstall"];
        if user_only {
            command.extend(["--user", "0"]);
        }
        command.push(package.as_str());

        let output = self.shell(serial, &command)?;

        if uninstall_succeeded(&output) {
            Ok(())
        } else {
            Err(pm_error(&output))
        }
    }
}

fn pm_error(output: &ProcessOutput) -> AdbError {
    let message = pm_failure(output).unwrap_or_else(|| {
        let diagnostics = output.diagnostics();
        if diagnostics.is_empty() {
            "command did not report success".to_owned()
        } else {
            diagnostics
        }
    });

    if permission_denied(output) {
        AdbError::PermissionDenied { message }
    } else {
        AdbError::command_failed(message)
    }
}
