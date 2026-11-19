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
- [x] Interface split into tabs: application list, optimization, and device tools.
- [x] Process tab showing processes ordered by CPU or memory, with configurable refresh interval.
- [x] Process tab refresh interval selectable from presets or a custom value in milliseconds, minimum 100 ms.
- [x] Process tab option to limit the process list to the first 5, 10, or 20 entries.
- [x] Diagnostic logging to a daily per-user log file, with terminal output in debug builds.
- [x] Logged ADB command lines, exit codes, durations, timeouts, failures, and application command events.
- [x] Centralized design tokens for palettes, semantic colors, font sizes, spacing, radii, and control sizes in `src/ui/theme.rs`.
- [x] Consistent panel spacing, row alignment, and rounded buttons and input fields.
- [x] Semantic action colors: red for uninstall and errors, amber for warnings, green for enable and success confirmations.
- [x] Dark theme primary buttons use white text on a darker accent fill with at least 4.5:1 contrast.
- [x] Dark and light palettes, typography, compact control heights, and corner radii aligned with Android Studio styling.
- [x] Dark and light themes with a toolbar theme switcher.
- [x] Selected theme persisted in `settings.json` below the per-user application data directory.
- [x] Process tab session recording that tracks per-process CPU and resident memory samples on every process refresh tick.
- [x] Start and stop recording controls that reset counters, show recorded update count, and stop collection immediately.
- [x] Resource usage summary dialog with elapsed time, update count, and per-process average CPU and memory for the first 10 processes in the selected sort order.
- [x] Samples follow the current leading processes; when the tracked set is full, the least recently sampled process is replaced.
- [x] Per-process averages safely handle processes with no samples and sessions stopped without any update.
- [x] Active recording cancelled when leaving the Process tab or switching devices.
- [x] Process tab label renamed from "Device" and a fixed spinner slot keeps the toolbar controls from shifting while loading.
- [x] Resource usage summary dialog widths the CPU and memory columns from the measured header and value text, grows up to the window width, and never clips those columns.
- [x] Optimization tab listing installed applications with the same all, system, and installed filters.
- [x] Doze whitelist read from `dumpsys deviceidle whitelist` with system and user entries tracked internally.
- [x] Per-application Doze toggle adding or removing a package from the whitelist through `dumpsys deviceidle whitelist +package` and `-package`.
- [x] Optimization tab reuses the cached application names without its own name-loading controls.
- [x] Optimization tab shows Background and Doze columns; Background values are Allowed and Denied, Doze values are Ignore and Not ignore.
- [x] Background execution states are read in one `cmd appops query-op RUN_ANY_IN_BACKGROUND ignore` call instead of one call per application.
- [x] Per-application background execution toggle switching between allow and ignore with `cmd appops set`.
- [x] Background execution policies load with the Doze whitelist when the Optimization tab opens and when it is refreshed.
- [x] Applications tab asks for confirmation before loading every application name, warning that the operation is slow.
- [x] UI application state split into actions and event handling modules, with device resets shared by one helper.
- [x] ADB integration split into client, device, and focused parser modules.
- [x] ADB command reference documents every command, possible outputs, and minimum Android version.
- [x] Development rules require documenting every new ADB command in the command reference.
- [x] Unsupported ADB commands and device services are reported as structured errors instead of empty results.
- [x] Disable, enable, and uninstall update the visible application row immediately instead of waiting for the full listing to reload.

## In Progress

- (none)

## Planned

- [ ] Application icon extraction and display.
- [ ] Configurable ADB executable path (currently resolved from `PATH`).
- [ ] Device information and analysis features beyond the application list.
- [ ] Verify Linux build and document platform prerequisites.
