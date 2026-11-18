//! Application commands and their background execution.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::mpsc;

use android_utility_core::{
    BackgroundExecution, DozeWhitelistEntry, InstalledApp, PackageName, ProcessUsage,
};

use crate::adb::{AdbError, Device, DeviceService};

/// An application whose label should be read from its APK.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelTarget {
    /// Package that owns the APK.
    pub package: PackageName,
    /// Remote path of the base APK.
    pub apk_path: String,
}

/// A user-triggered application action.
#[derive(Debug, Clone)]
pub enum Command {
    /// Refreshes the list of connected devices.
    LoadDevices,
    /// Loads installed packages from one device.
    LoadApps {
        /// Device serial.
        serial: String,
    },
    /// Reads the current process table from one device.
    LoadProcesses {
        /// Device serial.
        serial: String,
    },
    /// Reads the Doze whitelist from one device.
    LoadDozeWhitelist {
        /// Device serial.
        serial: String,
    },
    /// Reads the background execution policy for the given packages.
    LoadBackgroundExecution {
        /// Device serial.
        serial: String,
        /// Packages to query.
        packages: Vec<PackageName>,
    },
    /// Changes the background execution policy of one package.
    SetBackgroundExecution {
        /// Device serial.
        serial: String,
        /// Package to update.
        package: PackageName,
        /// Requested policy.
        policy: BackgroundExecution,
    },
    /// Adds or removes a package from the Doze whitelist.
    SetDozeWhitelist {
        /// Device serial.
        serial: String,
        /// Package to update.
        package: PackageName,
        /// Whether the package should be ignored by Doze.
        whitelisted: bool,
    },
    /// Reads application labels for the given packages.
    LoadLabels {
        /// Device serial.
        serial: String,
        /// Packages to resolve.
        targets: Vec<LabelTarget>,
    },
    /// Disables a package for the current user.
    Disable {
        /// Device serial.
        serial: String,
        /// Package to disable.
        package: PackageName,
    },
    /// Enables a disabled package.
    Enable {
        /// Device serial.
        serial: String,
        /// Package to enable.
        package: PackageName,
    },
    /// Uninstalls a package.
    Uninstall {
        /// Device serial.
        serial: String,
        /// Package to uninstall.
        package: PackageName,
    },
}

impl Command {
    /// Stable identifier used in logs.
    pub fn name(&self) -> &'static str {
        match self {
            Self::LoadDevices => "devices.load",
            Self::LoadApps { .. } => "apps.load",
            Self::LoadProcesses { .. } => "processes.load",
            Self::LoadDozeWhitelist { .. } => "optimization.load_doze_whitelist",
            Self::LoadBackgroundExecution { .. } => "optimization.load_background_execution",
            Self::SetBackgroundExecution { .. } => "optimization.set_background_execution",
            Self::SetDozeWhitelist { .. } => "optimization.set_doze_whitelist",
            Self::LoadLabels { .. } => "labels.load",
            Self::Disable { .. } => "apps.disable",
            Self::Enable { .. } => "apps.enable",
            Self::Uninstall { .. } => "apps.uninstall",
        }
    }
}

/// Result of a completed state-changing operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationOutcome {
    /// The package is disabled.
    Disabled,
    /// The package is enabled.
    Enabled,
    /// The package was uninstalled for the current user.
    Uninstalled,
}

/// Message sent from a command thread back to the interface.
#[derive(Debug)]
pub enum CommandEvent {
    /// Device discovery result.
    Devices(Result<Vec<Device>, AdbError>),
    /// Installed package result.
    Apps {
        /// Device serial the listing belongs to.
        serial: String,
        /// Loaded packages or failure.
        result: Result<Vec<InstalledApp>, AdbError>,
    },
    /// Process table result.
    Processes {
        /// Device serial the listing belongs to.
        serial: String,
        /// Loaded processes or failure.
        result: Result<Vec<ProcessUsage>, AdbError>,
    },
    /// Doze whitelist result.
    DozeWhitelist {
        /// Device serial the listing belongs to.
        serial: String,
        /// Loaded whitelist entries or failure.
        result: Result<Vec<DozeWhitelistEntry>, AdbError>,
    },
    /// Background execution policies for the requested packages.
    BackgroundExecution {
        /// Device serial the policies belong to.
        serial: String,
        /// Policies or failure.
        result: Result<HashMap<PackageName, BackgroundExecution>, AdbError>,
    },
    /// Background execution policy update result.
    BackgroundExecutionChanged {
        /// Device serial the update belongs to.
        serial: String,
        /// Package that was updated.
        package: PackageName,
        /// Requested policy.
        policy: BackgroundExecution,
        /// Success or failure.
        result: Result<(), AdbError>,
    },
    /// Doze whitelist update result.
    DozeWhitelistChanged {
        /// Device serial the update belongs to.
        serial: String,
        /// Package that was updated.
        package: PackageName,
        /// Whether the package should now be ignored by Doze.
        whitelisted: bool,
        /// Success or failure.
        result: Result<(), AdbError>,
    },
    /// A resolved application label.
    Label {
        /// Device serial the label belongs to.
        serial: String,
        /// Package the label belongs to.
        package: PackageName,
        /// Resolved label, when it could be read.
        label: Option<String>,
    },
    /// All requested labels have been processed.
    LabelsFinished {
        /// Device serial the labels belong to.
        serial: String,
    },
    /// State-changing operation result.
    Operation {
        /// Package the operation targeted.
        package: PackageName,
        /// Outcome or failure.
        outcome: Result<OperationOutcome, AdbError>,
    },
}

/// Runs a command on a background thread.
pub fn spawn(service: Arc<dyn DeviceService>, command: Command) -> mpsc::Receiver<CommandEvent> {
    let (sender, receiver) = mpsc::channel();

    std::thread::spawn(move || {
        execute(service.as_ref(), command, &mut |event| {
            let _ = sender.send(event);
        });
    });

    receiver
}

/// Runs a command and forwards every produced event to `emit`.
pub fn execute(service: &dyn DeviceService, command: Command, emit: &mut dyn FnMut(CommandEvent)) {
    tracing::debug!(
        command = command.name(),
        details = ?command,
        "executing command"
    );

    match command {
        Command::LoadDevices => {
            emit(CommandEvent::Devices(service.devices()));
        }
        Command::LoadApps { serial } => {
            let result = service.list_apps(&serial);
            emit(CommandEvent::Apps { serial, result });
        }
        Command::LoadProcesses { serial } => {
            let result = service.processes(&serial);
            emit(CommandEvent::Processes { serial, result });
        }
        Command::LoadDozeWhitelist { serial } => {
            let result = service.doze_whitelist(&serial);
            emit(CommandEvent::DozeWhitelist { serial, result });
        }
        Command::LoadBackgroundExecution { serial, packages } => {
            let result = service.ignored_background_packages(&serial).map(|ignored| {
                packages
                    .into_iter()
                    .map(|package| {
                        let policy = if ignored.contains(&package) {
                            BackgroundExecution::Ignored
                        } else {
                            BackgroundExecution::Allowed
                        };

                        (package, policy)
                    })
                    .collect()
            });

            emit(CommandEvent::BackgroundExecution { serial, result });
        }
        Command::SetBackgroundExecution {
            serial,
            package,
            policy,
        } => {
            let result = service.set_background_execution(&serial, &package, policy);
            emit(CommandEvent::BackgroundExecutionChanged {
                serial,
                package,
                policy,
                result,
            });
        }
        Command::SetDozeWhitelist {
            serial,
            package,
            whitelisted,
        } => {
            let result = service.set_doze_whitelisted(&serial, &package, whitelisted);
            emit(CommandEvent::DozeWhitelistChanged {
                serial,
                package,
                whitelisted,
                result,
            });
        }
        Command::LoadLabels { serial, targets } => {
            for target in targets {
                if let Ok(label) = service.application_label(&serial, &target.apk_path) {
                    emit(CommandEvent::Label {
                        serial: serial.clone(),
                        package: target.package,
                        label,
                    });
                }
            }

            emit(CommandEvent::LabelsFinished { serial });
        }
        Command::Disable { serial, package } => {
            let outcome = service
                .disable_app(&serial, &package)
                .map(|()| OperationOutcome::Disabled);

            emit(CommandEvent::Operation { package, outcome });
        }
        Command::Enable { serial, package } => {
            let outcome = service
                .enable_app(&serial, &package)
                .map(|()| OperationOutcome::Enabled);

            emit(CommandEvent::Operation { package, outcome });
        }
        Command::Uninstall { serial, package } => {
            let outcome = service
                .uninstall_app(&serial, &package)
                .map(|()| OperationOutcome::Uninstalled);

            emit(CommandEvent::Operation { package, outcome });
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    struct FakeService {
        devices: Result<Vec<Device>, AdbError>,
        apps: Result<Vec<InstalledApp>, AdbError>,
        processes: Result<Vec<ProcessUsage>, AdbError>,
        doze_whitelist: Result<Vec<DozeWhitelistEntry>, AdbError>,
        set_doze_whitelist: Result<(), AdbError>,
        ignored_background: Result<std::collections::HashSet<PackageName>, AdbError>,
        set_background_execution: Result<(), AdbError>,
        label: Result<Option<String>, AdbError>,
        disable: Result<(), AdbError>,
        enable: Result<(), AdbError>,
        uninstall_results: Mutex<Vec<Result<(), AdbError>>>,
        calls: Mutex<Vec<String>>,
    }

    impl Default for FakeService {
        fn default() -> Self {
            Self {
                devices: Ok(Vec::new()),
                apps: Ok(Vec::new()),
                processes: Ok(Vec::new()),
                doze_whitelist: Ok(Vec::new()),
                set_doze_whitelist: Ok(()),
                ignored_background: Ok(std::collections::HashSet::new()),
                set_background_execution: Ok(()),
                label: Ok(None),
                disable: Ok(()),
                enable: Ok(()),
                uninstall_results: Mutex::new(Vec::new()),
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    impl FakeService {
        fn record(&self, call: impl Into<String>) {
            self.calls.lock().unwrap().push(call.into());
        }
    }

    impl DeviceService for FakeService {
        fn devices(&self) -> Result<Vec<Device>, AdbError> {
            self.record("devices");
            match &self.devices {
                Ok(devices) => Ok(devices.clone()),
                Err(error) => Err(error.clone()),
            }
        }

        fn list_apps(&self, serial: &str) -> Result<Vec<InstalledApp>, AdbError> {
            self.record(format!("list_apps({serial})"));
            match &self.apps {
                Ok(apps) => Ok(apps.clone()),
                Err(error) => Err(error.clone()),
            }
        }

        fn processes(&self, serial: &str) -> Result<Vec<ProcessUsage>, AdbError> {
            self.record(format!("processes({serial})"));
            match &self.processes {
                Ok(processes) => Ok(processes.clone()),
                Err(error) => Err(error.clone()),
            }
        }

        fn doze_whitelist(&self, serial: &str) -> Result<Vec<DozeWhitelistEntry>, AdbError> {
            self.record(format!("doze_whitelist({serial})"));
            match &self.doze_whitelist {
                Ok(entries) => Ok(entries.clone()),
                Err(error) => Err(error.clone()),
            }
        }

        fn set_doze_whitelisted(
            &self,
            serial: &str,
            package: &PackageName,
            whitelisted: bool,
        ) -> Result<(), AdbError> {
            self.record(format!(
                "set_doze_whitelist({serial},{package},{whitelisted})"
            ));
            self.set_doze_whitelist.clone()
        }

        fn ignored_background_packages(
            &self,
            serial: &str,
        ) -> Result<std::collections::HashSet<PackageName>, AdbError> {
            self.record(format!("ignored_background_packages({serial})"));
            self.ignored_background.clone()
        }

        fn set_background_execution(
            &self,
            serial: &str,
            package: &PackageName,
            policy: BackgroundExecution,
        ) -> Result<(), AdbError> {
            self.record(format!(
                "set_background_execution({serial},{package},{policy:?})"
            ));
            self.set_background_execution.clone()
        }

        fn application_label(
            &self,
            serial: &str,
            apk_path: &str,
        ) -> Result<Option<String>, AdbError> {
            self.record(format!("label({serial},{apk_path})"));
            match &self.label {
                Ok(label) => Ok(label.clone()),
                Err(error) => Err(error.clone()),
            }
        }

        fn disable_app(&self, serial: &str, package: &PackageName) -> Result<(), AdbError> {
            self.record(format!("disable({serial},{package})"));
            match &self.disable {
                Ok(()) => Ok(()),
                Err(error) => Err(error.clone()),
            }
        }

        fn enable_app(&self, serial: &str, package: &PackageName) -> Result<(), AdbError> {
            self.record(format!("enable({serial},{package})"));
            match &self.enable {
                Ok(()) => Ok(()),
                Err(error) => Err(error.clone()),
            }
        }

        fn uninstall_app(&self, serial: &str, package: &PackageName) -> Result<(), AdbError> {
            self.record(format!("uninstall({serial},{package})"));
            let mut results = self.uninstall_results.lock().unwrap();
            if results.is_empty() {
                return Ok(());
            }
            results.remove(0)
        }
    }

    fn package() -> PackageName {
        PackageName::parse("com.example").unwrap()
    }

    fn collect(service: &FakeService, command: Command) -> Vec<CommandEvent> {
        let mut events = Vec::new();
        execute(service, command, &mut |event| events.push(event));
        events
    }

    #[test]
    fn exposes_stable_command_identifiers() {
        assert_eq!(Command::LoadDevices.name(), "devices.load");
        assert_eq!(
            Command::LoadApps {
                serial: "serial".to_owned()
            }
            .name(),
            "apps.load"
        );
        assert_eq!(
            Command::LoadProcesses {
                serial: "serial".to_owned()
            }
            .name(),
            "processes.load"
        );
        assert_eq!(
            Command::LoadDozeWhitelist {
                serial: "serial".to_owned()
            }
            .name(),
            "optimization.load_doze_whitelist"
        );
        assert_eq!(
            Command::LoadBackgroundExecution {
                serial: "serial".to_owned(),
                packages: vec![package()],
            }
            .name(),
            "optimization.load_background_execution"
        );
        assert_eq!(
            Command::SetBackgroundExecution {
                serial: "serial".to_owned(),
                package: package(),
                policy: BackgroundExecution::Ignored,
            }
            .name(),
            "optimization.set_background_execution"
        );
        assert_eq!(
            Command::SetDozeWhitelist {
                serial: "serial".to_owned(),
                package: package(),
                whitelisted: true,
            }
            .name(),
            "optimization.set_doze_whitelist"
        );
        assert_eq!(
            Command::Disable {
                serial: "serial".to_owned(),
                package: package()
            }
            .name(),
            "apps.disable"
        );
    }

    #[test]
    fn forwards_device_listing() {
        let service = FakeService {
            devices: Ok(vec![Device {
                serial: "emulator-5554".to_owned(),
                state: crate::adb::DeviceState::Online,
                model: Some("Pixel".to_owned()),
            }]),
            ..Default::default()
        };

        let events = collect(&service, Command::LoadDevices);

        assert!(matches!(
            &events[0],
            CommandEvent::Devices(Ok(devices)) if devices.len() == 1
        ));
    }

    #[test]
    fn forwards_process_listing() {
        let service = FakeService {
            processes: Ok(vec![ProcessUsage {
                pid: 42,
                cpu_percent: 12.5,
                memory_bytes: 1024,
                cpu_time: "00:00.01".to_owned(),
                command: "com.example".to_owned(),
            }]),
            ..Default::default()
        };

        let events = collect(
            &service,
            Command::LoadProcesses {
                serial: "serial".to_owned(),
            },
        );

        assert!(matches!(
            &events[0],
            CommandEvent::Processes {
                result: Ok(processes),
                ..
            } if processes.len() == 1 && processes[0].pid == 42
        ));
        assert_eq!(
            service.calls.lock().unwrap().as_slice(),
            ["processes(serial)"]
        );
    }

    #[test]
    fn forwards_doze_whitelist() {
        let service = FakeService {
            doze_whitelist: Ok(vec![DozeWhitelistEntry {
                package: package(),
                source: android_utility_core::DozeWhitelistSource::User,
            }]),
            ..Default::default()
        };

        let events = collect(
            &service,
            Command::LoadDozeWhitelist {
                serial: "serial".to_owned(),
            },
        );

        assert!(matches!(
            &events[0],
            CommandEvent::DozeWhitelist {
                result: Ok(entries),
                ..
            } if entries.len() == 1 && entries[0].package == package()
        ));
        assert_eq!(
            service.calls.lock().unwrap().as_slice(),
            ["doze_whitelist(serial)"]
        );
    }

    #[test]
    fn forwards_background_execution_with_one_bulk_query() {
        let other = PackageName::parse("com.other").unwrap();
        let service = FakeService {
            ignored_background: Ok([package()].into_iter().collect()),
            ..Default::default()
        };

        let events = collect(
            &service,
            Command::LoadBackgroundExecution {
                serial: "serial".to_owned(),
                packages: vec![package(), other.clone()],
            },
        );

        assert_eq!(events.len(), 1);
        match &events[0] {
            CommandEvent::BackgroundExecution {
                result: Ok(policies),
                ..
            } => {
                assert_eq!(policies[&package()], BackgroundExecution::Ignored);
                assert_eq!(policies[&other], BackgroundExecution::Allowed);
            }
            other => panic!("unexpected event: {other:?}"),
        }
        assert_eq!(
            service.calls.lock().unwrap().as_slice(),
            ["ignored_background_packages(serial)"]
        );
    }

    #[test]
    fn forwards_background_execution_updates() {
        let service = FakeService::default();

        let events = collect(
            &service,
            Command::SetBackgroundExecution {
                serial: "serial".to_owned(),
                package: package(),
                policy: BackgroundExecution::Allowed,
            },
        );

        assert!(matches!(
            &events[0],
            CommandEvent::BackgroundExecutionChanged {
                policy: BackgroundExecution::Allowed,
                result: Ok(()),
                ..
            }
        ));
        assert_eq!(
            service.calls.lock().unwrap().as_slice(),
            ["set_background_execution(serial,com.example,Allowed)"]
        );
    }

    #[test]
    fn forwards_doze_whitelist_updates() {
        let service = FakeService::default();

        let added = collect(
            &service,
            Command::SetDozeWhitelist {
                serial: "serial".to_owned(),
                package: package(),
                whitelisted: true,
            },
        );
        let removed = collect(
            &service,
            Command::SetDozeWhitelist {
                serial: "serial".to_owned(),
                package: package(),
                whitelisted: false,
            },
        );

        assert!(matches!(
            &added[0],
            CommandEvent::DozeWhitelistChanged {
                whitelisted: true,
                result: Ok(()),
                ..
            }
        ));
        assert!(matches!(
            &removed[0],
            CommandEvent::DozeWhitelistChanged {
                whitelisted: false,
                result: Ok(()),
                ..
            }
        ));
        assert_eq!(
            service.calls.lock().unwrap().as_slice(),
            [
                "set_doze_whitelist(serial,com.example,true)",
                "set_doze_whitelist(serial,com.example,false)"
            ]
        );
    }

    #[test]
    fn forwards_application_listing_failure() {
        let service = FakeService {
            apps: Err(AdbError::command_failed("device offline")),
            ..Default::default()
        };

        let events = collect(
            &service,
            Command::LoadApps {
                serial: "emulator-5554".to_owned(),
            },
        );

        assert!(matches!(
            &events[0],
            CommandEvent::Apps {
                result: Err(AdbError::CommandFailed { .. }),
                ..
            }
        ));
    }

    #[test]
    fn emits_disable_outcome() {
        let service = FakeService::default();
        let events = collect(
            &service,
            Command::Disable {
                serial: "serial".to_owned(),
                package: package(),
            },
        );

        assert!(matches!(
            &events[0],
            CommandEvent::Operation {
                outcome: Ok(OperationOutcome::Disabled),
                ..
            }
        ));
        assert_eq!(
            service.calls.lock().unwrap().as_slice(),
            ["disable(serial,com.example)"]
        );
    }

    #[test]
    fn uninstall_targets_the_current_user_only() {
        let service = FakeService::default();

        let events = collect(
            &service,
            Command::Uninstall {
                serial: "serial".to_owned(),
                package: package(),
            },
        );

        assert!(matches!(
            &events[0],
            CommandEvent::Operation {
                outcome: Ok(OperationOutcome::Uninstalled),
                ..
            }
        ));
        assert_eq!(
            service.calls.lock().unwrap().as_slice(),
            ["uninstall(serial,com.example)"]
        );
    }

    #[test]
    fn uninstall_reports_the_device_failure() {
        let service = FakeService {
            uninstall_results: Mutex::new(vec![Err(AdbError::command_failed("failure"))]),
            ..Default::default()
        };

        let events = collect(
            &service,
            Command::Uninstall {
                serial: "serial".to_owned(),
                package: package(),
            },
        );

        match &events[0] {
            CommandEvent::Operation {
                outcome: Err(AdbError::CommandFailed { message }),
                ..
            } => {
                assert!(message.contains("failure"));
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn label_failures_are_skipped_and_completion_is_emitted() {
        let service = FakeService {
            label: Err(AdbError::command_failed("pull failed")),
            ..Default::default()
        };

        let events = collect(
            &service,
            Command::LoadLabels {
                serial: "serial".to_owned(),
                targets: vec![LabelTarget {
                    package: package(),
                    apk_path: "/data/app/base.apk".to_owned(),
                }],
            },
        );

        assert_eq!(events.len(), 1);
        assert!(matches!(events[0], CommandEvent::LabelsFinished { .. }));
    }

    #[test]
    fn resolved_labels_are_emitted() {
        let service = FakeService {
            label: Ok(Some("Example".to_owned())),
            ..Default::default()
        };

        let events = collect(
            &service,
            Command::LoadLabels {
                serial: "serial".to_owned(),
                targets: vec![LabelTarget {
                    package: package(),
                    apk_path: "/data/app/base.apk".to_owned(),
                }],
            },
        );

        assert_eq!(events.len(), 2);
        assert!(matches!(
            &events[0],
            CommandEvent::Label { package: event_package, label: Some(label), .. }
                if event_package == &package() && label == "Example"
        ));
        assert!(matches!(events[1], CommandEvent::LabelsFinished { .. }));
    }
}
