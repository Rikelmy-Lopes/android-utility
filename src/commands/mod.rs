//! Application commands and their background execution.

use std::sync::Arc;
use std::sync::mpsc;

use android_utility_core::{InstalledApp, PackageName, ProcessUsage};

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
    /// The package was uninstalled completely.
    Uninstalled,
    /// The package was removed for the current user only.
    RemovedForCurrentUser,
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
            let outcome = uninstall_with_fallback(service, &serial, &package);

            emit(CommandEvent::Operation { package, outcome });
        }
    }
}

fn uninstall_with_fallback(
    service: &dyn DeviceService,
    serial: &str,
    package: &PackageName,
) -> Result<OperationOutcome, AdbError> {
    match service.uninstall_app(serial, package, false) {
        Ok(()) => Ok(OperationOutcome::Uninstalled),
        Err(full_uninstall_error) => match service.uninstall_app(serial, package, true) {
            Ok(()) => Ok(OperationOutcome::RemovedForCurrentUser),
            Err(user_uninstall_error) => Err(AdbError::command_failed(format!(
                "{full_uninstall_error}; {user_uninstall_error}"
            ))),
        },
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

        fn uninstall_app(
            &self,
            serial: &str,
            package: &PackageName,
            user_only: bool,
        ) -> Result<(), AdbError> {
            self.record(format!("uninstall({serial},{package},{user_only})"));
            let mut results = self.uninstall_results.lock().unwrap();
            if results.is_empty() {
                return Ok(());
            }
            let result = results.remove(0);
            match result {
                Ok(()) => Ok(()),
                Err(error) => Err(error),
            }
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
    fn uninstall_falls_back_to_current_user() {
        let service = FakeService {
            uninstall_results: Mutex::new(vec![
                Err(AdbError::command_failed(
                    "Failure [DELETE_FAILED_INTERNAL_ERROR]",
                )),
                Ok(()),
            ]),
            ..Default::default()
        };

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
                outcome: Ok(OperationOutcome::RemovedForCurrentUser),
                ..
            }
        ));
        assert_eq!(
            service.calls.lock().unwrap().as_slice(),
            [
                "uninstall(serial,com.example,false)",
                "uninstall(serial,com.example,true)"
            ]
        );
    }

    #[test]
    fn uninstall_reports_both_failures() {
        let service = FakeService {
            uninstall_results: Mutex::new(vec![
                Err(AdbError::command_failed("first")),
                Err(AdbError::command_failed("second")),
            ]),
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
                assert!(message.contains("first"));
                assert!(message.contains("second"));
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
