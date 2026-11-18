use eframe::egui;

use android_utility_core::PackageName;

use crate::i18n::Translations;

use super::theme;

/// Destructive operation that needs explicit confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmKind {
    /// Disable the package for the current user.
    Disable,
    /// Uninstall the package.
    Uninstall,
}

/// Confirmation requested by the user interface.
#[derive(Debug, Clone)]
pub struct PendingConfirmation {
    /// Operation awaiting confirmation.
    pub kind: ConfirmKind,
    /// Target package.
    pub package: PackageName,
    /// Label or package name shown to the user.
    pub display_name: String,
    /// Whether the package belongs to the system image.
    pub is_system: bool,
}

/// User decision about a pending confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmResponse {
    /// The operation should run.
    Confirmed,
    /// The operation should be dropped.
    Cancelled,
}

/// Renders the confirmation modal and returns the user decision.
pub fn show(
    context: &egui::Context,
    translations: &Translations,
    pending: &PendingConfirmation,
) -> Option<ConfirmResponse> {
    let mut response = None;

    let modal = egui::Modal::new(egui::Id::unique("confirmation")).show(context, |ui| {
        let palette = theme::palette(ui);

        ui.set_width(380.0);
        ui.heading(translations.get(title_key(pending.kind)));
        ui.add_space(theme::size::SPACING_SM);

        let message = translations.format(
            message_key(pending.kind),
            &[
                ("name", &pending.display_name),
                ("package", pending.package.as_str()),
            ],
        );
        ui.label(message);

        if pending.is_system {
            ui.add_space(theme::size::SPACING_SM);
            ui.colored_label(
                palette.warning_text,
                translations.get("confirm.system_warning"),
            );
        }

        ui.add_space(theme::size::SPACING_MD);
        ui.horizontal(|ui| {
            let button_size = egui::vec2(112.0, theme::size::CONTROL_HEIGHT + 2.0);

            if ui
                .add(
                    theme::toned_button(
                        translations.get("confirm.cancel"),
                        theme::ButtonTone::Neutral,
                        palette,
                    )
                    .min_size(button_size),
                )
                .clicked()
            {
                response = Some(ConfirmResponse::Cancelled);
            }

            let confirm = theme::toned_button(
                translations.get("confirm.confirm"),
                confirm_tone(pending.kind),
                palette,
            )
            .min_size(button_size);

            if ui.add(confirm).clicked() {
                response = Some(ConfirmResponse::Confirmed);
            }
        });
    });

    if response.is_none() && modal.should_close() {
        response = Some(ConfirmResponse::Cancelled);
    }

    response
}

/// Renders the confirmation for the slow application-name load.
///
/// Reading every APK can take a long time, so the user must opt in.
pub fn show_load_names(
    context: &egui::Context,
    translations: &Translations,
) -> Option<ConfirmResponse> {
    let mut response = None;

    let modal = egui::Modal::new(egui::Id::unique("load_names_confirmation")).show(context, |ui| {
        let palette = theme::palette(ui);

        ui.set_width(420.0);
        ui.heading(translations.get("confirm.load_names_title"));
        ui.add_space(theme::size::SPACING_SM);
        ui.colored_label(
            palette.warning_text,
            translations.get("confirm.load_names_message"),
        );

        ui.add_space(theme::size::SPACING_MD);
        ui.horizontal(|ui| {
            let button_size = egui::vec2(112.0, theme::size::CONTROL_HEIGHT + 2.0);

            if ui
                .add(
                    theme::toned_button(
                        translations.get("confirm.cancel"),
                        theme::ButtonTone::Neutral,
                        palette,
                    )
                    .min_size(button_size),
                )
                .clicked()
            {
                response = Some(ConfirmResponse::Cancelled);
            }

            let confirm = theme::toned_button(
                translations.get("apps.load_names"),
                theme::ButtonTone::Primary,
                palette,
            )
            .min_size(button_size);

            if ui.add(confirm).clicked() {
                response = Some(ConfirmResponse::Confirmed);
            }
        });
    });

    if response.is_none() && modal.should_close() {
        response = Some(ConfirmResponse::Cancelled);
    }

    response
}

fn title_key(kind: ConfirmKind) -> &'static str {
    match kind {
        ConfirmKind::Disable => "confirm.disable_title",
        ConfirmKind::Uninstall => "confirm.uninstall_title",
    }
}

fn message_key(kind: ConfirmKind) -> &'static str {
    match kind {
        ConfirmKind::Disable => "confirm.disable_message",
        ConfirmKind::Uninstall => "confirm.uninstall_message",
    }
}

fn confirm_tone(kind: ConfirmKind) -> theme::ButtonTone {
    match kind {
        ConfirmKind::Disable => theme::ButtonTone::Warning,
        ConfirmKind::Uninstall => theme::ButtonTone::Danger,
    }
}
