use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use eframe::egui;
use tracing::{debug, info, warn};

use android_utility_core::{
    InstalledApp, PackageName, ProcessSort, ProcessUsage, sort_apps, sort_processes,
};

use super::apps_panel::{self, AppsAction, AppsPanel};
use super::confirm::{self, ConfirmKind, ConfirmResponse, PendingConfirmation};
use super::processes_panel::{self, ProcessLimit, ProcessRefresh, ProcessesAction, ProcessesPanel};
use crate::adb::{Adb, AdbError, Device, DeviceService, DeviceState};
use crate::commands::{self, Command, CommandEvent, LabelTarget, OperationOutcome};
use crate::i18n::Translations;
use crate::startup::AppPaths;
use crate::startup::labels::LabelCache;

/// Top-level application state rendered by the `egui` interface.
pub struct AndroidUtilityApp {
    service: Arc<dyn DeviceService>,
    translations: Translations,
    devices: Vec<Device>,
    selected_device_index: Option<usize>,
    apps: Vec<InstalledApp>,
    labels: HashMap<String, Option<String>>,
    label_cache: LabelCache,
    search: String,
    receivers: Vec<Receiver<CommandEvent>>,
    busy: HashSet<PackageName>,
    loading_devices: bool,
    loading_apps: bool,
    labels_pending: bool,
    confirmation: Option<PendingConfirmation>,
    selected_tab: Tab,
    processes: Vec<ProcessUsage>,
    process_sort: ProcessSort,
    process_refresh: ProcessRefresh,
    process_custom_refresh_ms: u64,
    process_limit: ProcessLimit,
    loading_processes: bool,
    last_process_refresh: Option<Instant>,
    status: StatusMessage,
}

/// Top-level sections available in the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Apps,
    Device,
}

struct StatusMessage {
    text: String,
    is_error: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperationKind {
    Disable,
    Enable,
    Uninstall,
}

impl AndroidUtilityApp {
    pub fn new(_creation_context: &eframe::CreationContext<'_>) -> Self {
        let translations = Translations::english();

        let (service, label_cache, startup_error) = match AppPaths::for_current_user()
            .and_then(|paths| paths.ensure_cache_dir())
        {
            Ok(cache_dir) => (
                Arc::new(Adb::new().with_cache_dir(cache_dir.clone())) as Arc<dyn DeviceService>,
                LabelCache::load(&cache_dir),
                None,
            ),
            Err(error) => {
                tracing::warn!(%error, "cache directory unavailable");

                (
                    Arc::new(Adb::new()) as Arc<dyn DeviceService>,
                    LabelCache::disabled(),
                    Some(
                        translations
                            .format("error.cache_directory", &[("error", &error.to_string())]),
                    ),
                )
            }
        };

        let labels = label_cache
            .labels()
            .iter()
            .map(|(package, name)| (package.clone(), Some(name.clone())))
            .collect();

        let status = match startup_error {
            Some(text) => StatusMessage {
                text,
                is_error: true,
            },
            None => StatusMessage {
                text: translations.get("status.ready").to_owned(),
                is_error: false,
            },
        };

        let mut app = Self {
            service,
            translations,
            devices: Vec::new(),
            selected_device_index: None,
            apps: Vec::new(),
            labels,
            label_cache,
            search: String::new(),
            receivers: Vec::new(),
            busy: HashSet::new(),
            loading_devices: false,
            loading_apps: false,
            labels_pending: false,
            confirmation: None,
            selected_tab: Tab::Apps,
            processes: Vec::new(),
            process_sort: ProcessSort::Cpu,
            process_refresh: ProcessRefresh::Preset(2000),
            process_custom_refresh_ms: 2000,
            process_limit: ProcessLimit::All,
            loading_processes: false,
            last_process_refresh: None,
            status,
        };

        app.load_devices();
        app
    }

    fn set_info(&mut self, text: String) {
        self.status = StatusMessage {
            text,
            is_error: false,
        };
    }

    fn set_error(&mut self, text: String) {
        warn!(message = %text, "operation failed");

        self.status = StatusMessage {
            text,
            is_error: true,
        };
    }

    fn selected_serial(&self) -> Option<&str> {
        self.selected_device_index
            .and_then(|index| self.devices.get(index))
            .filter(|device| device.is_online())
            .map(|device| device.serial.as_str())
    }

    fn selected_device_text(&self) -> String {
        match self
            .selected_device_index
            .and_then(|index| self.devices.get(index))
        {
            Some(device) => format!(
                "{} · {}",
                device.name(),
                self.translations.get(device_state_key(device.state))
            ),
            None => self.translations.get("app.no_devices").to_owned(),
        }
    }

    fn error_text(&self, error: &AdbError) -> String {
        match error {
            AdbError::AdbNotFound { .. } => self.translations.get("error.adb_not_found").to_owned(),
            AdbError::PermissionDenied { message } => self
                .translations
                .format("error.permission_denied", &[("message", message)]),
            _ => format!("{} {}", self.translations.get("error.generic"), error),
        }
    }

    fn unavailable_device_text(&self) -> String {
        match self.devices.first() {
            Some(device) => self
                .translations
                .get(device_state_key(device.state))
                .to_owned(),
            None => self.translations.get("app.no_devices").to_owned(),
        }
    }

    fn display_name_for(&self, package: &PackageName) -> String {
        self.apps
            .iter()
            .find(|app| &app.package == package)
            .map(|app| app.display_name().to_owned())
            .unwrap_or_else(|| package.to_string())
    }

    fn load_devices(&mut self) {
        if self.loading_devices {
            return;
        }

        self.loading_devices = true;
        self.receivers
            .push(commands::spawn(self.service.clone(), Command::LoadDevices));
    }

    fn load_apps(&mut self) {
        let Some(serial) = self.selected_serial().map(str::to_owned) else {
            self.set_error(self.translations.get("error.no_device_selected").to_owned());
            return;
        };

        if self.loading_apps {
            return;
        }

        self.loading_apps = true;
        self.receivers.push(commands::spawn(
            self.service.clone(),
            Command::LoadApps { serial },
        ));
    }

    fn load_processes(&mut self) {
        if self.loading_processes {
            return;
        }

        let Some(serial) = self.selected_serial().map(str::to_owned) else {
            return;
        };

        self.loading_processes = true;
        self.last_process_refresh = Some(Instant::now());
        self.receivers.push(commands::spawn(
            self.service.clone(),
            Command::LoadProcesses { serial },
        ));
    }

    fn load_missing_labels(&mut self) {
        if self.labels_pending {
            return;
        }

        let Some(serial) = self.selected_serial().map(str::to_owned) else {
            return;
        };

        let targets: Vec<LabelTarget> = self
            .apps
            .iter()
            .filter(|app| !self.labels.contains_key(app.package.as_str()))
            .filter_map(|app| {
                app.apk_path.as_ref().map(|path| LabelTarget {
                    package: app.package.clone(),
                    apk_path: path.clone(),
                })
            })
            .collect();

        if targets.is_empty() {
            return;
        }

        self.labels_pending = true;
        self.receivers.push(commands::spawn(
            self.service.clone(),
            Command::LoadLabels { serial, targets },
        ));
    }

    fn load_label(&mut self, package: PackageName) {
        if self.labels.contains_key(package.as_str()) {
            return;
        }

        let Some(serial) = self.selected_serial().map(str::to_owned) else {
            return;
        };

        let Some(target) = self
            .apps
            .iter()
            .find(|app| app.package == package)
            .and_then(|app| {
                app.apk_path.as_ref().map(|path| LabelTarget {
                    package: package.clone(),
                    apk_path: path.clone(),
                })
            })
        else {
            return;
        };

        self.receivers.push(commands::spawn(
            self.service.clone(),
            Command::LoadLabels {
                serial,
                targets: vec![target],
            },
        ));
    }

    fn start_operation(&mut self, kind: OperationKind, package: PackageName) {
        let Some(serial) = self.selected_serial().map(str::to_owned) else {
            self.set_error(self.translations.get("error.no_device_selected").to_owned());
            return;
        };

        if self.busy.contains(&package) {
            return;
        }

        self.busy.insert(package.clone());
        info!(
            operation = ?kind,
            serial = %serial,
            package = %package,
            "starting operation"
        );
        self.set_info(
            self.translations
                .format("status.operation_started", &[("package", package.as_str())]),
        );

        let command = match kind {
            OperationKind::Disable => Command::Disable { serial, package },
            OperationKind::Enable => Command::Enable { serial, package },
            OperationKind::Uninstall => Command::Uninstall { serial, package },
        };

        self.receivers
            .push(commands::spawn(self.service.clone(), command));
    }

    fn request_confirmation(&mut self, kind: ConfirmKind, package: PackageName) {
        let (display_name, is_system) = match self.apps.iter().find(|app| app.package == package) {
            Some(app) => (app.display_name().to_owned(), app.is_system),
            None => (package.to_string(), false),
        };

        self.confirmation = Some(PendingConfirmation {
            kind,
            package,
            display_name,
            is_system,
        });
    }

    fn handle_apps_action(&mut self, action: AppsAction) {
        match action {
            AppsAction::Disable(package) => {
                self.request_confirmation(ConfirmKind::Disable, package);
            }
            AppsAction::Enable(package) => {
                self.start_operation(OperationKind::Enable, package);
            }
            AppsAction::Uninstall(package) => {
                self.request_confirmation(ConfirmKind::Uninstall, package);
            }
            AppsAction::LoadNames => {
                self.load_missing_labels();
            }
            AppsAction::LoadName(package) => {
                self.load_label(package);
            }
        }
    }

    fn poll_events(&mut self) {
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

                        self.set_info(message);
                        info!(package = %package, ?outcome, "operation succeeded");
                        self.load_apps();
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
                    self.apps.clear();
                    self.loading_apps = false;
                    self.labels_pending = false;
                    self.processes.clear();
                    self.loading_processes = false;
                    self.last_process_refresh = None;
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
                self.apps.clear();
                self.processes.clear();
                self.loading_processes = false;
                self.last_process_refresh = None;
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

                let count = self.apps.len().to_string();
                debug!(apps = self.apps.len(), "applications loaded");
                self.set_info(
                    self.translations
                        .format("status.apps_loaded", &[("count", &count)]),
                );
            }
            Err(error) => {
                self.apps.clear();
                self.set_error(self.error_text(&error));
            }
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
                debug!(processes = processes.len(), "process table loaded");
                self.processes = processes;
            }
            Err(error) => {
                self.set_error(self.error_text(&error));
            }
        }
    }

    fn draw_toolbar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("toolbar").show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.heading(self.translations.get("app.title"));
                ui.separator();

                if self.devices.is_empty() {
                    ui.label(self.translations.get("app.no_devices"));
                } else {
                    let mut selection = self.selected_device_index;
                    let selected_text = self.selected_device_text();

                    egui::ComboBox::from_id_salt("device_selector")
                        .width(260.0)
                        .selected_text(selected_text)
                        .show_ui(ui, |ui| {
                            for (index, device) in self.devices.iter().enumerate() {
                                let label = format!(
                                    "{} · {}",
                                    device.name(),
                                    self.translations.get(device_state_key(device.state))
                                );

                                ui.selectable_value(&mut selection, Some(index), label);
                            }
                        });

                    if selection != self.selected_device_index {
                        debug!(
                            previous = ?self.selected_device_index,
                            selected = ?selection,
                            "device selection changed"
                        );

                        self.selected_device_index = selection;
                        self.apps.clear();
                        self.loading_apps = false;
                        self.labels_pending = false;
                        self.processes.clear();
                        self.loading_processes = false;
                        self.last_process_refresh = None;
                        self.load_apps();
                    }
                }

                if ui
                    .add_enabled(
                        !self.loading_devices,
                        egui::Button::new(self.translations.get("app.refresh")),
                    )
                    .clicked()
                {
                    self.load_devices();
                }

                if self.loading_devices {
                    ui.spinner();
                }
            });
            ui.add_space(4.0);
        });
    }

    fn draw_status_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("status").show(ui, |ui| {
            if self.status.is_error {
                ui.colored_label(ui.visuals().error_fg_color, &self.status.text);
            } else {
                ui.label(&self.status.text);
            }
        });
    }

    fn show_confirmation(&mut self, context: &egui::Context) {
        let Some(pending) = self.confirmation.clone() else {
            return;
        };

        match confirm::show(context, &self.translations, &pending) {
            Some(ConfirmResponse::Confirmed) => {
                self.confirmation = None;

                let kind = match pending.kind {
                    ConfirmKind::Disable => OperationKind::Disable,
                    ConfirmKind::Uninstall => OperationKind::Uninstall,
                };

                self.start_operation(kind, pending.package);
            }
            Some(ConfirmResponse::Cancelled) => {
                self.confirmation = None;
            }
            None => {}
        }
    }
}

impl eframe::App for AndroidUtilityApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.poll_events();
        self.draw_toolbar(ui);
        self.draw_status_bar(ui);

        let mut action = None;
        let mut process_action = None;
        let translations = &self.translations;
        let previous_tab = self.selected_tab;
        let previous_sort = self.process_sort;
        let previous_interval = (self.process_refresh, self.process_custom_refresh_ms);
        let mut selected_tab = self.selected_tab;

        egui::CentralPanel::default().show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.selectable_value(&mut selected_tab, Tab::Apps, translations.get("tabs.apps"));
                ui.selectable_value(
                    &mut selected_tab,
                    Tab::Device,
                    translations.get("tabs.device"),
                );
            });
            ui.separator();

            match selected_tab {
                Tab::Apps => {
                    action = apps_panel::show(
                        ui,
                        AppsPanel {
                            apps: &self.apps,
                            search: &mut self.search,
                            busy: &self.busy,
                            loading: self.loading_apps,
                            loading_labels: self.labels_pending,
                            translations,
                        },
                    );
                }
                Tab::Device => {
                    process_action = processes_panel::show(
                        ui,
                        ProcessesPanel {
                            processes: &self.processes,
                            sort: &mut self.process_sort,
                            refresh: &mut self.process_refresh,
                            custom_refresh_ms: &mut self.process_custom_refresh_ms,
                            limit: &mut self.process_limit,
                            loading: self.loading_processes,
                            translations,
                        },
                    );
                }
            }
        });

        self.selected_tab = selected_tab;

        if self.process_sort != previous_sort {
            sort_processes(&mut self.processes, self.process_sort);
        }

        if (self.process_refresh, self.process_custom_refresh_ms) != previous_interval {
            self.last_process_refresh = None;
        }

        if let Some(action) = action {
            self.handle_apps_action(action);
        }

        if process_action == Some(ProcessesAction::Refresh) {
            self.load_processes();
        }

        let process_interval = Duration::from_millis(
            self.process_refresh
                .interval_ms(self.process_custom_refresh_ms),
        );
        let entered_device_tab = previous_tab != Tab::Device && selected_tab == Tab::Device;
        let refresh_due = process_refresh_due(self.last_process_refresh, process_interval);

        if selected_tab == Tab::Device && (entered_device_tab || refresh_due) {
            self.load_processes();
        }

        self.show_confirmation(ui.ctx());

        let repaint_delay = if selected_tab == Tab::Device {
            let wait = self
                .last_process_refresh
                .map(|last| process_interval.saturating_sub(last.elapsed()))
                .unwrap_or(process_interval);

            Some(wait.max(Duration::from_millis(processes_panel::MIN_REFRESH_MS)))
        } else {
            None
        };

        let repaint_delay = if self.receivers.is_empty() {
            repaint_delay
        } else {
            Some(
                repaint_delay
                    .unwrap_or(Duration::from_millis(100))
                    .min(Duration::from_millis(100)),
            )
        };

        if let Some(delay) = repaint_delay {
            ui.ctx().request_repaint_after(delay);
        }
    }
}

fn device_state_key(state: DeviceState) -> &'static str {
    match state {
        DeviceState::Online => "app.device_online",
        DeviceState::Offline => "app.device_offline",
        DeviceState::Unauthorized => "app.device_unauthorized",
        DeviceState::NoPermissions => "app.device_no_permissions",
        DeviceState::Unknown => "app.device_unknown_state",
    }
}

fn process_refresh_due(last: Option<Instant>, interval: Duration) -> bool {
    last.is_none_or(|last| last.elapsed() >= interval)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_is_due_without_a_previous_snapshot() {
        assert!(process_refresh_due(None, Duration::from_secs(2)));
    }

    #[test]
    fn refresh_waits_for_the_configured_interval() {
        let now = Instant::now();
        let recent = now
            .checked_sub(Duration::from_millis(500))
            .expect("instant");

        assert!(!process_refresh_due(Some(recent), Duration::from_secs(2)));
        assert!(process_refresh_due(
            Some(recent),
            Duration::from_millis(400)
        ));
    }
}
