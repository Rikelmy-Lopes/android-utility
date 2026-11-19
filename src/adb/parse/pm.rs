//! Interpretation of `pm` command results.

use super::super::process::ProcessOutput;

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
