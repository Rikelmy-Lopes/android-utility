use eframe::egui;

use android_utility_core::{ProcessSort, ProcessUsage};

use crate::i18n::Translations;

/// Height allocated for one process row, in points.
const ROW_HEIGHT: f32 = 26.0;

/// Refresh intervals offered to the user, in seconds.
pub const REFRESH_INTERVALS: [u64; 4] = [1, 2, 5, 10];

/// State needed to render the device process list.
pub struct ProcessesPanel<'a> {
    /// Processes to display.
    pub processes: &'a [ProcessUsage],
    /// Selected ordering.
    pub sort: &'a mut ProcessSort,
    /// Selected refresh interval in seconds.
    pub refresh_seconds: &'a mut u64,
    /// Whether a process snapshot is being read.
    pub loading: bool,
    /// Translation source.
    pub translations: &'a Translations,
}

/// Action requested by the user in the process list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessesAction {
    /// Read a new process snapshot now.
    Refresh,
}

/// Renders process resource usage and returns the requested action.
pub fn show(ui: &mut egui::Ui, state: ProcessesPanel<'_>) -> Option<ProcessesAction> {
    let translations = state.translations;
    let mut action = None;

    ui.horizontal(|ui| {
        ui.label(translations.get("processes.sort"));
        egui::ComboBox::from_id_salt("process_sort")
            .selected_text(sort_label(translations, *state.sort))
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    state.sort,
                    ProcessSort::Cpu,
                    translations.get("processes.sort_cpu"),
                );
                ui.selectable_value(
                    state.sort,
                    ProcessSort::Memory,
                    translations.get("processes.sort_memory"),
                );
            });

        ui.separator();

        ui.label(translations.get("processes.interval"));
        egui::ComboBox::from_id_salt("process_refresh_interval")
            .selected_text(seconds_label(translations, *state.refresh_seconds))
            .show_ui(ui, |ui| {
                for seconds in REFRESH_INTERVALS {
                    ui.selectable_value(
                        state.refresh_seconds,
                        seconds,
                        seconds_label(translations, seconds),
                    );
                }
            });

        if ui
            .add(egui::Button::new(translations.get("processes.refresh")))
            .clicked()
        {
            action = Some(ProcessesAction::Refresh);
        }

        if state.loading {
            ui.spinner();
        }
    });

    ui.add_space(4.0);

    if state.loading && state.processes.is_empty() {
        ui.horizontal(|ui| {
            ui.spinner();
            ui.label(translations.get("processes.loading"));
        });
        return action;
    }

    if state.processes.is_empty() {
        ui.label(translations.get("processes.empty"));
        return action;
    }

    draw_header(ui, translations);
    ui.separator();

    let available_width = ui.available_width();
    let fixed_width = 56.0 + 64.0 + 88.0 + 92.0;
    let spacing = ui.spacing().item_spacing.x * 4.0;
    let command_width = (available_width - fixed_width - spacing).max(160.0);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(ui, ROW_HEIGHT, state.processes.len(), |ui, range| {
            for process in &state.processes[range] {
                ui.horizontal(|ui| {
                    cell(ui, 56.0, process.pid.to_string(), Align::Right);
                    cell(
                        ui,
                        64.0,
                        format!("{:.1}", process.cpu_percent),
                        Align::Right,
                    );
                    cell(ui, 88.0, format_memory(process.memory_bytes), Align::Right);
                    cell(ui, 92.0, process.cpu_time.clone(), Align::Right);

                    ui.add_sized(
                        egui::vec2(command_width, ROW_HEIGHT),
                        egui::Label::new(&process.command).truncate(),
                    )
                    .on_hover_text(&process.command);
                });
            }
        });

    action
}

#[derive(Clone, Copy)]
enum Align {
    Left,
    Right,
}

fn draw_header(ui: &mut egui::Ui, translations: &Translations) {
    let available_width = ui.available_width();
    let fixed_width = 56.0 + 64.0 + 88.0 + 92.0;
    let spacing = ui.spacing().item_spacing.x * 4.0;
    let command_width = (available_width - fixed_width - spacing).max(160.0);

    ui.horizontal(|ui| {
        header_cell(ui, 56.0, translations.get("processes.pid"), Align::Right);
        header_cell(ui, 64.0, translations.get("processes.cpu"), Align::Right);
        header_cell(ui, 88.0, translations.get("processes.memory"), Align::Right);
        header_cell(ui, 92.0, translations.get("processes.time"), Align::Right);
        header_cell(
            ui,
            command_width,
            translations.get("processes.command"),
            Align::Left,
        );
    });
}

fn header_cell(ui: &mut egui::Ui, width: f32, text: &str, align: Align) {
    let text = egui::RichText::new(text).strong();
    cell(ui, width, text, align);
}

fn cell(ui: &mut egui::Ui, width: f32, text: impl Into<egui::WidgetText>, align: Align) {
    let layout = match align {
        Align::Left => egui::Layout::left_to_right(egui::Align::Center),
        Align::Right => egui::Layout::right_to_left(egui::Align::Center),
    };

    ui.allocate_ui_with_layout(egui::vec2(width, ROW_HEIGHT), layout, |ui| {
        ui.add(egui::Label::new(text).truncate());
    });
}

fn sort_label(translations: &Translations, sort: ProcessSort) -> &str {
    match sort {
        ProcessSort::Cpu => translations.get("processes.sort_cpu"),
        ProcessSort::Memory => translations.get("processes.sort_memory"),
    }
}

fn seconds_label(translations: &Translations, seconds: u64) -> String {
    translations.format("processes.seconds", &[("seconds", &seconds.to_string())])
}

fn format_memory(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = KIB * 1024.0;
    const GIB: f64 = MIB * 1024.0;

    let value = bytes as f64;

    if value >= GIB {
        format!("{:.1} GB", value / GIB)
    } else if value >= MIB {
        format!("{:.1} MB", value / MIB)
    } else if value >= KIB {
        format!("{:.0} KB", value / KIB)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_memory_with_binary_units() {
        assert_eq!(format_memory(0), "0 B");
        assert_eq!(format_memory(512), "512 B");
        assert_eq!(format_memory(2 * 1024), "2 KB");
        assert_eq!(format_memory(3 * 1024 * 1024), "3.0 MB");
        assert_eq!(format_memory(2 * 1024 * 1024 * 1024), "2.0 GB");
    }
}
