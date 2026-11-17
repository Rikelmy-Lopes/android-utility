use std::collections::HashSet;

use eframe::egui;

use android_utility_core::{InstalledApp, PackageName};

use crate::i18n::Translations;

use super::theme;

/// Package subset selected by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppsFilter {
    /// Show every installed package.
    All,
    /// Show packages that belong to the system image.
    System,
    /// Show packages installed by the user, excluding system packages.
    Installed,
}

impl AppsFilter {
    /// Filters offered to the user.
    pub const OPTIONS: [Self; 3] = [Self::All, Self::System, Self::Installed];

    /// Returns whether `app` belongs to the selected subset.
    pub fn matches(self, app: &InstalledApp) -> bool {
        match self {
            Self::All => true,
            Self::System => app.is_system,
            Self::Installed => !app.is_system,
        }
    }
}

/// Read-only state needed to render the application list.
pub struct AppsPanel<'a> {
    /// Applications to display.
    pub apps: &'a [InstalledApp],
    /// Package subset selected by the user.
    pub filter: &'a mut AppsFilter,
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
    let palette = theme::palette(ui);
    let mut action = None;

    ui.horizontal(|ui| {
        let search_frame = egui::Frame::new()
            .fill(palette.input_background)
            .stroke(egui::Stroke::new(1.0, palette.border))
            .corner_radius(egui::CornerRadius::same(theme::radius::CONTROL))
            .inner_margin(egui::Margin::symmetric(
                theme::size::INPUT_PADDING_X,
                theme::size::INPUT_PADDING_Y,
            ));

        ui.add(
            egui::TextEdit::singleline(state.search)
                .frame(search_frame)
                .hint_text(translations.get("apps.search_hint"))
                .desired_width(theme::size::SEARCH_WIDTH),
        );

        ui.label(translations.get("apps.filter"));
        egui::ComboBox::from_id_salt("apps_filter")
            .selected_text(filter_label(translations, *state.filter))
            .show_ui(ui, |ui| {
                for filter in AppsFilter::OPTIONS {
                    ui.selectable_value(state.filter, filter, filter_label(translations, filter));
                }
            });

        let filtered_count = state
            .apps
            .iter()
            .filter(|app| state.filter.matches(app))
            .count();

        ui.label(translations.format("apps.count", &[("count", &filtered_count.to_string())]));

        if state.loading_labels {
            ui.spinner();
            ui.label(translations.get("apps.loading_names"));
        } else if ui
            .add_enabled(
                !state.apps.is_empty(),
                theme::toned_button(
                    translations.get("apps.load_names"),
                    theme::ButtonTone::Primary,
                    palette,
                ),
            )
            .on_hover_text(translations.get("apps.load_names_tooltip"))
            .clicked()
        {
            action = Some(AppsAction::LoadNames);
        }
    });

    ui.add_space(theme::size::SPACING_SM);

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

    let visible: Vec<&InstalledApp> = state
        .apps
        .iter()
        .filter(|app| state.filter.matches(app))
        .filter(matches_query)
        .collect();

    if visible.is_empty() {
        ui.label(translations.get("apps.no_matches"));
        return action;
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(
            ui,
            theme::size::ROW_HEIGHT_APPS,
            visible.len(),
            |ui, range| {
                for app in &visible[range] {
                    let is_busy = state.busy.contains(&app.package);

                    ui.allocate_ui_with_layout(
                        egui::vec2(ui.available_width(), theme::size::ROW_HEIGHT_APPS),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.add_space(theme::size::ROW_PADDING);

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
                                let load = theme::toned_button(
                                    translations.get("apps.load_name"),
                                    theme::ButtonTone::Neutral,
                                    palette,
                                )
                                .min_size(egui::vec2(
                                    theme::size::BUTTON_WIDTH_COMPACT,
                                    theme::size::CONTROL_HEIGHT,
                                ));

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
                                        .color(palette.warning_text),
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

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.add_space(theme::size::ROW_PADDING);

                                    let button_size = egui::vec2(
                                        theme::size::BUTTON_WIDTH,
                                        theme::size::CONTROL_HEIGHT,
                                    );

                                    let uninstall = theme::toned_button(
                                        translations.get("apps.uninstall"),
                                        theme::ButtonTone::Danger,
                                        palette,
                                    )
                                    .min_size(button_size);

                                    if ui.add_enabled(!is_busy, uninstall).clicked() {
                                        action = Some(AppsAction::Uninstall(app.package.clone()));
                                    }

                                    if app.is_disabled {
                                        let enable = theme::toned_button(
                                            translations.get("apps.enable"),
                                            theme::ButtonTone::Success,
                                            palette,
                                        )
                                        .min_size(button_size);
                                        if ui.add_enabled(!is_busy, enable).clicked() {
                                            action = Some(AppsAction::Enable(app.package.clone()));
                                        }
                                    } else {
                                        let disable = theme::toned_button(
                                            translations.get("apps.disable"),
                                            theme::ButtonTone::Warning,
                                            palette,
                                        )
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
                                },
                            );
                        },
                    );
                }
            },
        );

    action
}

fn filter_label(translations: &Translations, filter: AppsFilter) -> &str {
    match filter {
        AppsFilter::All => translations.get("apps.filter_all"),
        AppsFilter::System => translations.get("apps.filter_system"),
        AppsFilter::Installed => translations.get("apps.filter_installed"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(package: &str, is_system: bool) -> InstalledApp {
        InstalledApp {
            package: PackageName::parse(package).expect("valid package name"),
            label: None,
            apk_path: None,
            is_system,
            is_disabled: false,
        }
    }

    #[test]
    fn all_filter_keeps_every_application() {
        assert!(AppsFilter::All.matches(&app("com.example", false)));
        assert!(AppsFilter::All.matches(&app("com.android.settings", true)));
    }

    #[test]
    fn system_filter_keeps_only_system_applications() {
        assert!(AppsFilter::System.matches(&app("com.android.settings", true)));
        assert!(!AppsFilter::System.matches(&app("com.example", false)));
    }

    #[test]
    fn installed_filter_excludes_system_applications() {
        assert!(!AppsFilter::Installed.matches(&app("com.android.settings", true)));
        assert!(AppsFilter::Installed.matches(&app("com.example", false)));
    }
}
