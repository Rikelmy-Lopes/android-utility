//! GUI-independent business logic for Android Utility.
//!
//! This crate holds domain models, device information analysis, and domain
//! validation. It must not depend on the graphical interface, ADB process
//! execution, or operating-system process APIs.

pub mod app;
pub mod background;
pub mod doze;
pub mod process;
pub mod session;

pub use app::{InstalledApp, PackageName, PackageNameError, sort_apps};
pub use background::BackgroundExecution;
pub use doze::{DozeWhitelistEntry, DozeWhitelistSource};
pub use process::{ProcessSort, ProcessUsage, sort_processes};
pub use session::{MAX_TRACKED_PROCESSES, ProcessAverage, ResourceSession, SessionSummary};
