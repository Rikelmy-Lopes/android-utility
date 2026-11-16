use std::collections::HashSet;

use eframe::egui;

use android_utility_core::{InstalledApp, PackageName};

use crate::i18n::Translations;

/// Height allocated for one application row, in points.
///
/// Rows are rendered with fixed height so the list can be virtualized and only
/// the visible range is laid out each frame.
const ROW_HEIGHT: f32 = 44.0;

/// Read-only state needed to render the application list.
pub struct AppsPanel<'a> {
    /// Applications to display.
    pub apps: &'a [InstalledApp],
    /// Search query edited by the user.
    pub search: &'a mut String,
    /// Packages with an operation in progress.
    pub busy: &'a HashSet<PackageName>,
    /// Whether the application list is being loaded.
    pub loading: bool,
    /// Whether application labels are being resolved.
    pub loading_labels: bool,
    /// Translation source.
    pub translations: &'a Translations,
}

/// Action requested by the user in the application list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppsAction {
    /// Disable a package.
    Disable(PackageName),
    /// Enable a package.
    Enable(PackageName),
    /// Uninstall a package.
    Uninstall(PackageName),
    /// Read application names for the listed packages.
    LoadNames,
    /// Read the application name for one package.
    LoadName(PackageName),
}

/// Renders the application list and returns the requested action.
pub fn show(ui: &mut egui::Ui, state: AppsPanel<'_>) -> Option<AppsAction> {
    let translations = state.translations;
    let mut action = None;

    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(state.search)
                .hint_text(translations.get("apps.search_hint"))
                .desired_width(280.0),
        );

        ui.label(translations.format("apps.count", &[("count", &state.apps.len().to_string())]));

        if state.loading_labels {
            ui.spinner();
            ui.label(translations.get("apps.loading_names"));
        } else if ui
            .add_enabled(
                !state.apps.is_empty(),
                egui::Button::new(translations.get("apps.load_names"))
                    .min_size(egui::vec2(110.0, 26.0)),
            )
            .on_hover_text(translations.get("apps.load_names_tooltip"))
            .clicked()
        {
            action = Some(AppsAction::LoadNames);
        }
    });

    ui.add_space(4.0);

    if state.loading && state.apps.is_empty() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(translations.get("apps.loading"));
        });
        return None;
    }

    if state.apps.is_empty() {
        ui.label(translations.get("apps.empty"));
        return None;
    }

    let query = state.search.trim().to_lowercase();
    let matches_query = |app: &&InstalledApp| {
        query.is_empty()
            || app.display_name().to_lowercase().contains(&query)
            || app.package.as_str().contains(&query)
    };

    let visible: Vec<&InstalledApp> = state.apps.iter().filter(matches_query).collect();

    if visible.is_empty() {
        ui.label(translations.get("apps.no_matches"));
        return action;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, ROW_HEIGHT, visible.len(), |ui, range| {
            for app in &visible[range] {
                let is_busy = state.busy.contains(&app.package);

                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), ROW_HEIGHT),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(app.display_name()).strong());
                            ui.label(
                                egui::RichText::new(app.package.as_str())
                                    .small()
                                    .weak()
                                    .monospace(),
                            );
                        });

                        if app.label.is_none() && app.apk_path.is_some() {
                            let load = egui::Button::new(
                                egui::RichText::new(translations.get("apps.load_name")).small(),
                            )
                            .min_size(egui::vec2(86.0, 24.0));

                            if ui
                                .add_enabled(!state.loading_labels, load)
                                .on_hover_text(translations.get("apps.load_name_tooltip"))
                                .clicked()
                            {
                                action = Some(AppsAction::LoadName(app.package.clone()));
                            }
                        }

                        if app.is_system {
                            ui.label(
                                egui::RichText::new(translations.get("apps.system"))
                                    .small()
                                    .color(ui.visuals().warn_fg_color),
                            )
                            .on_hover_text(translations.get("apps.system_tooltip"));
                        }

                        if app.is_disabled {
                            ui.label(
                                egui::RichText::new(translations.get("apps.disabled"))
                                    .small()
                                    .weak(),
                            );
                        }

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let button_size = egui::vec2(104.0, 30.0);

                            let uninstall = egui::Button::new(
                                egui::RichText::new(translations.get("apps.uninstall"))
                                    .color(ui.visuals().error_fg_color),
                            )
                            .min_size(button_size);

                            if ui.add_enabled(!is_busy, uninstall).clicked() {
                                action = Some(AppsAction::Uninstall(app.package.clone()));
                            }

                            if app.is_disabled {
                                let enable = egui::Button::new(translations.get("apps.enable"))
                                    .min_size(button_size);
                                if ui.add_enabled(!is_busy, enable).clicked() {
                                    action = Some(AppsAction::Enable(app.package.clone()));
                                }
                            } else {
                                let disable = egui::Button::new(translations.get("apps.disable"))
                                    .min_size(button_size);
                                let response = ui.add_enabled(!is_busy, disable);
                                let response = if app.is_system {
                                    response.on_hover_text(
                                        translations.get("apps.disable_system_tooltip"),
                                    )
                                } else {
                                    response
                                };

                                if response.clicked() {
                                    action = Some(AppsAction::Disable(app.package.clone()));
                                }
                            }
                        });
                    },
                );
            }
        });

    action
}
