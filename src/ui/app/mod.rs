use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

use eframe::egui;
use tracing::{debug, warn};

use android_utility_core::{
    BackgroundExecution, DozeWhitelistSource, InstalledApp, PackageName, ProcessSort, ProcessUsage,
    ResourceSession, SessionSummary, sort_processes,
};

use super::apps_panel::{self, AppsFilter, AppsPanel, AppsStatusFilter};
use super::confirm::{self, ConfirmKind, ConfirmResponse, PendingConfirmation};
use super::optimization_panel::{self, BackgroundFilter, DozeFilter, OptimizationPanel};
use super::processes_panel::{self, ProcessLimit, ProcessRefresh, ProcessesAction, ProcessesPanel};
use super::session_summary;
use super::theme;
use crate::adb::{Adb, AdbError, Device, DeviceService, DeviceState};
use crate::i18n::Translations;
use crate::startup::AppPaths;
use crate::startup::labels::LabelCache;
use crate::startup::settings::{Settings, ThemeMode};

mod actions;
mod events;

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
    apps_status_filter: AppsStatusFilter,
    optimization_search: String,
    optimization_filter: AppsFilter,
    optimization_background_filter: BackgroundFilter,
    optimization_doze_filter: DozeFilter,
    doze_whitelist: HashMap<PackageName, DozeWhitelistSource>,
    doze_whitelist_loaded: bool,
    loading_doze_whitelist: bool,
    background_execution: HashMap<PackageName, BackgroundExecution>,
    background_execution_loaded: bool,
    loading_background_execution: bool,
    receivers: Vec<Receiver<crate::commands::CommandEvent>>,
    busy: HashSet<PackageName>,
    loading_devices: bool,
    loading_apps: bool,
    apps_reload_pending: bool,
    labels_pending: bool,
    confirmation: Option<PendingConfirmation>,
    load_names_confirmation: bool,
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
            apps_status_filter: AppsStatusFilter::All,
            optimization_search: String::new(),
            optimization_filter: AppsFilter::All,
            optimization_background_filter: BackgroundFilter::All,
            optimization_doze_filter: DozeFilter::All,
            doze_whitelist: HashMap::new(),
            doze_whitelist_loaded: false,
            loading_doze_whitelist: false,
            background_execution: HashMap::new(),
            background_execution_loaded: false,
            loading_background_execution: false,
            receivers: Vec::new(),
            busy: HashSet::new(),
            loading_devices: false,
            loading_apps: false,
            apps_reload_pending: false,
            labels_pending: false,
            confirmation: None,
            load_names_confirmation: false,
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
            AdbError::UnsupportedCommand { command, message } => self.translations.format(
                "error.unsupported_command",
                &[("command", command), ("message", message.as_str())],
            ),
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

    /// Clears every value that belongs to the previously selected device.
    fn reset_device_data(&mut self) {
        self.apps.clear();
        self.loading_apps = false;
        self.apps_reload_pending = false;
        self.labels_pending = false;
        self.processes.clear();
        self.loading_processes = false;
        self.last_process_refresh = None;
        self.clear_doze_whitelist();
        self.clear_background_execution();
        self.cancel_usage_session();
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
                            self.reset_device_data();
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
        let show_apps_total = matches!(self.selected_tab, Tab::Apps | Tab::Optimization);
        let total_text = self
            .translations
            .format("apps.count", &[("count", &self.apps.len().to_string())]);

        egui::Panel::bottom("status")
            .show_separator_line(true)
            .show(ui, |ui| {
                ui.add_space(theme::size::SPACING_XS);

                ui.horizontal(|ui| {
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

                    if show_apps_total {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new(&total_text).weak());
                        });
                    }
                });

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

    fn show_load_names_confirmation(&mut self, context: &egui::Context) {
        if !self.load_names_confirmation {
            return;
        }

        match confirm::show_load_names(context, &self.translations) {
            Some(ConfirmResponse::Confirmed) => {
                self.load_names_confirmation = false;
                self.load_missing_labels();
            }
            Some(ConfirmResponse::Cancelled) => {
                self.load_names_confirmation = false;
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
                            status_filter: &mut self.apps_status_filter,
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
                            background: &self.background_execution,
                            filter: &mut self.optimization_filter,
                            background_filter: &mut self.optimization_background_filter,
                            doze_filter: &mut self.optimization_doze_filter,
                            search: &mut self.optimization_search,
                            busy: &self.busy,
                            loading_apps: self.loading_apps,
                            loading_whitelist: self.loading_doze_whitelist,
                            loading_background: self.loading_background_execution,
                            whitelist_loaded: self.doze_whitelist_loaded,
                            background_loaded: self.background_execution_loaded,
                            translations,
                        },
                    );
                }
            }
        });

        self.selected_tab = selected_tab;

        if previous_tab != Tab::Optimization && selected_tab == Tab::Optimization {
            self.load_doze_whitelist();
            self.load_background_execution();
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
        self.show_load_names_confirmation(ui.ctx());
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
