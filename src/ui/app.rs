use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, TryRecvError};
use std::time::{Duration, Instant};

use eframe::egui;
use tracing::{debug, info, warn};

use android_utility_core::{
    DozeWhitelistEntry, DozeWhitelistSource, InstalledApp, PackageName, ProcessSort, ProcessUsage,
    ResourceSession, SessionSummary, sort_apps, sort_processes,
};

use super::apps_panel::{self, AppsAction, AppsFilter, AppsPanel};
use super::confirm::{self, ConfirmKind, ConfirmResponse, PendingConfirmation};
use super::optimization_panel::{self, OptimizationAction, OptimizationPanel};
use super::processes_panel::{self, ProcessLimit, ProcessRefresh, ProcessesAction, ProcessesPanel};
use super::session_summary;
use super::theme;
use crate::adb::{Adb, AdbError, Device, DeviceService, DeviceState};
use crate::commands::{self, Command, CommandEvent, LabelTarget, OperationOutcome};
use crate::i18n::Translations;
use crate::startup::AppPaths;
use crate::startup::labels::LabelCache;
use crate::startup::settings::{Settings, ThemeMode};

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
    apps_filter: AppsFilter,
    optimization_search: String,
    optimization_filter: AppsFilter,
    doze_whitelist: HashMap<PackageName, DozeWhitelistSource>,
    doze_whitelist_loaded: bool,
    loading_doze_whitelist: bool,
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
    usage_session: ResourceSession,
    usage_summary: Option<SessionSummary>,
    theme: ThemeMode,
    settings_path: Option<std::path::PathBuf>,
    status: StatusMessage,
}

/// Top-level sections available in the interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tab {
    Apps,
    Optimization,
    Device,
}

/// Semantic category of the status bar message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatusKind {
    /// Neutral information.
    Info,
    /// Confirmed successful action.
    Success,
    /// Failed action or unavailable device.
    Error,
}

struct StatusMessage {
    text: String,
    kind: StatusKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperationKind {
    Disable,
    Enable,
    Uninstall,
}

impl AndroidUtilityApp {
    pub fn new(creation_context: &eframe::CreationContext<'_>) -> Self {
        let translations = Translations::english();

        let paths = AppPaths::for_current_user();
        let settings = match &paths {
            Ok(paths) => Settings::load(&paths.settings_file()),
            Err(_) => Settings::default(),
        };
        let settings_path = paths.as_ref().ok().map(AppPaths::settings_file);

        theme::apply(&creation_context.egui_ctx, settings.theme);

        let (service, label_cache, startup_error) = match paths
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
                kind: StatusKind::Error,
            },
            None => StatusMessage {
                text: translations.get("status.ready").to_owned(),
                kind: StatusKind::Info,
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
            apps_filter: AppsFilter::All,
            optimization_search: String::new(),
            optimization_filter: AppsFilter::All,
            doze_whitelist: HashMap::new(),
            doze_whitelist_loaded: false,
            loading_doze_whitelist: false,
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
            usage_session: ResourceSession::default(),
            usage_summary: None,
            theme: settings.theme,
            settings_path,
            status,
        };

        app.load_devices();
        app
    }

    fn set_info(&mut self, text: String) {
        self.status = StatusMessage {
            text,
            kind: StatusKind::Info,
        };
    }

    fn set_success(&mut self, text: String) {
        self.status = StatusMessage {
            text,
            kind: StatusKind::Success,
        };
    }

    fn set_error(&mut self, text: String) {
        warn!(message = %text, "operation failed");

        self.status = StatusMessage {
            text,
            kind: StatusKind::Error,
        };
    }

    fn set_theme(&mut self, context: &egui::Context, theme_mode: ThemeMode) {
        if self.theme == theme_mode {
            return;
        }

        self.theme = theme_mode;
        theme::apply(context, theme_mode);

        let Some(path) = &self.settings_path else {
            return;
        };

        if let Err(error) = (Settings { theme: theme_mode }).save(path) {
            warn!(%error, "could not save settings");
        }
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

    fn load_doze_whitelist(&mut self) {
        if self.loading_doze_whitelist {
            return;
        }

        let Some(serial) = self.selected_serial().map(str::to_owned) else {
            return;
        };

        self.loading_doze_whitelist = true;
        self.receivers.push(commands::spawn(
            self.service.clone(),
            Command::LoadDozeWhitelist { serial },
        ));
    }

    fn set_doze_whitelisted(&mut self, package: PackageName, whitelisted: bool) {
        let Some(serial) = self.selected_serial().map(str::to_owned) else {
            self.set_error(self.translations.get("error.no_device_selected").to_owned());
            return;
        };

        if self.busy.contains(&package) {
            return;
        }

        self.busy.insert(package.clone());
        info!(
            serial = %serial,
            package = %package,
            whitelisted,
            "updating doze whitelist"
        );
        self.set_info(self.translations.format(
            "status.doze_change_started",
            &[("package", package.as_str())],
        ));

        self.receivers.push(commands::spawn(
            self.service.clone(),
            Command::SetDozeWhitelist {
                serial,
                package,
                whitelisted,
            },
        ));
    }

    fn clear_doze_whitelist(&mut self) {
        self.doze_whitelist.clear();
        self.doze_whitelist_loaded = false;
        self.loading_doze_whitelist = false;
    }

    fn start_usage_session(&mut self) {
        self.usage_session.start();
        self.usage_summary = None;
        info!("resource usage session started");
        self.set_info(self.translations.get("status.recording_started").to_owned());
    }

    fn stop_usage_session(&mut self) {
        if let Some(summary) = self.usage_session.finish() {
            info!(
                updates = summary.updates,
                elapsed_ms = summary.elapsed.as_millis(),
                "resource usage session stopped"
            );
            self.usage_summary = Some(summary);
            self.set_info(self.translations.get("status.recording_stopped").to_owned());
        }
    }

    fn cancel_usage_session(&mut self) {
        if self.usage_session.is_active() {
            debug!("resource usage session cancelled");
            self.usage_session.cancel();
        }

        self.usage_summary = None;
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

    fn handle_optimization_action(&mut self, action: OptimizationAction) {
        match action {
            OptimizationAction::Refresh => self.load_doze_whitelist(),
            OptimizationAction::LoadNames => self.load_missing_labels(),
            OptimizationAction::LoadName(package) => self.load_label(package),
            OptimizationAction::SetWhitelisted {
                package,
                whitelisted,
            } => self.set_doze_whitelisted(package, whitelisted),
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

                        self.set_success(message);
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
                    self.clear_doze_whitelist();
                    self.cancel_usage_session();
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
                self.clear_doze_whitelist();
                self.cancel_usage_session();
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

                if self.selected_tab == Tab::Optimization && !self.doze_whitelist_loaded {
                    self.load_doze_whitelist();
                }
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

    fn draw_toolbar(&mut self, ui: &mut egui::Ui) {
        let mut requested_theme = None;

        egui::Panel::top("toolbar")
            .show_separator_line(true)
            .show(ui, |ui| {
                ui.add_space(theme::size::SPACING_SM);
                ui.horizontal(|ui| {
                    ui.heading(self.translations.get("app.title"));
                    ui.separator();

                    if self.devices.is_empty() {
                        ui.label(self.translations.get("app.no_devices"));
                    } else {
                        let mut selection = self.selected_device_index;
                        let selected_text = self.selected_device_text();

                        egui::ComboBox::from_id_salt("device_selector")
                            .width(theme::size::DEVICE_SELECTOR_WIDTH)
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
                            self.clear_doze_whitelist();
                            self.cancel_usage_session();
                            self.load_apps();
                        }
                    }

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let palette = theme::palette(ui);

                        if ui
                            .add_enabled(
                                !self.loading_devices,
                                theme::toned_button(
                                    self.translations.get("app.refresh"),
                                    theme::ButtonTone::Primary,
                                    palette,
                                ),
                            )
                            .clicked()
                        {
                            self.load_devices();
                        }

                        if self.loading_devices {
                            ui.spinner();
                        }

                        ui.separator();

                        let selected_is_dark = self.theme == ThemeMode::Dark;
                        let dark_tone = if selected_is_dark {
                            theme::ButtonTone::Primary
                        } else {
                            theme::ButtonTone::Neutral
                        };
                        let dark = theme::toned_button(
                            self.translations.get("theme.dark"),
                            dark_tone,
                            palette,
                        )
                        .min_size(egui::vec2(64.0, theme::size::CONTROL_HEIGHT));

                        if ui
                            .add(dark)
                            .on_hover_text(self.translations.get("theme.dark_tooltip"))
                            .clicked()
                        {
                            requested_theme = Some(ThemeMode::Dark);
                        }

                        let light_tone = if selected_is_dark {
                            theme::ButtonTone::Neutral
                        } else {
                            theme::ButtonTone::Primary
                        };
                        let light = theme::toned_button(
                            self.translations.get("theme.light"),
                            light_tone,
                            palette,
                        )
                        .min_size(egui::vec2(64.0, theme::size::CONTROL_HEIGHT));

                        if ui
                            .add(light)
                            .on_hover_text(self.translations.get("theme.light_tooltip"))
                            .clicked()
                        {
                            requested_theme = Some(ThemeMode::Light);
                        }

                        ui.label(egui::RichText::new(self.translations.get("theme.label")));
                    });
                });
                ui.add_space(theme::size::SPACING_SM);
            });

        if let Some(theme_mode) = requested_theme {
            self.set_theme(ui.ctx(), theme_mode);
        }
    }

    fn draw_status_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("status")
            .show_separator_line(true)
            .show(ui, |ui| {
                ui.add_space(theme::size::SPACING_XS);

                match self.status.kind {
                    StatusKind::Info => {
                        ui.label(&self.status.text);
                    }
                    StatusKind::Success => {
                        ui.colored_label(
                            theme::palette(ui).success_text,
                            egui::RichText::new(&self.status.text),
                        );
                    }
                    StatusKind::Error => {
                        ui.colored_label(
                            ui.visuals().error_fg_color,
                            egui::RichText::new(&self.status.text),
                        );
                    }
                }

                ui.add_space(theme::size::SPACING_XS);
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

    fn show_usage_summary(&mut self, context: &egui::Context) {
        let Some(summary) = self.usage_summary.as_ref() else {
            return;
        };

        let dismissed = session_summary::show(context, &self.translations, summary);

        if dismissed {
            self.usage_summary = None;
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
        let mut optimization_action = None;
        let translations = &self.translations;
        let previous_tab = self.selected_tab;
        let previous_sort = self.process_sort;
        let previous_interval = (self.process_refresh, self.process_custom_refresh_ms);
        let mut selected_tab = self.selected_tab;

        egui::CentralPanel::default().show(ui, |ui| {
            ui.add_space(theme::size::SPACING_SM);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut selected_tab, Tab::Apps, translations.get("tabs.apps"));
                ui.selectable_value(
                    &mut selected_tab,
                    Tab::Optimization,
                    translations.get("tabs.optimization"),
                );
                ui.selectable_value(
                    &mut selected_tab,
                    Tab::Device,
                    translations.get("tabs.process"),
                );
            });
            ui.add_space(theme::size::SPACING_XS);
            ui.separator();

            match selected_tab {
                Tab::Apps => {
                    action = apps_panel::show(
                        ui,
                        AppsPanel {
                            apps: &self.apps,
                            filter: &mut self.apps_filter,
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
                            recording: self.usage_session.is_active(),
                            recorded_updates: self.usage_session.updates(),
                            translations,
                        },
                    );
                }
                Tab::Optimization => {
                    optimization_action = optimization_panel::show(
                        ui,
                        OptimizationPanel {
                            apps: &self.apps,
                            whitelist: &self.doze_whitelist,
                            filter: &mut self.optimization_filter,
                            search: &mut self.optimization_search,
                            busy: &self.busy,
                            loading_apps: self.loading_apps,
                            loading_whitelist: self.loading_doze_whitelist,
                            whitelist_loaded: self.doze_whitelist_loaded,
                            loading_labels: self.labels_pending,
                            translations,
                        },
                    );
                }
            }
        });

        self.selected_tab = selected_tab;

        if previous_tab != Tab::Optimization && selected_tab == Tab::Optimization {
            self.load_doze_whitelist();
        }

        if selected_tab != Tab::Device {
            self.cancel_usage_session();
        }

        if let Some(action) = optimization_action {
            self.handle_optimization_action(action);
        }

        if self.process_sort != previous_sort {
            sort_processes(&mut self.processes, self.process_sort);
        }

        if (self.process_refresh, self.process_custom_refresh_ms) != previous_interval {
            self.last_process_refresh = None;
        }

        if let Some(action) = action {
            self.handle_apps_action(action);
        }

        match process_action {
            Some(ProcessesAction::Refresh) => self.load_processes(),
            Some(ProcessesAction::StartRecording) => self.start_usage_session(),
            Some(ProcessesAction::StopRecording) => self.stop_usage_session(),
            None => {}
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
        self.show_usage_summary(ui.ctx());

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
