//! Execution of external processes with a bounded runtime.

use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::error::AdbError;

/// Windows process creation flag that runs a console program without a console
/// window and without the hidden-console setup cost of `CREATE_NO_WINDOW`.
#[cfg(windows)]
const DETACHED_PROCESS: u32 = 0x0000_0008;

/// Result of a finished process invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOutput {
    /// Exit code reported by the operating system, when available.
    pub code: Option<i32>,
    /// Captured standard output.
    pub stdout: String,
    /// Captured standard error.
    pub stderr: String,
    /// Whether the process was killed because the timeout elapsed.
    pub timed_out: bool,
}

impl ProcessOutput {
    /// Returns `true` when the process exited successfully.
    pub fn succeeded(&self) -> bool {
        !self.timed_out && self.code == Some(0)
    }

    /// Combines standard output and standard error into one diagnostic string.
    pub fn diagnostics(&self) -> String {
        let stdout = self.stdout.trim();
        let stderr = self.stderr.trim();

        match (stdout.is_empty(), stderr.is_empty()) {
            (false, false) => format!("{stdout} {stderr}"),
            (false, true) => stdout.to_owned(),
            (true, false) => stderr.to_owned(),
            (true, true) => String::new(),
        }
    }
}

/// Runs a program with argument-based invocation and a timeout.
pub fn run(
    program: &str,
    arguments: &[String],
    timeout: Duration,
) -> Result<ProcessOutput, AdbError> {
    let command_line = format_command(program, arguments);
    tracing::debug!(
        command = %command_line,
        timeout_ms = timeout.as_millis() as u64,
        "starting process"
    );

    let started = Instant::now();

    let mut command = Command::new(program);
    command
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;

        command.creation_flags(DETACHED_PROCESS);
    }

    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            tracing::error!(
                command = %command_line,
                %error,
                "process could not be started"
            );

            return Err(match error.kind() {
                std::io::ErrorKind::NotFound => AdbError::AdbNotFound {
                    program: program.to_owned(),
                },
                _ => AdbError::Spawn {
                    message: error.to_string(),
                },
            });
        }
    };

    let stdout_reader = child.stdout.take().map(spawn_reader);
    let stderr_reader = child.stderr.take().map(spawn_reader);

    let deadline = Instant::now() + timeout;
    let mut timed_out = false;

    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break Some(status),
            Ok(None) if Instant::now() >= deadline => {
                timed_out = true;
                let _ = child.kill();
                break child.wait().ok();
            }
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(_) => {
                let _ = child.kill();
                break child.wait().ok();
            }
        }
    };

    let stdout = join_reader(stdout_reader);
    let stderr = join_reader(stderr_reader);

    if timed_out {
        tracing::warn!(
            command = %command_line,
            timeout_secs = timeout.as_secs(),
            "process timed out"
        );

        return Err(AdbError::Timeout {
            seconds: timeout.as_secs(),
        });
    }

    let output = ProcessOutput {
        code: status.and_then(|status| status.code()),
        stdout,
        stderr,
        timed_out,
    };

    let elapsed_ms = started.elapsed().as_millis() as u64;

    if output.succeeded() {
        tracing::debug!(
            command = %command_line,
            exit_code = ?output.code,
            elapsed_ms,
            "process finished"
        );
    } else {
        tracing::warn!(
            command = %command_line,
            exit_code = ?output.code,
            elapsed_ms,
            stderr = %preview(&output.stderr),
            "process failed"
        );
    }

    Ok(output)
}

/// Formats a process invocation for a log record.
fn format_command(program: &str, arguments: &[String]) -> String {
    let mut command = String::from(program);

    for argument in arguments {
        command.push(' ');

        if argument.is_empty() {
            command.push_str("\"\"");
        } else if argument.chars().any(char::is_whitespace) {
            command.push('"');
            command.push_str(argument);
            command.push('"');
        } else {
            command.push_str(argument);
        }
    }

    command
}

/// Returns a bounded, trimmed excerpt of diagnostic output.
fn preview(text: &str) -> String {
    const MAX_CHARS: usize = 500;

    let trimmed = text.trim();

    if trimmed.chars().count() <= MAX_CHARS {
        return trimmed.to_owned();
    }

    let mut excerpt: String = trimmed.chars().take(MAX_CHARS).collect();
    excerpt.push('…');
    excerpt
}

fn spawn_reader<R>(mut reader: R) -> thread::JoinHandle<String>
where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let mut buffer = Vec::new();
        let _ = reader.read_to_end(&mut buffer);
        String::from_utf8_lossy(&buffer).into_owned()
    })
}

fn join_reader(handle: Option<thread::JoinHandle<String>>) -> String {
    handle
        .and_then(|handle| handle.join().ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arguments(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn formats_command_with_quoted_whitespace_arguments() {
        assert_eq!(
            format_command("adb", &arguments(&["-s", "serial", "shell", "pm list"])),
            "adb -s serial shell \"pm list\""
        );
    }

    #[test]
    fn formats_empty_argument_as_quoted_pair() {
        assert_eq!(format_command("adb", &arguments(&[""])), "adb \"\"");
    }

    #[test]
    fn keeps_short_diagnostics_untouched() {
        assert_eq!(preview("  failure  "), "failure");
    }

    #[test]
    fn bounds_long_diagnostics() {
        let text = "x".repeat(600);
        let excerpt = preview(&text);

        assert_eq!(excerpt.chars().count(), 501);
        assert!(excerpt.ends_with('…'));
    }
}
