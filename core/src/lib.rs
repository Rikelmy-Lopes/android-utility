//! GUI-independent business logic for Android Utility.
//!
//! This crate holds domain models, device information analysis, and domain
//! validation. It must not depend on the graphical interface, ADB process
//! execution, or operating-system process APIs.

pub mod app;

pub use app::{InstalledApp, PackageName, PackageNameError, sort_apps};
