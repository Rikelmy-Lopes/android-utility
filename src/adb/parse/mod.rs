//! Parsing of textual ADB output into application-level values.

mod devices;
mod optimization;
mod packages;
mod pm;
mod processes;

pub use devices::parse_devices;
pub use optimization::{parse_doze_whitelist, parse_ignored_background_packages};
pub use packages::{merge_package_paths, parse_package_entries, parse_package_names};
pub use pm::{permission_denied, pm_failure, uninstall_succeeded};
pub use processes::parse_top_processes;
