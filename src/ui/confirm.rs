use eframe::egui;

use android_utility_core::PackageName;

use crate::i18n::Translations;

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
        ui.set_width(380.0);
        ui.heading(translations.get(title_key(pending.kind)));
        ui.add_space(8.0);

        let message = translations.format(
            message_key(pending.kind),
            &[
                ("name", &pending.display_name),
                ("package", pending.package.as_str()),
            ],
        );
        ui.label(message);

        if pending.is_system {
            ui.add_space(8.0);
            ui.colored_label(
                ui.visuals().warn_fg_color,
                translations.get("confirm.system_warning"),
            );
        }

        ui.add_space(12.0);
        ui.horizontal(|ui| {
            if ui.button(translations.get("confirm.cancel")).clicked() {
                response = Some(ConfirmResponse::Cancelled);
            }

            let confirm = egui::Button::new(
                egui::RichText::new(translations.get("confirm.confirm"))
                    .color(egui::Color32::WHITE),
            )
            .fill(confirm_color(pending.kind));

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

fn confirm_color(kind: ConfirmKind) -> egui::Color32 {
    match kind {
        ConfirmKind::Disable => egui::Color32::from_rgb(150, 105, 20),
        ConfirmKind::Uninstall => egui::Color32::from_rgb(170, 45, 40),
    }
}
