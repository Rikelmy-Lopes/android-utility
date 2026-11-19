//! Detection of device responses that report an unavailable command or service.

use super::super::process::ProcessOutput;

const UNSUPPORTED_MARKERS: &[&str] = &[
    "can't find service",
    "cannot find service",
    "unknown service",
    "cmd: not found",
    "cmd: inaccessible or not found",
    "top: not found",
    "top: inaccessible or not found",
    "unknown option",
    "invalid option",
    "unknown command",
];

/// Returns the diagnostic message when the device reports that the requested
/// command or service is unavailable.
pub fn unsupported_command(output: &ProcessOutput) -> Option<String> {
    let diagnostics = output.diagnostics();
    let lowercase = diagnostics.to_ascii_lowercase();

    UNSUPPORTED_MARKERS
        .iter()
        .any(|marker| lowercase.contains(marker))
        .then_some(diagnostics)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn output(stdout: &str, stderr: &str, code: i32) -> ProcessOutput {
        ProcessOutput {
            code: Some(code),
            stdout: stdout.to_owned(),
            stderr: stderr.to_owned(),
            timed_out: false,
        }
    }

    #[test]
    fn detects_missing_doze_service() {
        let failure = output("Can't find service: deviceidle\n", "", 0);

        assert_eq!(
            unsupported_command(&failure).as_deref(),
            Some("Can't find service: deviceidle")
        );
    }

    #[test]
    fn detects_missing_cmd_binary() {
        let failure = output("", "/system/bin/sh: cmd: not found\n", 127);

        assert_eq!(
            unsupported_command(&failure).as_deref(),
            Some("/system/bin/sh: cmd: not found")
        );
    }

    #[test]
    fn ignores_normal_output() {
        assert!(unsupported_command(&output("com.example\n", "", 0)).is_none());
        assert!(unsupported_command(&output("", "", 0)).is_none());
    }
}
