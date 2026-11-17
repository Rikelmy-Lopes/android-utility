# Status

Current implementation status of Android Utility. Read this file before starting work and update it after completing a task.

## Done

- [x] Initial project documentation adapted from general architecture, development, and UI guidelines.
- [x] Project name (Android Utility) and high-level objective defined.
- [x] Rust language and `egui`/`eframe` GUI framework selected.
- [x] Windows and Linux build targets defined.
- [x] Architectural boundaries for core logic, application actions, ADB integration, and UI documented.
- [x] General development, testing, error handling, and UI principles documented.
- [x] Rust Cargo workspace created with the application package and GUI-independent `core` crate.
- [x] Minimal `eframe` application entry point added under `src/ui`.
- [x] ADB process runner with argument-based invocation, timeouts, and structured errors.
- [x] Connected-device discovery and device selection in the interface.
- [x] Installed-application listing with package name, system flag, and disabled state.
- [x] Application label resolution by reading the base APK manifest on a background thread.
- [x] Disable, enable, and uninstall actions with confirmation dialogs.
- [x] Uninstall, disable, and enable actions scoped to the current user only.
- [x] Warning that removing a system application may brick the device.
- [x] Application names loaded on demand; the default listing shows package names only.
- [x] Per-row action to resolve a single application name without loading all of them.
- [x] Resolved application names cached in `labels.json` and reused before reading an APK again.
- [x] English translation loading from `locales/en.json`.
- [x] Per-user cache directory under `%LOCALAPPDATA%` on Windows and `$XDG_DATA_HOME`/`$HOME/.local/share` on Linux.
- [x] Virtualized application list with fixed-height rows to keep window resizing responsive.
- [x] Application list filter to show all apps, system apps only, or user-installed apps only.
- [x] Interface split into tabs: application list and device tools.
- [x] Device tab showing processes ordered by CPU or memory, with configurable refresh interval.
- [x] Device tab refresh interval selectable from presets or a custom value in milliseconds, minimum 100 ms.
- [x] Device tab option to limit the process list to the first 5, 10, or 20 entries.
- [x] Diagnostic logging to a daily per-user log file, with terminal output in debug builds.
- [x] Logged ADB command lines, exit codes, durations, timeouts, failures, and application command events.
- [x] Centralized design tokens for palettes, semantic colors, font sizes, spacing, radii, and control sizes in `src/ui/theme.rs`.
- [x] Consistent panel spacing, row alignment, and rounded buttons and input fields.
- [x] Semantic action colors: red for uninstall and errors, amber for warnings, green for enable and success confirmations.
- [x] Dark and light themes with a toolbar theme switcher.
- [x] Selected theme persisted in `settings.json` below the per-user application data directory.
- [x] Device tab session recording that tracks per-process CPU and resident memory samples on every process refresh tick.
- [x] Start and stop recording controls that reset counters, show recorded update count, and stop collection immediately.
- [x] Resource usage summary dialog with elapsed time, update count, and per-process average CPU and memory for the first 10 processes in the selected sort order.
- [x] Samples follow the current leading processes; when the tracked set is full, the least recently sampled process is replaced.
- [x] Per-process averages safely handle processes with no samples and sessions stopped without any update.
- [x] Active recording cancelled when leaving the Device tab or switching devices.

## In Progress

- (none)

## Planned

- [ ] Application icon extraction and display.
- [ ] Configurable ADB executable path (currently resolved from `PATH`).
- [ ] Device information and analysis features beyond the application list.
- [ ] Verify Linux build and document platform prerequisites.
