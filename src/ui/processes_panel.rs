use eframe::egui;

use android_utility_core::{ProcessSort, ProcessUsage};

use crate::i18n::Translations;

use super::theme;

/// Smallest refresh interval offered to the user, in milliseconds.
pub const MIN_REFRESH_MS: u64 = 100;

/// Largest refresh interval offered to the user, in milliseconds.
pub const MAX_REFRESH_MS: u64 = 60_000;

/// Refresh interval selected by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessRefresh {
    /// Use one of the preset intervals.
    Preset(u64),
    /// Use the interval entered by the user.
    Custom,
}

impl ProcessRefresh {
    /// Choices offered to the user.
    pub const OPTIONS: [Self; 5] = [
        Self::Preset(1000),
        Self::Preset(2000),
        Self::Preset(5000),
        Self::Preset(10_000),
        Self::Custom,
    ];

    /// Effective refresh interval in milliseconds.
    pub fn interval_ms(self, custom_ms: u64) -> u64 {
        match self {
            Self::Preset(ms) => ms,
            Self::Custom => custom_ms.clamp(MIN_REFRESH_MS, MAX_REFRESH_MS),
        }
    }
}

/// Number of processes shown in the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessLimit {
    /// Show every process.
    All,
    /// Show the first five processes.
    Five,
    /// Show the first ten processes.
    Ten,
    /// Show the first twenty processes.
    Twenty,
}

impl ProcessLimit {
    /// Limits offered to the user.
    pub const OPTIONS: [Self; 4] = [Self::All, Self::Five, Self::Ten, Self::Twenty];

    /// Maximum number of processes to display, or `None` for no limit.
    pub fn count(self) -> Option<usize> {
        match self {
            Self::All => None,
            Self::Five => Some(5),
            Self::Ten => Some(10),
            Self::Twenty => Some(20),
        }
    }
}

/// State needed to render the device process list.
pub struct ProcessesPanel<'a> {
    /// Processes to display.
    pub processes: &'a [ProcessUsage],
    /// Selected ordering.
    pub sort: &'a mut ProcessSort,
    /// Selected refresh interval.
    pub refresh: &'a mut ProcessRefresh,
    /// User-entered refresh interval in milliseconds.
    pub custom_refresh_ms: &'a mut u64,
    /// Selected number of processes to display.
    pub limit: &'a mut ProcessLimit,
    /// Whether a process snapshot is being read.
    pub loading: bool,
    /// Whether a resource-usage session is being recorded.
    pub recording: bool,
    /// Number of samples recorded in the active session.
    pub recorded_updates: u64,
    /// Translation source.
    pub translations: &'a Translations,
}

/// Action requested by the user in the process list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessesAction {
    /// Read a new process snapshot now.
    Refresh,
    /// Start recording resource usage.
    StartRecording,
    /// Stop recording and show the summary.
    StopRecording,
}

/// Renders process resource usage and returns the requested action.
pub fn show(ui: &mut egui::Ui, state: ProcessesPanel<'_>) -> Option<ProcessesAction> {
    let translations = state.translations;
    let palette = theme::palette(ui);
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
            .selected_text(refresh_label(translations, *state.refresh))
            .show_ui(ui, |ui| {
                for option in ProcessRefresh::OPTIONS {
                    ui.selectable_value(state.refresh, option, refresh_label(translations, option));
                }
            });

        if *state.refresh == ProcessRefresh::Custom {
            ui.add(
                egui::DragValue::new(state.custom_refresh_ms)
                    .range(MIN_REFRESH_MS..=MAX_REFRESH_MS)
                    .speed(50.0)
                    .suffix(translations.get("processes.millisecond_suffix")),
            );
            *state.custom_refresh_ms =
                (*state.custom_refresh_ms).clamp(MIN_REFRESH_MS, MAX_REFRESH_MS);
        }

        ui.separator();

        ui.label(translations.get("processes.show"));
        egui::ComboBox::from_id_salt("process_limit")
            .selected_text(limit_label(translations, *state.limit))
            .show_ui(ui, |ui| {
                for limit in ProcessLimit::OPTIONS {
                    ui.selectable_value(state.limit, limit, limit_label(translations, limit));
                }
            });

        ui.separator();

        if ui
            .add(theme::toned_button(
                translations.get("processes.refresh"),
                theme::ButtonTone::Primary,
                palette,
            ))
            .clicked()
        {
            action = Some(ProcessesAction::Refresh);
        }

        if state.loading {
            ui.spinner();
        }

        ui.separator();

        let recording_tone = if state.recording {
            theme::ButtonTone::Warning
        } else {
            theme::ButtonTone::Primary
        };
        let recording_label = if state.recording {
            translations.get("processes.stop_recording")
        } else {
            translations.get("processes.start_recording")
        };

        let button = theme::toned_button(recording_label, recording_tone, palette);

        if ui
            .add(button)
            .on_hover_text(translations.get("processes.recording_tooltip"))
            .clicked()
        {
            action = Some(if state.recording {
                ProcessesAction::StopRecording
            } else {
                ProcessesAction::StartRecording
            });
        }

        if state.recording {
            let count = state.recorded_updates.to_string();
            ui.colored_label(
                palette.warning_text,
                translations.format("processes.recording", &[("count", &count)]),
            );
        }
    });

    ui.add_space(theme::size::SPACING_SM);

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

    let visible = visible_processes(state.processes, *state.limit);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show_rows(
            ui,
            theme::size::ROW_HEIGHT_PROCESSES,
            visible.len(),
            |ui, range| {
                for process in &visible[range] {
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
                            egui::vec2(command_width, theme::size::ROW_HEIGHT_PROCESSES),
                            egui::Label::new(&process.command).truncate(),
                        )
                        .on_hover_text(&process.command);
                    });
                }
            },
        );

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

    ui.allocate_ui_with_layout(
        egui::vec2(width, theme::size::ROW_HEIGHT_PROCESSES),
        layout,
        |ui| {
            ui.add(egui::Label::new(text).truncate());
        },
    );
}

fn sort_label(translations: &Translations, sort: ProcessSort) -> &str {
    match sort {
        ProcessSort::Cpu => translations.get("processes.sort_cpu"),
        ProcessSort::Memory => translations.get("processes.sort_memory"),
    }
}

fn refresh_label(translations: &Translations, refresh: ProcessRefresh) -> String {
    match refresh {
        ProcessRefresh::Preset(ms) => translations.format(
            "processes.seconds",
            &[("seconds", &(ms / 1000).to_string())],
        ),
        ProcessRefresh::Custom => translations.get("processes.custom").to_owned(),
    }
}

fn limit_label(translations: &Translations, limit: ProcessLimit) -> String {
    match limit.count() {
        Some(count) => count.to_string(),
        None => translations.get("processes.show_all").to_owned(),
    }
}

/// Returns the leading slice of processes allowed by the selected limit.
fn visible_processes(processes: &[ProcessUsage], limit: ProcessLimit) -> &[ProcessUsage] {
    match limit.count() {
        Some(limit) => &processes[..processes.len().min(limit)],
        None => processes,
    }
}

pub(super) fn format_memory(bytes: u64) -> String {
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
    fn preset_refresh_intervals_ignore_the_custom_value() {
        assert_eq!(ProcessRefresh::Preset(1000).interval_ms(500), 1000);
        assert_eq!(ProcessRefresh::Preset(10_000).interval_ms(500), 10_000);
    }

    #[test]
    fn custom_refresh_interval_is_clamped_to_the_allowed_range() {
        assert_eq!(ProcessRefresh::Custom.interval_ms(500), 500);
        assert_eq!(ProcessRefresh::Custom.interval_ms(1), MIN_REFRESH_MS);
        assert_eq!(
            ProcessRefresh::Custom.interval_ms(MAX_REFRESH_MS * 2),
            MAX_REFRESH_MS
        );
    }

    #[test]
    fn process_limit_reports_a_maximum_row_count() {
        assert_eq!(ProcessLimit::All.count(), None);
        assert_eq!(ProcessLimit::Five.count(), Some(5));
        assert_eq!(ProcessLimit::Ten.count(), Some(10));
        assert_eq!(ProcessLimit::Twenty.count(), Some(20));
    }

    fn sample_processes(count: u32) -> Vec<ProcessUsage> {
        (0..count)
            .map(|pid| ProcessUsage {
                pid,
                cpu_percent: 0.0,
                memory_bytes: 0,
                cpu_time: String::new(),
                command: format!("proc{pid}"),
            })
            .collect()
    }

    #[test]
    fn visible_processes_keeps_every_entry_without_a_limit() {
        let processes = sample_processes(30);

        assert_eq!(visible_processes(&processes, ProcessLimit::All).len(), 30);
    }

    #[test]
    fn visible_processes_keeps_only_the_leading_entries_with_a_limit() {
        let processes = sample_processes(30);

        let visible = visible_processes(&processes, ProcessLimit::Ten);
        assert_eq!(visible.len(), 10);
        assert_eq!(visible[0].pid, 0);
        assert_eq!(visible[9].pid, 9);
    }

    #[test]
    fn visible_processes_handles_a_list_shorter_than_the_limit() {
        let processes = sample_processes(3);

        assert_eq!(visible_processes(&processes, ProcessLimit::Five).len(), 3);
    }

    #[test]
    fn formats_memory_with_binary_units() {
        assert_eq!(format_memory(0), "0 B");
        assert_eq!(format_memory(512), "512 B");
        assert_eq!(format_memory(2 * 1024), "2 KB");
        assert_eq!(format_memory(3 * 1024 * 1024), "3.0 MB");
        assert_eq!(format_memory(2 * 1024 * 1024 * 1024), "2.0 GB");
    }
}
