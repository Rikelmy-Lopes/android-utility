//! ADB client that invokes the `adb` executable.

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use android_utility_core::{
    BackgroundExecution, DozeWhitelistEntry, InstalledApp, PackageName, ProcessUsage, sort_apps,
};

use super::parse::{
    merge_package_paths, parse_devices, parse_doze_whitelist, parse_ignored_background_packages,
    parse_package_entries, parse_package_names, parse_top_processes, permission_denied, pm_failure,
    uninstall_succeeded, unsupported_command,
};
use super::process::ProcessOutput;
use super::{AdbError, Device, DeviceService};

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
        super::process::run(&self.program, arguments, timeout)
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

    fn processes(&self, serial: &str) -> Result<Vec<ProcessUsage>, AdbError> {
        let output = self.shell(
            serial,
            &[
                "top",
                "-b",
                "-n",
                "1",
                "-o",
                "PID,%CPU,RES,TIME+,ARGS",
                "-s",
                "2",
            ],
        )?;

        if let Some(message) = unsupported_command(&output) {
            return Err(AdbError::unsupported_command("top", message));
        }

        if !output.succeeded() {
            return Err(AdbError::command_failed(output.diagnostics()));
        }

        let processes = parse_top_processes(&output.stdout);

        if processes.is_empty() {
            return Err(AdbError::command_failed(
                "top output did not contain a process table",
            ));
        }

        Ok(processes)
    }

    fn doze_whitelist(&self, serial: &str) -> Result<Vec<DozeWhitelistEntry>, AdbError> {
        let output = self.shell(serial, &["dumpsys", "deviceidle", "whitelist"])?;

        if let Some(message) = unsupported_command(&output) {
            return Err(AdbError::unsupported_command(
                "dumpsys deviceidle whitelist",
                message,
            ));
        }

        if !output.succeeded() {
            return Err(AdbError::command_failed(output.diagnostics()));
        }

        Ok(parse_doze_whitelist(&output.stdout))
    }

    fn set_doze_whitelisted(
        &self,
        serial: &str,
        package: &PackageName,
        whitelisted: bool,
    ) -> Result<(), AdbError> {
        self.ensure_device_ready(serial)?;

        let operation = if whitelisted { "+" } else { "-" };
        let argument = format!("{operation}{package}");
        let output = self.shell(
            serial,
            &["dumpsys", "deviceidle", "whitelist", argument.as_str()],
        )?;

        doze_whitelist_command_result(&output)
    }

    fn ignored_background_packages(&self, serial: &str) -> Result<HashSet<PackageName>, AdbError> {
        let output = self.shell(
            serial,
            &[
                "cmd",
                "appops",
                "query-op",
                "RUN_ANY_IN_BACKGROUND",
                "ignore",
            ],
        )?;

        appops_command_result(&output, "cmd appops query-op")
            .map(|()| parse_ignored_background_packages(&output.stdout))
    }

    fn set_background_execution(
        &self,
        serial: &str,
        package: &PackageName,
        policy: BackgroundExecution,
    ) -> Result<(), AdbError> {
        self.ensure_device_ready(serial)?;

        let mode = match policy {
            BackgroundExecution::Allowed => "allow",
            BackgroundExecution::Ignored => "ignore",
        };
        let output = self.shell(
            serial,
            &[
                "cmd",
                "appops",
                "set",
                package.as_str(),
                "RUN_ANY_IN_BACKGROUND",
                mode,
            ],
        )?;

        appops_command_result(&output, "cmd appops set")
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

        let output = self.shell(serial, &["pm", "enable", "--user", "0", package.as_str()])?;

        match pm_failure(&output) {
            Some(_) => Err(pm_error(&output)),
            None => Ok(()),
        }
    }

    fn uninstall_app(&self, serial: &str, package: &PackageName) -> Result<(), AdbError> {
        self.ensure_device_ready(serial)?;

        let output = self.shell(
            serial,
            &["pm", "uninstall", "--user", "0", package.as_str()],
        )?;

        if uninstall_succeeded(&output) {
            Ok(())
        } else {
            Err(pm_error(&output))
        }
    }
}

fn doze_whitelist_command_result(output: &ProcessOutput) -> Result<(), AdbError> {
    if let Some(message) = unsupported_command(output) {
        return Err(AdbError::unsupported_command(
            "dumpsys deviceidle whitelist",
            message,
        ));
    }

    if doze_whitelist_command_succeeded(output) {
        Ok(())
    } else {
        Err(AdbError::command_failed(output.diagnostics()))
    }
}

fn doze_whitelist_command_succeeded(output: &ProcessOutput) -> bool {
    output.succeeded()
        && !output
            .stdout
            .to_ascii_lowercase()
            .contains("unknown package")
}

fn appops_command_result(output: &ProcessOutput, command: &'static str) -> Result<(), AdbError> {
    if let Some(message) = unsupported_command(output) {
        return Err(AdbError::unsupported_command(command, message));
    }

    if appops_command_succeeded(output) {
        Ok(())
    } else {
        Err(AdbError::command_failed(output.diagnostics()))
    }
}

fn appops_command_succeeded(output: &ProcessOutput) -> bool {
    if !output.succeeded() {
        return false;
    }

    let stderr = output.stderr.to_ascii_lowercase();
    if stderr.contains("error") || stderr.contains("exception") {
        return false;
    }

    let stdout = output.stdout.trim().to_ascii_lowercase();
    !(stdout.starts_with("error")
        || stdout.starts_with("exception")
        || stdout.starts_with("java.lang."))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn output(stdout: &str, code: i32) -> ProcessOutput {
        ProcessOutput {
            code: Some(code),
            stdout: stdout.to_owned(),
            stderr: String::new(),
            timed_out: false,
        }
    }

    #[test]
    fn appops_change_requires_success_without_reported_errors() {
        assert!(appops_command_succeeded(&output("", 0)));
        assert!(!appops_command_succeeded(&output("", 1)));
        assert!(!appops_command_succeeded(&output(
            "Error: Unknown package: com.example",
            0
        )));
        assert!(!appops_command_succeeded(&output(
            "java.lang.SecurityException: denied",
            0
        )));
        assert!(appops_command_succeeded(&output("com.error.app\n", 0)));
    }

    #[test]
    fn doze_whitelist_change_requires_success_and_a_known_package() {
        assert!(doze_whitelist_command_succeeded(&output("", 0)));
        assert!(doze_whitelist_command_succeeded(&output(
            "Added: com.example",
            0
        )));
        assert!(!doze_whitelist_command_succeeded(&output(
            "Unknown package: com.example",
            0
        )));
        assert!(!doze_whitelist_command_succeeded(&output("", 1)));
    }

    #[test]
    fn missing_doze_service_is_reported_as_unsupported() {
        let missing = output("Can't find service: deviceidle", 0);

        assert!(matches!(
            doze_whitelist_command_result(&missing),
            Err(AdbError::UnsupportedCommand { .. })
        ));
    }

    #[test]
    fn missing_cmd_binary_is_reported_as_unsupported() {
        let missing = ProcessOutput {
            code: Some(127),
            stdout: String::new(),
            stderr: "/system/bin/sh: cmd: not found".to_owned(),
            timed_out: false,
        };

        assert!(matches!(
            appops_command_result(&missing, "cmd appops query-op"),
            Err(AdbError::UnsupportedCommand { .. })
        ));
    }
}
