//! Loading actions and state changes triggered by the interface.

use tracing::{debug, info};

use android_utility_core::{BackgroundExecution, PackageName};

use super::{AndroidUtilityApp, ConfirmKind, OperationKind, PendingConfirmation};
use crate::commands::{self, Command, LabelTarget};
use crate::ui::apps_panel::AppsAction;
use crate::ui::optimization_panel::OptimizationAction;

impl AndroidUtilityApp {
    pub(super) fn load_devices(&mut self) {
        if self.loading_devices {
            return;
        }

        self.loading_devices = true;
        self.receivers
            .push(commands::spawn(self.service.clone(), Command::LoadDevices));
    }

    pub(super) fn load_apps(&mut self) {
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

    pub(super) fn load_processes(&mut self) {
        if self.loading_processes {
            return;
        }

        let Some(serial) = self.selected_serial().map(str::to_owned) else {
            return;
        };

        self.loading_processes = true;
        self.last_process_refresh = Some(std::time::Instant::now());
        self.receivers.push(commands::spawn(
            self.service.clone(),
            Command::LoadProcesses { serial },
        ));
    }

    pub(super) fn load_doze_whitelist(&mut self) {
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

    pub(super) fn load_background_execution(&mut self) {
        if self.loading_background_execution || self.apps.is_empty() {
            return;
        }

        let Some(serial) = self.selected_serial().map(str::to_owned) else {
            return;
        };

        let packages = self.apps.iter().map(|app| app.package.clone()).collect();

        self.loading_background_execution = true;
        self.receivers.push(commands::spawn(
            self.service.clone(),
            Command::LoadBackgroundExecution { serial, packages },
        ));
    }

    fn set_background_execution(&mut self, package: PackageName, policy: BackgroundExecution) {
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
            ?policy,
            "updating background execution policy"
        );
        self.set_info(self.translations.format(
            "status.background_change_started",
            &[("package", package.as_str())],
        ));

        self.receivers.push(commands::spawn(
            self.service.clone(),
            Command::SetBackgroundExecution {
                serial,
                package,
                policy,
            },
        ));
    }

    pub(super) fn clear_background_execution(&mut self) {
        self.background_execution.clear();
        self.background_execution_loaded = false;
        self.loading_background_execution = false;
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

    pub(super) fn clear_doze_whitelist(&mut self) {
        self.doze_whitelist.clear();
        self.doze_whitelist_loaded = false;
        self.loading_doze_whitelist = false;
    }

    pub(super) fn start_usage_session(&mut self) {
        self.usage_session.start();
        self.usage_summary = None;
        info!("resource usage session started");
        self.set_info(self.translations.get("status.recording_started").to_owned());
    }

    pub(super) fn stop_usage_session(&mut self) {
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

    pub(super) fn cancel_usage_session(&mut self) {
        if self.usage_session.is_active() {
            debug!("resource usage session cancelled");
            self.usage_session.cancel();
        }

        self.usage_summary = None;
    }

    pub(super) fn load_missing_labels(&mut self) {
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

    pub(super) fn start_operation(&mut self, kind: OperationKind, package: PackageName) {
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

    pub(super) fn handle_apps_action(&mut self, action: AppsAction) {
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
                self.load_names_confirmation = true;
            }
            AppsAction::LoadName(package) => {
                self.load_label(package);
            }
        }
    }

    pub(super) fn handle_optimization_action(&mut self, action: OptimizationAction) {
        match action {
            OptimizationAction::Refresh => {
                self.load_doze_whitelist();
                self.load_background_execution();
            }
            OptimizationAction::SetWhitelisted {
                package,
                whitelisted,
            } => self.set_doze_whitelisted(package, whitelisted),
            OptimizationAction::SetBackgroundExecution { package, policy } => {
                self.set_background_execution(package, policy);
            }
        }
    }
}
