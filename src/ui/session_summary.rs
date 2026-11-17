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
            let grid_spacing = theme::size::SPACING_MD;
            let command_min_width = 140.0;

            let screen_width =
                (context.content_rect().width() - 4.0 * theme::size::SPACING_MD).max(320.0);
            let (dialog_width, cpu_width, memory_width) = if summary.processes.is_empty() {
                (screen_width.min(620.0), 0.0, 0.0)
            } else {
                let cpu_width = column_width(
                    ui,
                    translations.get("session.cpu"),
                    summary
                        .processes
                        .iter()
                        .map(|process| format!("{:.1} %", process.average_cpu_percent)),
                );
                let memory_width = column_width(
                    ui,
                    translations.get("session.memory"),
                    summary
                        .processes
                        .iter()
                        .map(|process| format_memory(process.average_memory_bytes)),
                );
                let table_width = command_min_width + cpu_width + memory_width + 2.0 * grid_spacing;

                (
                    table_width.max(620.0).min(screen_width),
                    cpu_width,
                    memory_width,
                )
            };

            ui.set_width(dialog_width);
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
                let command_width =
                    (ui.available_width() - cpu_width - memory_width - 2.0 * grid_spacing).max(0.0);

                egui::Grid::new("resource_session_process_grid")
                    .num_columns(3)
                    .striped(true)
                    .spacing([grid_spacing, theme::size::SPACING_XS])
                    .show(ui, |ui| {
                        header(
                            ui,
                            translations.get("session.process"),
                            Align::Left,
                            command_width,
                        );
                        header(ui, translations.get("session.cpu"), Align::Right, cpu_width);
                        header(
                            ui,
                            translations.get("session.memory"),
                            Align::Right,
                            memory_width,
                        );
                        ui.end_row();

                        for process in &summary.processes {
                            command_cell(ui, process, command_width);
                            value_cell(
                                ui,
                                format!("{:.1} %", process.average_cpu_percent),
                                cpu_width,
                            );
                            value_cell(
                                ui,
                                format_memory(process.average_memory_bytes),
                                memory_width,
                            );
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

fn header(ui: &mut egui::Ui, text: &str, align: Align, width: f32) {
    let text = egui::RichText::new(text).strong();
    aligned_cell(ui, text, align, width);
}

fn command_cell(ui: &mut egui::Ui, process: &ProcessAverage, width: f32) {
    let label = egui::Label::new(format!("{} ({})", process.command, process.pid)).truncate();

    ui.add_sized(egui::vec2(width, theme::size::ROW_HEIGHT_PROCESSES), label)
        .on_hover_text(&process.command);
}

/// Returns the width needed to show `header` and every value on a single line.
fn column_width(ui: &egui::Ui, header: &str, values: impl Iterator<Item = String>) -> f32 {
    let font_id = egui::TextStyle::Body.resolve(ui.style());
    let measure = |text: &str| {
        ui.painter()
            .layout_no_wrap(text.to_owned(), font_id.clone(), egui::Color32::PLACEHOLDER)
            .size()
            .x
    };

    let mut width = measure(header);
    for value in values {
        width = width.max(measure(&value));
    }

    width + theme::size::SPACING_SM
}

fn value_cell(ui: &mut egui::Ui, text: String, width: f32) {
    aligned_cell(ui, text, Align::Right, width);
}

fn aligned_cell(ui: &mut egui::Ui, text: impl Into<egui::WidgetText>, align: Align, width: f32) {
    let halign = match align {
        Align::Left => egui::Align::Min,
        Align::Right => egui::Align::Max,
    };

    let label = egui::Label::new(text).truncate().halign(halign);
    ui.add_sized(egui::vec2(width, theme::size::ROW_HEIGHT_PROCESSES), label);
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

    #[test]
    fn column_width_covers_the_header_and_every_value() {
        let context = egui::Context::default();
        let measured = std::cell::Cell::new((0.0f32, 0.0f32, 0.0f32));

        let mut output = context.run_ui(Default::default(), |ui| {
            let font_id = egui::TextStyle::Body.resolve(ui.style());
            let galley_width = |text: &str| {
                ui.painter()
                    .layout_no_wrap(text.to_owned(), font_id.clone(), egui::Color32::PLACEHOLDER)
                    .size()
                    .x
            };

            let longest = "100.0 %";
            let column = column_width(
                ui,
                "Average CPU",
                [longest.to_owned(), "0.0 %".to_owned()].into_iter(),
            );
            measured.set((column, galley_width("Average CPU"), galley_width(longest)));
        });
        output.textures_delta.clear();

        let (column, header, value) = measured.get();
        assert!(column >= header);
        assert!(column >= value);
        assert!(column >= theme::size::SPACING_SM);
    }
}
