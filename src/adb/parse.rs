//! Parsing of textual ADB output into application-level values.

use std::collections::{BTreeMap, HashSet};

use android_utility_core::PackageName;

use super::{DeviceState, process::ProcessOutput};

/// One `pm list packages` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageEntry {
    /// Parsed package name.
    pub package: PackageName,
    /// APK path reported when `-f` was used.
    pub apk_path: Option<String>,
}

/// Parsed representation of one line from `adb devices -l`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceEntry {
    /// Device serial or emulator identifier.
    pub serial: String,
    /// Reported connection state.
    pub state: DeviceState,
    /// Device model reported by the transport, when present.
    pub model: Option<String>,
}

/// Parses `adb devices -l` output.
pub fn parse_devices(output: &str) -> Vec<DeviceEntry> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();

            if line.is_empty() || line.starts_with("List of devices") || line.starts_with('*') {
                return None;
            }

            let serial = line.split_whitespace().next()?.to_owned();
            let lowercase_line = line.to_ascii_lowercase();

            let state = if lowercase_line.contains("no permissions") {
                DeviceState::NoPermissions
            } else {
                match line.split_whitespace().nth(1)? {
                    "device" => DeviceState::Online,
                    "offline" => DeviceState::Offline,
                    "unauthorized" => DeviceState::Unauthorized,
                    _ => DeviceState::Unknown,
                }
            };

            let model = line
                .split_whitespace()
                .find_map(|token| token.strip_prefix("model:").map(str::to_owned));

            Some(DeviceEntry {
                serial,
                state,
                model,
            })
        })
        .collect()
}

/// Parses `pm list packages` output, keeping APK paths when present.
pub fn parse_package_entries(output: &str) -> Vec<PackageEntry> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let remainder = line.strip_prefix("package:")?;

            let (apk_path, raw_package) = match remainder.rsplit_once('=') {
                Some((path, raw_package)) => (Some(path.to_owned()), raw_package),
                None => (None, remainder),
            };

            let package = PackageName::parse(raw_package.trim()).ok()?;

            Some(PackageEntry { package, apk_path })
        })
        .collect()
}

/// Extracts the set of package names from `pm list packages` output.
pub fn parse_package_names(output: &str) -> HashSet<PackageName> {
    parse_package_entries(output)
        .into_iter()
        .map(|entry| entry.package)
        .collect()
}

/// Merges duplicate packages, preferring the base APK path.
pub fn merge_package_paths(entries: &[PackageEntry]) -> BTreeMap<PackageName, Option<String>> {
    let mut merged: BTreeMap<PackageName, Option<String>> = BTreeMap::new();

    for entry in entries {
        let candidate = entry.apk_path.as_deref();
        let preferred = candidate.is_some_and(|path| path.ends_with("/base.apk"));

        match merged.get_mut(&entry.package) {
            Some(existing) => {
                let existing_is_base = existing
                    .as_deref()
                    .is_some_and(|path| path.ends_with("/base.apk"));

                if !existing_is_base && (preferred || existing.is_none()) {
                    *existing = entry.apk_path.clone();
                }
            }
            None => {
                merged.insert(entry.package.clone(), entry.apk_path.clone());
            }
        }
    }

    merged
}

/// Returns a diagnostic message when a `pm` command failed.
///
/// Stack frames are removed so that only the actionable message reaches the
/// interface.
pub fn pm_failure(output: &ProcessOutput) -> Option<String> {
    if output.code != Some(0) {
        let diagnostics = summarize_diagnostics(&output.diagnostics());
        if diagnostics.is_empty() {
            return Some(format!("exit code {:?}", output.code));
        }
        return Some(diagnostics);
    }

    let stdout = output.stdout.trim();
    let lowercase = stdout.to_ascii_lowercase();

    if lowercase.contains("failure") || lowercase.contains("exception") {
        return Some(summarize_diagnostics(&output.diagnostics()));
    }

    None
}

/// Returns `true` when the device rejected a command because of permissions.
pub fn permission_denied(output: &ProcessOutput) -> bool {
    let diagnostics = output.diagnostics().to_ascii_lowercase();
    diagnostics.contains("securityexception") || diagnostics.contains("permission denial")
}

/// Returns `true` when an uninstall reported success.
pub fn uninstall_succeeded(output: &ProcessOutput) -> bool {
    pm_failure(output).is_none() && output.stdout.trim().eq_ignore_ascii_case("success")
}

fn summarize_diagnostics(diagnostics: &str) -> String {
    diagnostics
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("at "))
        .collect::<Vec<_>>()
        .join(" ")
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
    fn parses_devices() {
        let devices = parse_devices(
            "List of devices attached\n\
             emulator-5554\tdevice product:sdk model:Pixel_7 device:x transport_id:1\n\
             R58M12345\toffline\n\
             ABC123\tunauthorized\n\
             XYZ\tno permissions (user in plugdev group); see [http://developer.android.com]\n",
        );

        assert_eq!(devices.len(), 4);
        assert_eq!(devices[0].serial, "emulator-5554");
        assert_eq!(devices[0].state, DeviceState::Online);
        assert_eq!(devices[0].model.as_deref(), Some("Pixel_7"));
        assert_eq!(devices[1].state, DeviceState::Offline);
        assert_eq!(devices[2].state, DeviceState::Unauthorized);
        assert_eq!(devices[3].state, DeviceState::NoPermissions);
    }

    #[test]
    fn ignores_device_list_header_and_empty_output() {
        assert!(parse_devices("").is_empty());
        assert!(parse_devices("List of devices attached\n").is_empty());
    }

    #[test]
    fn parses_package_entries_with_and_without_paths() {
        let entries = parse_package_entries(
            "package:/data/app/~~abc==/com.example-1/base.apk=com.example\n\
             package:com.other\n\
             not a package line\n\
             package:com.invalid-name\n",
        );

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].package.as_str(), "com.example");
        assert_eq!(
            entries[0].apk_path.as_deref(),
            Some("/data/app/~~abc==/com.example-1/base.apk")
        );
        assert_eq!(entries[1].apk_path, None);
    }

    #[test]
    fn merges_duplicate_packages_and_prefers_base_apk() {
        let entries = parse_package_entries(
            "package:/data/app/com.example-1/split_config.arm64.apk=com.example\n\
             package:/data/app/com.example-1/base.apk=com.example\n\
             package:com.plain\n\
             package:com.plain\n",
        );

        let merged = merge_package_paths(&entries);

        assert_eq!(merged.len(), 2);
        assert_eq!(
            merged[&PackageName::parse("com.example").unwrap()].as_deref(),
            Some("/data/app/com.example-1/base.apk")
        );
        assert_eq!(merged[&PackageName::parse("com.plain").unwrap()], None);
    }

    #[test]
    fn detects_pm_failures() {
        assert_eq!(pm_failure(&output("Success", 0)), None);
        assert_eq!(
            pm_failure(&output("Failure [DELETE_FAILED_INTERNAL_ERROR]", 0)).as_deref(),
            Some("Failure [DELETE_FAILED_INTERNAL_ERROR]")
        );
        assert_eq!(
            pm_failure(&output("", 1)).as_deref(),
            Some("exit code Some(1)")
        );
    }

    #[test]
    fn removes_stack_frames_from_failure_messages() {
        let failure = output(
            "Exception occurred while executing 'disable-user':\n\
             java.lang.SecurityException: Cannot disable system packages.\n\
             \tat com.android.server.pm.PackageManagerServiceImpl.shouldRestrictEnabledSettingsChange(PackageManagerServiceImpl.java:927)\n\
             \tat com.android.server.pm.PackageManagerService.setEnabledSettings(PackageManagerService.java:4082)",
            1,
        );

        let message = pm_failure(&failure).expect("failure detected");
        assert!(message.contains("SecurityException"));
        assert!(message.contains("Cannot disable system packages"));
        assert!(!message.contains("\tat "));
        assert!(!message.contains("PackageManagerServiceImpl"));
    }

    #[test]
    fn detects_permission_denial() {
        let failure = output(
            "Exception occurred while executing 'disable-user':\n\
             java.lang.SecurityException: Cannot disable system packages.\n",
            1,
        );

        assert!(permission_denied(&failure));
        assert!(!permission_denied(&output("Failure [not installed]", 1)));
    }

    #[test]
    fn uninstall_requires_success_text() {
        assert!(uninstall_succeeded(&output("Success\n", 0)));
        assert!(!uninstall_succeeded(&output("", 0)));
        assert!(!uninstall_succeeded(&output("Failure", 1)));
    }
}
