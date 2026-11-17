use std::time::Duration;

use eframe::egui;

use android_utility_core::{ProcessAverage, SessionSummary};

use crate::i18n::Translations;

use super::processes_panel::format_memory;
use super::theme;

/// Renders the completed resource-session summary.
///
/// Returns `true` when the user dismissed the dialog.
pub fn show(
    context: &egui::Context,
    translations: &Translations,
    summary: &SessionSummary,
) -> bool {
    let mut dismissed = false;

    let modal =
        egui::Modal::new(egui::Id::unique("resource_session_summary")).show(context, |ui| {
            let palette = theme::palette(ui);

            ui.set_width(620.0);
            ui.heading(translations.get("session.title"));
            ui.add_space(theme::size::SPACING_MD);

            egui::Grid::new("resource_session_overview_grid")
                .num_columns(2)
                .spacing([theme::size::SPACING_MD, theme::size::SPACING_SM])
                .show(ui, |ui| {
                    row(
                        ui,
                        translations.get("session.elapsed"),
                        format_elapsed(summary.elapsed),
                    );
                    row(
                        ui,
                        translations.get("session.updates"),
                        summary.updates.to_string(),
                    );
                });

            ui.add_space(theme::size::SPACING_MD);
            ui.separator();
            ui.add_space(theme::size::SPACING_SM);

            if summary.processes.is_empty() {
                ui.label(translations.get("session.empty"));
            } else {
                egui::Grid::new("resource_session_process_grid")
                    .num_columns(3)
                    .striped(true)
                    .spacing([theme::size::SPACING_MD, theme::size::SPACING_XS])
                    .show(ui, |ui| {
                        header(ui, translations.get("session.process"), Align::Left);
                        header(ui, translations.get("session.cpu"), Align::Right);
                        header(ui, translations.get("session.memory"), Align::Right);
                        ui.end_row();

                        for process in &summary.processes {
                            command_cell(ui, process);
                            value_cell(ui, format!("{:.1} %", process.average_cpu_percent));
                            value_cell(ui, format_memory(process.average_memory_bytes));
                            ui.end_row();
                        }
                    });
            }

            ui.add_space(theme::size::SPACING_MD);
            ui.horizontal(|ui| {
                let button = theme::toned_button(
                    translations.get("session.close"),
                    theme::ButtonTone::Primary,
                    palette,
                )
                .min_size(egui::vec2(112.0, theme::size::CONTROL_HEIGHT + 2.0));

                if ui.add(button).clicked() {
                    dismissed = true;
                }
            });
        });

    dismissed || modal.should_close()
}

fn row(ui: &mut egui::Ui, label: &str, value: String) {
    ui.label(label);
    ui.label(egui::RichText::new(value).strong());
    ui.end_row();
}

#[derive(Clone, Copy)]
enum Align {
    Left,
    Right,
}

fn header(ui: &mut egui::Ui, text: &str, align: Align) {
    let text = egui::RichText::new(text).strong();
    aligned_cell(ui, text, align);
}

fn command_cell(ui: &mut egui::Ui, process: &ProcessAverage) {
    let label = egui::Label::new(format!("{} ({})", process.command, process.pid)).truncate();

    ui.add_sized(egui::vec2(360.0, theme::size::ROW_HEIGHT_PROCESSES), label)
        .on_hover_text(&process.command);
}

fn value_cell(ui: &mut egui::Ui, text: String) {
    aligned_cell(ui, text, Align::Right);
}

fn aligned_cell(ui: &mut egui::Ui, text: impl Into<egui::WidgetText>, align: Align) {
    let layout = match align {
        Align::Left => egui::Layout::left_to_right(egui::Align::Center),
        Align::Right => egui::Layout::right_to_left(egui::Align::Center),
    };

    ui.allocate_ui_with_layout(
        egui::vec2(96.0, theme::size::ROW_HEIGHT_PROCESSES),
        layout,
        |ui| {
            ui.add(egui::Label::new(text).truncate());
        },
    );
}

fn format_elapsed(elapsed: Duration) -> String {
    let total = elapsed.as_secs();
    let hours = total / 3600;
    let minutes = (total % 3600) / 60;
    let seconds = total % 60;

    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_elapsed_time_as_hours_minutes_seconds() {
        assert_eq!(format_elapsed(Duration::from_secs(0)), "00:00:00");
        assert_eq!(format_elapsed(Duration::from_secs(59)), "00:00:59");
        assert_eq!(format_elapsed(Duration::from_secs(61)), "00:01:01");
        assert_eq!(format_elapsed(Duration::from_secs(3661)), "01:01:01");
        assert_eq!(format_elapsed(Duration::from_secs(90_000)), "25:00:00");
    }

    #[test]
    fn truncates_sub_second_durations() {
        assert_eq!(format_elapsed(Duration::from_millis(1900)), "00:00:01");
    }
}
