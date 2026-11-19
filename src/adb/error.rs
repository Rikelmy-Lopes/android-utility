//! Errors raised by ADB integration.

use std::fmt;

/// Failure while locating, running, or interpreting ADB.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdbError {
    /// The ADB executable could not be found.
    AdbNotFound {
        /// Program that was attempted.
        program: String,
    },
    /// The operating system refused to start the process.
    Spawn {
        /// Operating-system error message.
        message: String,
    },
    /// The process did not finish within the allowed time.
    Timeout {
        /// Timeout in seconds.
        seconds: u64,
    },
    /// A process finished with a non-zero exit code or reported failure.
    CommandFailed {
        /// Combined diagnostic output.
        message: String,
    },
    /// The device does not provide the requested command or service.
    UnsupportedCommand {
        /// Command or service that is unavailable.
        command: &'static str,
        /// Combined diagnostic output.
        message: String,
    },
    /// The device rejected the command because of permissions.
    PermissionDenied {
        /// Diagnostic message without stack frames.
        message: String,
    },
    /// A local file or directory operation failed.
    Io {
        /// Operating-system error message.
        message: String,
    },
}

impl AdbError {
    /// Builds a command failure from arbitrary diagnostic text.
    pub fn command_failed(message: impl Into<String>) -> Self {
        Self::CommandFailed {
            message: message.into(),
        }
    }

    /// Builds an error for a command or service the device does not provide.
    pub fn unsupported_command(command: &'static str, message: impl Into<String>) -> Self {
        Self::UnsupportedCommand {
            command,
            message: message.into(),
        }
    }
}

impl fmt::Display for AdbError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AdbNotFound { program } => {
                write!(formatter, "command not found: {program}")
            }
            Self::Spawn { message } => write!(formatter, "could not start process: {message}"),
            Self::Timeout { seconds } => {
                write!(formatter, "command timed out after {seconds} seconds")
            }
            Self::CommandFailed { message } => write!(formatter, "command failed: {message}"),
            Self::UnsupportedCommand { command, message } => {
                write!(formatter, "command not supported: {command} ({message})")
            }
            Self::PermissionDenied { message } => {
                write!(formatter, "permission denied: {message}")
            }
            Self::Io { message } => write!(formatter, "file operation failed: {message}"),
        }
    }
}

impl std::error::Error for AdbError {}
