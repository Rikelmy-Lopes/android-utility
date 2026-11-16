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
- [x] Uninstall fallback to current-user removal when full uninstall fails.
- [x] Warning that removing a system application may brick the device.
- [x] Application names loaded on demand; the default listing shows package names only.
- [x] Per-row action to resolve a single application name without loading all of them.
- [x] Resolved application names cached in `labels.json` and reused before reading an APK again.
- [x] English translation loading from `locales/en.json`.
- [x] Per-user cache directory under `%LOCALAPPDATA%` on Windows and `$XDG_DATA_HOME`/`$HOME/.local/share` on Linux.
- [x] Virtualized application list with fixed-height rows to keep window resizing responsive.

## In Progress

- (none)

## Planned

- [ ] Application icon extraction and display.
- [ ] Configurable ADB executable path (currently resolved from `PATH`).
- [ ] Device information and analysis features beyond the application list.
- [ ] Application configuration, logging, and theme persistence.
- [ ] Verify Linux build and document platform prerequisites.
