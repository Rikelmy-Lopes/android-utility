//! Doze whitelist view for the optimization tab.

use std::collections::{HashMap, HashSet};

use eframe::egui;

use android_utility_core::{BackgroundExecution, DozeWhitelistSource, InstalledApp, PackageName};

use crate::i18n::Translations;

use super::apps_panel::{AppsFilter, filter_label};
use super::theme;

/// Background execution state selected by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackgroundFilter {
    /// Show packages in every background state.
    All,
    /// Show packages allowed to run in the background.
    Allowed,
    /// Show packages restricted from running in the background.
    Denied,
}

impl BackgroundFilter {
    /// Filters offered to the user.
    pub const OPTIONS: [Self; 3] = [Self::All, Self::Allowed, Self::Denied];

    /// Returns whether `policy` matches the selected state.
    pub fn matches(self, policy: Option<BackgroundExecution>) -> bool {
        match self {
            Self::All => true,
            Self::Allowed => policy == Some(BackgroundExecution::Allowed),
            Self::Denied => policy == Some(BackgroundExecution::Ignored),
        }
    }
}

/// Doze whitelist state selected by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DozeFilter {
    /// Show packages in every Doze state.
    All,
    /// Show packages ignored by Doze.
    Ignored,
    /// Show packages not ignored by Doze.
    NotIgnored,
}

impl DozeFilter {
    /// Filters offered to the user.
    pub const OPTIONS: [Self; 3] = [Self::All, Self::Ignored, Self::NotIgnored];

    /// Returns whether the selected Doze state matches the package.
    ///
    /// Known states are matched only after the whitelist has loaded, so a
    /// pending load does not present every package as not ignored.
    pub fn matches(self, whitelist_loaded: bool, whitelisted: bool) -> bool {
        match self {
            Self::All => true,
            Self::Ignored => whitelist_loaded && whitelisted,
            Self::NotIgnored => whitelist_loaded && !whitelisted,
        }
    }
}

/// Read-only state needed to render the optimization list.
pub struct OptimizationPanel<'a> {
    /// Applications to display.
    pub apps: &'a [InstalledApp],
    /// Whitelisted packages and their origin.
    pub whitelist: &'a HashMap<PackageName, DozeWhitelistSource>,
    /// Known background execution policies.
    pub background: &'a HashMap<PackageName, BackgroundExecution>,
    /// Package subset selected by the user.
    pub filter: &'a mut AppsFilter,
    /// Background execution state selected by the user.
    pub background_filter: &'a mut BackgroundFilter,
    /// Doze whitelist state selected by the user.
    pub doze_filter: &'a mut DozeFilter,
    /// Search query edited by the user.
    pub search: &'a mut String,
    /// Packages with an operation in progress.
    pub busy: &'a HashSet<PackageName>,
    /// Whether applications are being loaded.
    pub loading_apps: bool,
    /// Whether the Doze whitelist is being loaded.
    pub loading_whitelist: bool,
    /// Whether background execution policies are being loaded.
    pub loading_background: bool,
    /// Whether the Doze whitelist was loaded successfully.
    pub whitelist_loaded: bool,
    /// Whether background execution policies were loaded successfully.
    pub background_loaded: bool,
    /// Translation source.
    pub translations: &'a Translations,
}

/// Action requested by the user in the optimization list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OptimizationAction {
    /// Read the Doze whitelist again.
    Refresh,
    /// Add or remove one package from the Doze whitelist.
    SetWhitelisted {
        /// Package to update.
        package: PackageName,
        /// Whether the package should be ignored by Doze.
        whitelisted: bool,
    },
    /// Change the background execution policy of one package.
    SetBackgroundExecution {
        /// Package to update.
        package: PackageName,
        /// Requested policy.
        policy: BackgroundExecution,
    },
}

/// Renders the optimization list and returns the requested action.
pub fn show(ui: &mut egui::Ui, state: OptimizationPanel<'_>) -> Option<OptimizationAction> {
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
        egui::ComboBox::from_id_salt("optimization_filter")
            .selected_text(filter_label(translations, *state.filter))
            .show_ui(ui, |ui| {
                for filter in AppsFilter::OPTIONS {
                    ui.selectable_value(state.filter, filter, filter_label(translations, filter));
                }
            });

        let refresh = theme::toned_button(
            translations.get("optimization.refresh"),
            theme::ButtonTone::Primary,
            palette,
        );

        if ui
            .add_enabled(
                !state.loading_apps && !state.loading_whitelist && !state.loading_background,
                refresh,
            )
            .on_hover_text(translations.get("optimization.refresh_tooltip"))
            .clicked()
        {
            action = Some(OptimizationAction::Refresh);
        }

        if state.loading_background {
            ui.spinner();
            ui.label(translations.get("optimization.background_loading"));
        }
    });

    ui.add_space(theme::size::SPACING_XS);

    ui.horizontal(|ui| {
        ui.label(translations.get("optimization.column_background"));
        egui::ComboBox::from_id_salt("optimization_background_filter")
            .selected_text(background_filter_label(
                translations,
                *state.background_filter,
            ))
            .show_ui(ui, |ui| {
                for filter in BackgroundFilter::OPTIONS {
                    ui.selectable_value(
                        state.background_filter,
                        filter,
                        background_filter_label(translations, filter),
                    );
                }
            });

        ui.label(translations.get("optimization.column_doze"));
        egui::ComboBox::from_id_salt("optimization_doze_filter")
            .selected_text(doze_filter_label(translations, *state.doze_filter))
            .show_ui(ui, |ui| {
                for filter in DozeFilter::OPTIONS {
                    ui.selectable_value(
                        state.doze_filter,
                        filter,
                        doze_filter_label(translations, filter),
                    );
                }
            });
    });

    ui.add_space(theme::size::SPACING_SM);

    if state.loading_apps && state.apps.is_empty() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(translations.get("apps.loading"));
        });
        return action;
    }

    if state.apps.is_empty() {
        ui.label(translations.get("apps.empty"));
        return action;
    }

    if !state.whitelist_loaded {
        ui.horizontal(|ui| {
            if state.loading_whitelist {
                ui.spinner();
                ui.label(translations.get("optimization.loading"));
            } else {
                ui.label(translations.get("optimization.unavailable"));
            }
        });
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
        .filter(|app| {
            let policy = if state.background_loaded {
                state.background.get(&app.package).copied()
            } else {
                None
            };

            state.background_filter.matches(policy)
        })
        .filter(|app| {
            state.doze_filter.matches(
                state.whitelist_loaded,
                state.whitelist.contains_key(&app.package),
            )
        })
        .filter(matches_query)
        .collect();

    if visible.is_empty() {
        ui.label(translations.get("apps.no_matches"));
        return action;
    }

    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), theme::size::CONTROL_HEIGHT),
        egui::Layout::right_to_left(egui::Align::Center),
        |ui| {
            ui.add_space(theme::size::ROW_PADDING);
            column_header(ui, translations.get("optimization.column_doze"));
            ui.add_space(theme::size::SPACING_XS);
            column_header(ui, translations.get("optimization.column_background"));
        },
    );

    ui.add_space(theme::size::SPACING_XS);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(
            ui,
            theme::size::ROW_HEIGHT_APPS,
            visible.len(),
            |ui, range| {
                for app in &visible[range] {
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

                            if app.is_system {
                                ui.label(
                                    egui::RichText::new(translations.get("apps.system"))
                                        .small()
                                        .color(palette.warning_text),
                                )
                                .on_hover_text(translations.get("apps.system_tooltip"));
                            }

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    ui.add_space(theme::size::ROW_PADDING);

                                    let is_whitelisted = state.whitelist.contains_key(&app.package);
                                    let (doze_label, doze_tone, doze_tooltip, doze_target) =
                                        if !state.whitelist_loaded {
                                            (
                                                "optimization.doze_unknown",
                                                theme::ButtonTone::Neutral,
                                                "optimization.doze_unknown_tooltip",
                                                None,
                                            )
                                        } else if is_whitelisted {
                                            (
                                                "optimization.doze_ignore",
                                                theme::ButtonTone::Success,
                                                "optimization.doze_not_ignore_tooltip",
                                                Some(false),
                                            )
                                        } else {
                                            (
                                                "optimization.doze_not_ignore",
                                                theme::ButtonTone::Neutral,
                                                "optimization.doze_ignore_tooltip",
                                                Some(true),
                                            )
                                        };

                                    let doze_button = theme::toned_button(
                                        translations.get(doze_label),
                                        doze_tone,
                                        palette,
                                    )
                                    .min_size(egui::vec2(
                                        theme::size::BUTTON_WIDTH,
                                        theme::size::CONTROL_HEIGHT,
                                    ));

                                    if ui
                                        .add_enabled(
                                            doze_target.is_some()
                                                && !state.busy.contains(&app.package),
                                            doze_button,
                                        )
                                        .on_hover_text(translations.get(doze_tooltip))
                                        .clicked()
                                        && let Some(whitelisted) = doze_target
                                    {
                                        action = Some(OptimizationAction::SetWhitelisted {
                                            package: app.package.clone(),
                                            whitelisted,
                                        });
                                    }

                                    ui.add_space(theme::size::SPACING_XS);

                                    let background = state.background.get(&app.package).copied();
                                    let (
                                        background_label,
                                        background_tone,
                                        background_tooltip,
                                        target,
                                    ) = match background {
                                        Some(BackgroundExecution::Allowed) => (
                                            "optimization.background_allowed",
                                            theme::ButtonTone::Success,
                                            "optimization.background_deny_tooltip",
                                            Some(BackgroundExecution::Ignored),
                                        ),
                                        Some(BackgroundExecution::Ignored) => (
                                            "optimization.background_denied",
                                            theme::ButtonTone::Warning,
                                            "optimization.background_allow_tooltip",
                                            Some(BackgroundExecution::Allowed),
                                        ),
                                        None => (
                                            "optimization.background_unknown",
                                            theme::ButtonTone::Neutral,
                                            "optimization.background_unknown_tooltip",
                                            None,
                                        ),
                                    };

                                    let background_button = theme::toned_button(
                                        translations.get(background_label),
                                        background_tone,
                                        palette,
                                    )
                                    .min_size(egui::vec2(
                                        theme::size::BUTTON_WIDTH,
                                        theme::size::CONTROL_HEIGHT,
                                    ));

                                    if ui
                                        .add_enabled(
                                            target.is_some() && !state.busy.contains(&app.package),
                                            background_button,
                                        )
                                        .on_hover_text(translations.get(background_tooltip))
                                        .clicked()
                                        && let Some(policy) = target
                                    {
                                        action = Some(OptimizationAction::SetBackgroundExecution {
                                            package: app.package.clone(),
                                            policy,
                                        });
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

fn background_filter_label(translations: &Translations, filter: BackgroundFilter) -> &str {
    match filter {
        BackgroundFilter::All => translations.get("optimization.filter_all"),
        BackgroundFilter::Allowed => translations.get("optimization.background_allowed"),
        BackgroundFilter::Denied => translations.get("optimization.background_denied"),
    }
}

fn doze_filter_label(translations: &Translations, filter: DozeFilter) -> &str {
    match filter {
        DozeFilter::All => translations.get("optimization.filter_all"),
        DozeFilter::Ignored => translations.get("optimization.doze_ignore"),
        DozeFilter::NotIgnored => translations.get("optimization.doze_not_ignore"),
    }
}

fn column_header(ui: &mut egui::Ui, text: &str) {
    ui.add_sized(
        egui::vec2(theme::size::BUTTON_WIDTH, theme::size::CONTROL_HEIGHT),
        egui::Label::new(egui::RichText::new(text).strong())
            .halign(egui::Align::Center)
            .selectable(false),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn background_filter_matches_the_selected_state() {
        assert!(BackgroundFilter::All.matches(None));
        assert!(BackgroundFilter::All.matches(Some(BackgroundExecution::Allowed)));
        assert!(BackgroundFilter::All.matches(Some(BackgroundExecution::Ignored)));

        assert!(BackgroundFilter::Allowed.matches(Some(BackgroundExecution::Allowed)));
        assert!(!BackgroundFilter::Allowed.matches(Some(BackgroundExecution::Ignored)));
        assert!(!BackgroundFilter::Allowed.matches(None));

        assert!(BackgroundFilter::Denied.matches(Some(BackgroundExecution::Ignored)));
        assert!(!BackgroundFilter::Denied.matches(Some(BackgroundExecution::Allowed)));
        assert!(!BackgroundFilter::Denied.matches(None));
    }

    #[test]
    fn doze_filter_matches_only_known_states() {
        assert!(DozeFilter::All.matches(false, false));
        assert!(DozeFilter::Ignored.matches(true, true));
        assert!(!DozeFilter::Ignored.matches(true, false));
        assert!(!DozeFilter::Ignored.matches(false, false));
        assert!(DozeFilter::NotIgnored.matches(true, false));
        assert!(!DozeFilter::NotIgnored.matches(true, true));
        assert!(!DozeFilter::NotIgnored.matches(false, false));
    }

    #[test]
    fn labels_are_defined_for_every_filter() {
        let translations = Translations::english();

        assert_eq!(
            background_filter_label(&translations, BackgroundFilter::All),
            "All"
        );
        assert_eq!(
            background_filter_label(&translations, BackgroundFilter::Allowed),
            "Allowed"
        );
        assert_eq!(
            background_filter_label(&translations, BackgroundFilter::Denied),
            "Denied"
        );

        assert_eq!(doze_filter_label(&translations, DozeFilter::All), "All");
        assert_eq!(
            doze_filter_label(&translations, DozeFilter::Ignored),
            "Ignore"
        );
        assert_eq!(
            doze_filter_label(&translations, DozeFilter::NotIgnored),
            "Not ignore"
        );
    }
}
