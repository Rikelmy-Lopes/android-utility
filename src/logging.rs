//! Logging setup: daily log files and debug-build terminal output.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tracing_appender::rolling;
use tracing_subscriber::Layer;
use tracing_subscriber::filter::{LevelFilter, Targets};
use tracing_subscriber::fmt;
use tracing_subscriber::fmt::time::OffsetTime;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

use crate::startup::AppPaths;

/// Cargo package name normalized to the Rust crate target name.
const PACKAGE_TARGET: &str = "android_utility";

/// Log timestamp format: date, time, and milliseconds.
const TIMESTAMP_FORMAT: &[time::format_description::FormatItem<'static>] = time::macros::format_description!(
    "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]"
);

/// Prefix of the daily log file names.
const LOG_FILE_PREFIX: &str = "android-utility";

/// Suffix of the daily log file names.
const LOG_FILE_SUFFIX: &str = "log";

/// Writer shared by every thread that emits a log record.
type LogWriter = Mutex<Box<dyn Write + Send>>;

/// Initializes the global logger.
///
/// Records are always written to a daily file under the per-user data
/// directory. In debug builds the same records also go to standard error.
/// Returns the log directory when file logging is active.
pub fn init() -> Option<PathBuf> {
    let (writer, directory) = open_writer();

    let console_filter = if cfg!(debug_assertions) {
        record_filter()
    } else {
        Targets::new().with_default(LevelFilter::OFF)
    };

    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .with_writer(writer)
                .with_ansi(false)
                .with_timer(local_timer())
                .with_filter(record_filter()),
        )
        .with(
            fmt::layer()
                .with_writer(io::stderr)
                .with_ansi(cfg!(debug_assertions))
                .with_timer(local_timer())
                .with_filter(console_filter),
        )
        .init();

    match &directory {
        Some(directory) => {
            tracing::info!(directory = %directory.display(), "logging initialized");
        }
        None => {
            tracing::warn!("file logging unavailable; records go to the terminal only");
        }
    }

    directory
}

/// Returns the level filter applied to every log destination.
///
/// Application records are kept at debug level. Records from dependencies are
/// limited to errors to keep the log readable.
fn record_filter() -> Targets {
    Targets::new()
        .with_default(LevelFilter::ERROR)
        .with_target(PACKAGE_TARGET, LevelFilter::DEBUG)
}

/// Returns a timer that renders local time, falling back to UTC when the
/// platform cannot report the local offset.
fn local_timer() -> OffsetTime<&'static [time::format_description::FormatItem<'static>]> {
    let offset = time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC);
    OffsetTime::new(offset, TIMESTAMP_FORMAT)
}

/// Builds the file writer and reports the directory in use.
fn open_writer() -> (LogWriter, Option<PathBuf>) {
    match log_directory() {
        Ok(directory) => match file_appender(&directory) {
            Ok(appender) => (Mutex::new(Box::new(appender)), Some(directory)),
            Err(error) => {
                report_setup_error("could not open log file", &error.to_string());
                (discarding_writer(), None)
            }
        },
        Err(error) => {
            report_setup_error("could not resolve log directory", &error);
            (discarding_writer(), None)
        }
    }
}

/// Returns the existing per-user log directory, creating it when needed.
fn log_directory() -> Result<PathBuf, String> {
    let paths = AppPaths::for_current_user().map_err(|error| error.to_string())?;
    paths.ensure_log_dir().map_err(|error| error.to_string())
}

/// Creates a daily rolling file appender inside `directory`.
fn file_appender(directory: &Path) -> io::Result<rolling::RollingFileAppender> {
    fs::create_dir_all(directory)?;

    rolling::Builder::new()
        .rotation(rolling::Rotation::DAILY)
        .filename_prefix(LOG_FILE_PREFIX)
        .filename_suffix(LOG_FILE_SUFFIX)
        .build(directory)
        .map_err(|error| io::Error::other(error.to_string()))
}

/// Writer that discards every record.
fn discarding_writer() -> LogWriter {
    Mutex::new(Box::new(io::sink()))
}

/// Reports a setup failure before the logger exists.
fn report_setup_error(context: &str, error: &str) {
    eprintln!("android-utility: {context}: {error}");
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use tracing_subscriber::fmt::time::FormatTime;

    use super::*;

    #[test]
    fn formats_timestamps_with_millisecond_precision() {
        let timer = local_timer();
        let mut buffer = String::new();

        timer
            .format_time(&mut tracing_subscriber::fmt::format::Writer::new(
                &mut buffer,
            ))
            .expect("format timestamp");

        assert_eq!(buffer.len(), 23, "unexpected timestamp: {buffer}");
        assert_eq!(&buffer[4..5], "-");
        assert_eq!(&buffer[7..8], "-");
        assert_eq!(&buffer[10..11], "T");
        assert_eq!(&buffer[19..20], ".");
    }

    #[test]
    fn daily_appender_writes_into_the_given_directory() {
        let directory = std::env::temp_dir().join(format!(
            "android-utility-log-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = fs::remove_dir_all(&directory);

        let mut appender = file_appender(&directory).expect("appender");
        appender.write_all(b"record\n").expect("write");
        appender.flush().expect("flush");

        let written: Vec<_> = fs::read_dir(&directory)
            .expect("read directory")
            .filter_map(Result::ok)
            .collect();

        assert_eq!(written.len(), 1);
        assert_eq!(
            fs::read_to_string(written[0].path()).expect("read log"),
            "record\n"
        );

        let _ = fs::remove_dir_all(&directory);
    }
}
