//! Parsing of textual ADB output into application-level values.

use std::collections::{BTreeMap, HashSet};

use android_utility_core::{
    BackgroundExecution, DozeWhitelistEntry, DozeWhitelistSource, PackageName, ProcessUsage,
};

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

/// Parses `dumpsys deviceidle whitelist` output.
///
/// Lines are reported as `source,package`, but older devices may print the
/// package name alone. Unrecognized lines are ignored.
pub fn parse_doze_whitelist(output: &str) -> Vec<DozeWhitelistEntry> {
    output
        .lines()
        .filter_map(|line| {
            let line = line.trim();

            if line.is_empty() {
                return None;
            }

            let (source, raw_package) = match line.split_once(',') {
                Some((source, package)) => (
                    doze_whitelist_source(source),
                    package.split(',').next().unwrap_or(package).trim(),
                ),
                None => (DozeWhitelistSource::Unknown, line),
            };

            let package = PackageName::parse(raw_package).ok()?;

            Some(DozeWhitelistEntry { package, source })
        })
        .collect()
}

/// Parses `cmd appops get <package> RUN_ANY_IN_BACKGROUND` output.
///
/// Devices print the value after a colon, an equals sign, or as a bare token.
/// When no explicit operation is reported, the fallback is `Default mode`.
/// The first recognized value wins. Unrecognized output yields `None`.
pub fn parse_background_execution(output: &str) -> Option<BackgroundExecution> {
    let explicit = output.lines().find_map(|line| {
        let (_, value) = line.split_once("RUN_ANY_IN_BACKGROUND")?;
        parse_appops_mode(value)
    });

    explicit.or_else(|| {
        output.lines().find_map(|line| {
            let (name, value) = line.split_once(':')?;

            if name.trim().eq_ignore_ascii_case("Default mode") {
                parse_appops_mode(value)
            } else {
                None
            }
        })
    })
}

fn parse_appops_mode(raw: &str) -> Option<BackgroundExecution> {
    let value = raw
        .trim_start_matches([':', '='])
        .split_whitespace()
        .next()?
        .trim_end_matches(';');

    match value.to_ascii_lowercase().as_str() {
        "allow" => Some(BackgroundExecution::Allowed),
        "ignore" => Some(BackgroundExecution::Ignored),
        _ => None,
    }
}

fn doze_whitelist_source(raw: &str) -> DozeWhitelistSource {
    match raw.trim().to_ascii_lowercase().as_str() {
        "system" | "system-excidle" | "system-ui" => DozeWhitelistSource::System,
        "user" => DozeWhitelistSource::User,
        _ => DozeWhitelistSource::Unknown,
    }
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

/// Column positions in a `top -o ...` process table.
#[derive(Debug, Clone, Copy)]
struct TopColumns {
    pid: usize,
    cpu: usize,
    memory: usize,
    cpu_time: usize,
    command: usize,
}

impl TopColumns {
    fn from_header(tokens: &[&str]) -> Option<Self> {
        let find = |names: &[&str]| {
            tokens.iter().position(|token| {
                let normalized = token.trim_matches(['[', ']']).trim().to_ascii_lowercase();
                names.contains(&normalized.as_str())
            })
        };

        Some(Self {
            pid: find(&["pid"])?,
            cpu: find(&["%cpu", "cpu"])?,
            memory: find(&["res", "rss"])?,
            cpu_time: find(&["time+", "time"])?,
            command: find(&["args", "cmdline", "command"])?,
        })
    }

    fn parse_row(&self, tokens: &[&str]) -> Option<ProcessUsage> {
        if tokens.len() <= self.command {
            return None;
        }

        let pid = tokens.get(self.pid)?.parse().ok()?;
        let cpu_percent = parse_cpu_percent(tokens.get(self.cpu)?)?;
        let memory_bytes = parse_memory_bytes(tokens.get(self.memory)?)?;
        let cpu_time = (*tokens.get(self.cpu_time)?).to_owned();
        let command = tokens[self.command..].join(" ");

        if command.trim().is_empty() {
            return None;
        }

        Some(ProcessUsage {
            pid,
            cpu_percent,
            memory_bytes,
            cpu_time,
            command,
        })
    }
}

/// Parses the process table printed by `top -o PID,%CPU,RES,TIME+,ARGS`.
///
/// Summary lines before the table are ignored. The column header is located
/// from the output so that devices adding or reordering columns keep working.
pub fn parse_top_processes(output: &str) -> Vec<ProcessUsage> {
    let mut columns = None;
    let mut processes = Vec::new();

    for line in output.lines() {
        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        // The sorted column is wrapped in brackets, which can remove the space
        // before it. Split those for the header without changing data rows.
        if line.contains("PID") {
            let spaced_header = line.replace('[', " [").replace(']', "] ");
            let header_tokens: Vec<&str> = spaced_header.split_whitespace().collect();

            if let Some(header) = TopColumns::from_header(&header_tokens) {
                columns = Some(header);
                continue;
            }
        }

        let Some(columns) = &columns else {
            continue;
        };

        if let Some(process) = columns.parse_row(&tokens) {
            processes.push(process);
        }
    }

    processes
}

fn parse_cpu_percent(text: &str) -> Option<f32> {
    let value: f64 = text.trim_end_matches('%').replace(',', ".").parse().ok()?;

    if value.is_finite() && value >= 0.0 {
        Some(value as f32)
    } else {
        None
    }
}

fn parse_memory_bytes(text: &str) -> Option<u64> {
    let text = text.trim().replace(',', ".");

    let suffix_start = text
        .char_indices()
        .rev()
        .find(|(_, character)| !character.is_ascii_alphabetic())
        .map_or(0, |(index, character)| index + character.len_utf8());

    let (number, suffix) = text.split_at(suffix_start);
    let multiplier = match suffix.to_ascii_lowercase().as_str() {
        "" => 1024,
        "b" => 1,
        "k" | "kb" | "kib" => 1024,
        "m" | "mb" | "mib" => 1024 * 1024,
        "g" | "gb" | "gib" => 1024 * 1024 * 1024,
        "t" | "tb" | "tib" => 1024_u64.pow(4),
        _ => return None,
    };

    let value: f64 = number.parse().ok()?;
    if !value.is_finite() || value < 0.0 {
        return None;
    }

    Some((value * multiplier as f64).round() as u64)
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

    #[test]
    fn parses_top_process_table() {
        let processes = parse_top_processes(
            "Tasks: 550 total,   2 running, 548 sleeping,   0 stopped,   0 zombie\n\
             Mem:      5.5G total,      5.2G used,    300M free\n\
             800%cpu  62%user   2%nice  29%sys 707%idle   0%iow   0%irq   3%sirq   0%host\n\
             PID %CPU RES TIME+ ARGS\n\
             1604 8.0 284M 1:04.08 system_server\n\
             2201 25.3 120M 00:12.45 com.example.app\n\
             42 0.0 12.5M 00:00.01 /system/bin/surfaceflinger --flag value\n",
        );

        assert_eq!(processes.len(), 3);
        assert_eq!(processes[0].pid, 1604);
        assert_eq!(processes[0].cpu_percent, 8.0);
        assert_eq!(processes[0].memory_bytes, 284 * 1024 * 1024);
        assert_eq!(processes[0].cpu_time, "1:04.08");
        assert_eq!(processes[0].command, "system_server");
        assert_eq!(processes[2].memory_bytes, (12.5 * 1024.0 * 1024.0) as u64);
        assert_eq!(
            processes[2].command,
            "/system/bin/surfaceflinger --flag value"
        );
    }

    #[test]
    fn parses_top_columns_in_any_order() {
        let processes = parse_top_processes(
            "ARGS RES PID %CPU TIME+\n\
             com.example 1.0G 99 12.5 00:01.00\n",
        );

        assert_eq!(processes.len(), 1);
        assert_eq!(processes[0].pid, 99);
        assert_eq!(processes[0].cpu_percent, 12.5);
        assert_eq!(processes[0].memory_bytes, 1024 * 1024 * 1024);
    }

    #[test]
    fn parses_header_with_attached_sort_bracket() {
        let processes = parse_top_processes(
            "Tasks: 927 total,   1 running, 926 sleeping\n\
             800%cpu  13%user   3%nice  30%sys 747%idle\n\
             PID[%CPU] RES     TIME+ ARGS\n\
             31438  3.3 358M   7:23.60 com.google.android.gms\n",
        );

        assert_eq!(processes.len(), 1);
        assert_eq!(processes[0].pid, 31438);
        assert_eq!(processes[0].cpu_percent, 3.3);
        assert_eq!(processes[0].memory_bytes, 358 * 1024 * 1024);
        assert_eq!(processes[0].command, "com.google.android.gms");
    }

    #[test]
    fn skips_malformed_top_rows() {
        assert!(parse_top_processes("").is_empty());
        assert!(parse_top_processes("Tasks: 1 total\n").is_empty());

        let processes = parse_top_processes(
            "PID %CPU RES TIME+ ARGS\n\
             not-a-pid 1.0 1M 00:00.01 broken\n\
             10 broken 1M 00:00.01 broken\n\
             11 1.0 broken 00:00.01 broken\n\
             12 1.0 1M 00:00.01\n\
             13 -1.0 1M 00:00.01 broken\n",
        );

        assert!(processes.is_empty());
    }

    #[test]
    fn parses_memory_suffixes() {
        assert_eq!(parse_memory_bytes("0"), Some(0));
        assert_eq!(parse_memory_bytes("512"), Some(512 * 1024));
        assert_eq!(parse_memory_bytes("512K"), Some(512 * 1024));
        assert_eq!(parse_memory_bytes("1.5M"), Some(1_572_864));
        assert_eq!(parse_memory_bytes("2GiB"), Some(2 * 1024 * 1024 * 1024));
        assert_eq!(parse_memory_bytes("512B"), Some(512));
        assert_eq!(parse_memory_bytes("broken"), None);
        assert_eq!(parse_memory_bytes("-1M"), None);
    }

    #[test]
    fn parses_background_execution_values() {
        assert_eq!(
            parse_background_execution("RUN_ANY_IN_BACKGROUND: allow\n"),
            Some(BackgroundExecution::Allowed)
        );
        assert_eq!(
            parse_background_execution("Uid mode: RUN_ANY_IN_BACKGROUND: ignore\n"),
            Some(BackgroundExecution::Ignored)
        );
        assert_eq!(
            parse_background_execution("RUN_ANY_IN_BACKGROUND=ALLOW"),
            Some(BackgroundExecution::Allowed)
        );
        assert_eq!(
            parse_background_execution("  RUN_ANY_IN_BACKGROUND   ignore  \n"),
            Some(BackgroundExecution::Ignored)
        );
        assert_eq!(
            parse_background_execution(
                "RUN_ANY_IN_BACKGROUND: allow; time=+2d22h19m40s676ms ago\n"
            ),
            Some(BackgroundExecution::Allowed)
        );
        assert_eq!(
            parse_background_execution("RUN_ANY_IN_BACKGROUND: ignore; time=+1h2m3s456ms ago\n"),
            Some(BackgroundExecution::Ignored)
        );
        assert_eq!(
            parse_background_execution("No operations.\nDefault mode: allow\n"),
            Some(BackgroundExecution::Allowed)
        );
        assert_eq!(
            parse_background_execution("No operations.\nDefault mode: ignore\n"),
            Some(BackgroundExecution::Ignored)
        );
    }

    #[test]
    fn ignores_missing_or_unknown_background_execution_values() {
        assert_eq!(parse_background_execution(""), None);
        assert_eq!(parse_background_execution("No operations.\n"), None);
        assert_eq!(
            parse_background_execution("RUN_ANY_IN_BACKGROUND: default\n"),
            None
        );
        assert_eq!(
            parse_background_execution("RUN_ANY_IN_BACKGROUND: deny\n"),
            None
        );
    }

    #[test]
    fn parses_doze_whitelist_sources() {
        let entries = parse_doze_whitelist(
            "system,com.android.shell\n\
             user,com.example.chat\n\
             system-excidle,com.google.android.gms\n\
             com.example.legacy\n\
             \n\
             not a package line\n\
             user,com.invalid-name\n",
        );

        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].package.as_str(), "com.android.shell");
        assert_eq!(entries[0].source, DozeWhitelistSource::System);
        assert_eq!(entries[1].package.as_str(), "com.example.chat");
        assert_eq!(entries[1].source, DozeWhitelistSource::User);
        assert_eq!(
            entries[2].source,
            DozeWhitelistSource::System,
            "temporary system exemptions count as system entries"
        );
        assert_eq!(entries[3].package.as_str(), "com.example.legacy");
        assert_eq!(entries[3].source, DozeWhitelistSource::Unknown);
    }

    #[test]
    fn ignores_empty_or_unrecognized_doze_whitelist_output() {
        assert!(parse_doze_whitelist("").is_empty());
        assert!(parse_doze_whitelist("Whitelist:\n").is_empty());
    }

    #[test]
    fn parses_doze_whitelist_lines_with_a_trailing_user_id() {
        let entries = parse_doze_whitelist(
            "system-excidle,com.android.shell,2000\n\
             system,com.android.shell,2000\n\
             user,com.whatsapp,10305\n\
             user,com.example,not-a-user-id\n",
        );

        assert_eq!(entries.len(), 4);
        assert_eq!(entries[0].package.as_str(), "com.android.shell");
        assert_eq!(entries[0].source, DozeWhitelistSource::System);
        assert_eq!(entries[2].package.as_str(), "com.whatsapp");
        assert_eq!(entries[2].source, DozeWhitelistSource::User);
    }
}
