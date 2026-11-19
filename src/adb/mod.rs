//! ADB integration: process invocation, output parsing, and device operations.
//!
//! Process execution stays inside this module. Parsed values are handed to the
//! application layer as domain types.

mod client;
mod device;
mod error;
mod parse;
mod process;

use std::collections::HashSet;

use android_utility_core::{
    BackgroundExecution, DozeWhitelistEntry, InstalledApp, PackageName, ProcessUsage,
};

pub use client::Adb;
pub use device::{Device, DeviceState};
pub use error::AdbError;

/// ADB operations required by application commands.
///
/// Implementations are substituted in tests so that no device or process is
/// needed.
pub trait DeviceService: Send + Sync {
    /// Lists devices reported by the ADB server.
    fn devices(&self) -> Result<Vec<Device>, AdbError>;

    /// Lists installed packages with system and disabled state.
    fn list_apps(&self, serial: &str) -> Result<Vec<InstalledApp>, AdbError>;

    /// Reads the current process table with CPU and resident memory usage.
    fn processes(&self, serial: &str) -> Result<Vec<ProcessUsage>, AdbError>;

    /// Lists packages exempt from Doze restrictions.
    fn doze_whitelist(&self, serial: &str) -> Result<Vec<DozeWhitelistEntry>, AdbError>;

    /// Adds or removes a package from the Device Idle whitelist.
    fn set_doze_whitelisted(
        &self,
        serial: &str,
        package: &PackageName,
        whitelisted: bool,
    ) -> Result<(), AdbError>;

    /// Lists packages restricted from running in the background.
    fn ignored_background_packages(&self, serial: &str) -> Result<HashSet<PackageName>, AdbError>;

    /// Changes the background execution policy of one package.
    fn set_background_execution(
        &self,
        serial: &str,
        package: &PackageName,
        policy: BackgroundExecution,
    ) -> Result<(), AdbError>;

    /// Reads the application label from the APK at `apk_path`.
    fn application_label(&self, serial: &str, apk_path: &str) -> Result<Option<String>, AdbError>;

    /// Disables a package for the current user.
    fn disable_app(&self, serial: &str, package: &PackageName) -> Result<(), AdbError>;

    /// Re-enables a package for the current user.
    fn enable_app(&self, serial: &str, package: &PackageName) -> Result<(), AdbError>;

    /// Uninstalls a package for the current user only.
    fn uninstall_app(&self, serial: &str, package: &PackageName) -> Result<(), AdbError>;
}
