//! Command event handling for the interface state.

use std::collections::HashMap;
use std::sync::mpsc::TryRecvError;

use tracing::{debug, info};

use android_utility_core::{
    BackgroundExecution, DozeWhitelistEntry, DozeWhitelistSource, InstalledApp, PackageName,
    ProcessUsage, sort_apps, sort_processes,
};

use super::{AndroidUtilityApp, Tab};
use crate::adb::{AdbError, Device};
use crate::commands::{CommandEvent, OperationOutcome};

impl AndroidUtilityApp {
    pub(super) fn poll_events(&mut self) {
        let receivers = std::mem::take(&mut self.receivers);
        let mut remaining = Vec::new();

        for receiver in receivers {
            let mut disconnected = false;

            loop {
                match receiver.try_recv() {
                    Ok(event) => self.handle_event(event),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        disconnected = true;
                        break;
                    }
                }
            }

            if !disconnected {
                remaining.push(receiver);
            }
        }

        self.receivers.extend(remaining);
    }

    fn handle_event(&mut self, event: CommandEvent) {
        match event {
            CommandEvent::Devices(result) => self.handle_devices(result),
            CommandEvent::Apps { serial, result } => self.handle_apps(&serial, result),
            CommandEvent::Processes { serial, result } => {
                self.handle_processes(&serial, result);
            }
            CommandEvent::DozeWhitelist { serial, result } => {
                self.handle_doze_whitelist(&serial, result);
            }
            CommandEvent::DozeWhitelistChanged {
                serial,
                package,
                whitelisted,
                result,
            } => {
                self.handle_doze_whitelist_changed(&serial, package, whitelisted, result);
            }
            CommandEvent::BackgroundExecution { serial, result } => {
                self.handle_background_execution(&serial, result);
            }
            CommandEvent::BackgroundExecutionChanged {
                serial,
                package,
                policy,
                result,
            } => {
                self.handle_background_execution_changed(&serial, package, policy, result);
            }
            CommandEvent::Label {
                serial,
                package,
                label,
            } => {
                if self.selected_serial() != Some(serial.as_str()) {
                    return;
                }

                self.labels
                    .insert(package.as_str().to_owned(), label.clone());

                if let Some(name) = &label
                    && let Err(error) = self.label_cache.insert(package.as_str(), name)
                {
                    let text = self
                        .translations
                        .format("error.label_cache", &[("error", &error.to_string())]);
                    self.set_error(text);
                }

                let updated = match self.apps.iter_mut().find(|app| app.package == package) {
                    Some(app) => {
                        app.label = label;
                        true
                    }
                    None => false,
                };

                if updated {
                    sort_apps(&mut self.apps);
                }
            }
            CommandEvent::LabelsFinished { serial } => {
                if self.selected_serial().is_none()
                    || self.selected_serial() == Some(serial.as_str())
                {
                    self.labels_pending = false;
                }
            }
            CommandEvent::Operation { package, outcome } => {
                self.busy.remove(&package);

                match outcome {
                    Ok(outcome) => {
                        let name = self.display_name_for(&package);
                        apply_operation_outcome(&mut self.apps, &package, outcome);

                        let message = match outcome {
                            OperationOutcome::Disabled => self
                                .translations
                                .format("status.disabled_ok", &[("name", &name)]),
                            OperationOutcome::Enabled => self
                                .translations
                                .format("status.enabled_ok", &[("name", &name)]),
                            OperationOutcome::Uninstalled => self
                                .translations
                                .format("status.uninstalled_ok", &[("name", &name)]),
                        };

                        self.set_success(message);
                        info!(package = %package, ?outcome, "operation succeeded");
                        self.reload_apps();
                    }
                    Err(error) => {
                        let text = self.error_text(&error);
                        self.set_error(text);
                    }
                }
            }
        }
    }

    fn handle_devices(&mut self, result: Result<Vec<Device>, AdbError>) {
        self.loading_devices = false;

        match result {
            Ok(devices) => {
                let previous_serial = self
                    .selected_device_index
                    .and_then(|index| self.devices.get(index))
                    .map(|device| device.serial.clone());

                self.devices = devices;

                let desired_index = previous_serial
                    .and_then(|serial| {
                        self.devices
                            .iter()
                            .position(|device| device.serial == serial && device.is_online())
                    })
                    .or_else(|| self.devices.iter().position(Device::is_online));

                if self.selected_device_index != desired_index {
                    self.reset_device_data();
                }

                self.selected_device_index = desired_index;

                debug!(
                    device_count = self.devices.len(),
                    serial = self.selected_serial().unwrap_or("none"),
                    "device list updated"
                );

                if desired_index.is_some() {
                    self.set_info(self.translations.get("status.devices_loaded").to_owned());
                    self.load_apps();
                } else {
                    self.set_error(self.unavailable_device_text());
                }
            }
            Err(error) => {
                self.devices.clear();
                self.selected_device_index = None;
                self.reset_device_data();
                self.set_error(self.error_text(&error));
            }
        }
    }

    fn handle_apps(&mut self, serial: &str, result: Result<Vec<InstalledApp>, AdbError>) {
        if self.selected_serial() != Some(serial) {
            return;
        }

        self.loading_apps = false;

        match result {
            Ok(mut apps) => {
                for app in &mut apps {
                    if let Some(Some(label)) = self.labels.get(app.package.as_str()) {
                        app.label = Some(label.clone());
                    }
                }

                sort_apps(&mut apps);
                self.apps = apps;
                self.background_execution_loaded = false;

                let count = self.apps.len().to_string();
                debug!(apps = self.apps.len(), "applications loaded");
                self.set_info(
                    self.translations
                        .format("status.apps_loaded", &[("count", &count)]),
                );

                if self.selected_tab == Tab::Optimization {
                    if !self.doze_whitelist_loaded {
                        self.load_doze_whitelist();
                    }

                    if !self.background_execution_loaded {
                        self.load_background_execution();
                    }
                }
            }
            Err(error) => {
                self.apps.clear();
                self.set_error(self.error_text(&error));
            }
        }

        if self.apps_reload_pending {
            self.apps_reload_pending = false;
            self.load_apps();
        }
    }

    fn handle_processes(&mut self, serial: &str, result: Result<Vec<ProcessUsage>, AdbError>) {
        if self.selected_serial() != Some(serial) {
            return;
        }

        self.loading_processes = false;

        match result {
            Ok(mut processes) => {
                sort_processes(&mut processes, self.process_sort);

                if self.usage_session.is_active() {
                    self.usage_session.record(&processes);
                }

                debug!(processes = processes.len(), "process table loaded");
                self.processes = processes;
            }
            Err(error) => {
                self.set_error(self.error_text(&error));
            }
        }
    }

    fn handle_doze_whitelist(
        &mut self,
        serial: &str,
        result: Result<Vec<DozeWhitelistEntry>, AdbError>,
    ) {
        if self.selected_serial() != Some(serial) {
            return;
        }

        self.loading_doze_whitelist = false;

        match result {
            Ok(entries) => {
                self.doze_whitelist = entries
                    .into_iter()
                    .map(|entry| (entry.package, entry.source))
                    .collect();
                self.doze_whitelist_loaded = true;

                let count = self.doze_whitelist.len().to_string();
                debug!(
                    whitelisted = self.doze_whitelist.len(),
                    "doze whitelist loaded"
                );
                self.set_info(
                    self.translations
                        .format("status.doze_loaded", &[("count", &count)]),
                );
            }
            Err(error) => {
                self.doze_whitelist.clear();
                self.doze_whitelist_loaded = false;
                self.set_error(self.error_text(&error));
            }
        }
    }

    fn handle_doze_whitelist_changed(
        &mut self,
        serial: &str,
        package: PackageName,
        whitelisted: bool,
        result: Result<(), AdbError>,
    ) {
        if self.selected_serial() != Some(serial) {
            return;
        }

        self.busy.remove(&package);

        match result {
            Ok(()) => {
                if whitelisted {
                    self.doze_whitelist
                        .insert(package.clone(), DozeWhitelistSource::User);
                } else {
                    self.doze_whitelist.remove(&package);
                }

                let name = self.display_name_for(&package);
                let message_key = if whitelisted {
                    "status.doze_whitelisted_ok"
                } else {
                    "status.doze_unwhitelisted_ok"
                };
                self.set_success(self.translations.format(message_key, &[("name", &name)]));
            }
            Err(error) => {
                self.set_error(self.error_text(&error));
            }
        }
    }

    fn handle_background_execution(
        &mut self,
        serial: &str,
        result: Result<HashMap<PackageName, BackgroundExecution>, AdbError>,
    ) {
        if self.selected_serial() != Some(serial) {
            return;
        }

        self.background_execution_loaded = true;
        self.loading_background_execution = false;

        match result {
            Ok(policies) => {
                debug!(
                    packages = policies.len(),
                    "background execution policies loaded"
                );
                self.background_execution = policies;
            }
            Err(error) => {
                self.background_execution.clear();
                self.background_execution_loaded = false;
                self.set_error(self.error_text(&error));
            }
        }
    }

    fn handle_background_execution_changed(
        &mut self,
        serial: &str,
        package: PackageName,
        policy: BackgroundExecution,
        result: Result<(), AdbError>,
    ) {
        if self.selected_serial() != Some(serial) {
            return;
        }

        self.busy.remove(&package);

        match result {
            Ok(()) => {
                self.background_execution.insert(package.clone(), policy);

                let name = self.display_name_for(&package);
                let message_key = match policy {
                    BackgroundExecution::Allowed => "status.background_allowed_ok",
                    BackgroundExecution::Ignored => "status.background_ignored_ok",
                };
                self.set_success(self.translations.format(message_key, &[("name", &name)]));
            }
            Err(error) => {
                self.set_error(self.error_text(&error));
            }
        }
    }
}

/// Applies the local state change of a completed package operation.
///
/// The device listing is reloaded afterwards, but this keeps the visible row in
/// sync immediately instead of waiting for the next full listing.
fn apply_operation_outcome(
    apps: &mut Vec<InstalledApp>,
    package: &PackageName,
    outcome: OperationOutcome,
) {
    match outcome {
        OperationOutcome::Disabled => {
            if let Some(app) = apps.iter_mut().find(|app| &app.package == package) {
                app.is_disabled = true;
            }
        }
        OperationOutcome::Enabled => {
            if let Some(app) = apps.iter_mut().find(|app| &app.package == package) {
                app.is_disabled = false;
            }
        }
        OperationOutcome::Uninstalled => {
            apps.retain(|app| &app.package != package);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(package: &str, is_disabled: bool) -> InstalledApp {
        InstalledApp {
            package: PackageName::parse(package).expect("valid package name"),
            label: None,
            apk_path: None,
            is_system: false,
            is_disabled,
        }
    }

    #[test]
    fn disabling_updates_the_matching_application_only() {
        let package = PackageName::parse("com.example").expect("valid package name");
        let mut apps = vec![app("com.example", false), app("com.other", false)];

        apply_operation_outcome(&mut apps, &package, OperationOutcome::Disabled);

        assert!(apps[0].is_disabled);
        assert!(!apps[1].is_disabled);
    }

    #[test]
    fn enabling_clears_the_disabled_flag() {
        let package = PackageName::parse("com.example").expect("valid package name");
        let mut apps = vec![app("com.example", true)];

        apply_operation_outcome(&mut apps, &package, OperationOutcome::Enabled);

        assert!(!apps[0].is_disabled);
    }

    #[test]
    fn uninstalling_removes_the_application() {
        let package = PackageName::parse("com.example").expect("valid package name");
        let mut apps = vec![app("com.example", false), app("com.other", false)];

        apply_operation_outcome(&mut apps, &package, OperationOutcome::Uninstalled);

        assert_eq!(apps.len(), 1);
        assert_eq!(apps[0].package.as_str(), "com.other");
    }

    #[test]
    fn unknown_package_leaves_the_list_unchanged() {
        let package = PackageName::parse("com.missing").expect("valid package name");
        let mut apps = vec![app("com.example", false)];

        apply_operation_outcome(&mut apps, &package, OperationOutcome::Disabled);

        assert_eq!(apps, vec![app("com.example", false)]);
    }
}
