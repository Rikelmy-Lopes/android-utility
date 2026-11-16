//! Execution of external processes with a bounded runtime.

use std::io::Read;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use super::error::AdbError;

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
    let mut child = Command::new(program)
        .args(arguments)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => AdbError::AdbNotFound {
                program: program.to_owned(),
            },
            _ => AdbError::Spawn {
                message: error.to_string(),
            },
        })?;

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
        return Err(AdbError::Timeout {
            seconds: timeout.as_secs(),
        });
    }

    Ok(ProcessOutput {
        code: status.and_then(|status| status.code()),
        stdout,
        stderr,
        timed_out,
    })
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
