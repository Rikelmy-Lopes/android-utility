# Android Utility

**Android Utility** is a Rust desktop application for inspecting and analyzing Android devices through Android Debug Bridge (ADB). Its graphical interface is built with `egui` via `eframe`, and the application must compile for Windows and Linux.

## Overview

| Item | Value |
|---|---|
| Product name | Android Utility |
| Language | Rust |
| GUI framework | `egui` (via `eframe`) |
| Target platforms | Windows and Linux |
| Purpose | Android device inspection and analysis through ADB |
| External integration | Android Debug Bridge (ADB) |
| Localization | Internationalization-ready; supported locales are defined by the project |

## Architectural Principles

- Follow Clean Architecture and keep business logic independent of the user interface.
- Keep the core crate free of `egui` and `eframe`.
- Isolate ADB process execution, output parsing, and device communication in a dedicated integration layer.
- Keep UI components focused on presentation and interaction.
- Execute long-running operations outside the main UI thread.
- Use explicit error handling and automated tests.
- Apply the Single Responsibility Principle (SRP).

## Project Structure

The application uses a Cargo workspace with a GUI-independent core crate and an application package.

```text
android-utility/
├── Cargo.toml                  # Workspace root and application package
├── core/       # GUI-independent business logic
│   ├── Cargo.toml
│   └── src/
│       └── ...
├── src/
│   ├── main.rs
│   ├── adb/                    # ADB integration and device communication
│   ├── commands/               # Application actions and execution orchestration
│   ├── i18n/                   # Translation loading and formatting
│   ├── startup/                # Application initialization and configuration
│   └── ui/                     # egui/eframe interface
├── locales/                    # Translation files
├── docs/
│   ├── architecture.md
│   ├── development.md
│   ├── ui-design.md
│   └── STATUS.md
├── README.md
└── AGENTS.md
```

Create modules as responsibilities are implemented; do not add empty folders or unnecessary abstractions. Keep `core` independent of `egui` and `eframe`.

## ADB Requirements

- ADB must be available and its executable location must be resolved or configured reliably.
- Detect whether ADB can be started before attempting device operations.
- Do not assume a device is authorized, online, or available.
- Handle command failures, timeouts, interrupted operations, and unexpected output.
- Avoid exposing raw process details to the UI; translate them into useful application-level results and errors.

## Cross-Platform Builds

The application must compile for both Windows and Linux.

- Keep platform-specific behavior behind small, well-defined abstractions.
- Resolve application data, configuration, and log directories using platform-appropriate locations.
- Do not hardcode Windows-only paths or assume a particular shell.
- Document build prerequisites and commands for both target platforms.
- Test platform-specific process handling and path behavior where practical.

## Runtime Files

Configuration, logs, caches, and other generated files must be stored in an appropriate per-user application data directory. Do not write generated files into the installation directory unless the platform explicitly requires it.

## Documentation

- [Architecture](docs/architecture.md)
- [Development guide](docs/development.md)
- [UI design](docs/ui-design.md)
- [Status](docs/STATUS.md)
- [Agent guidelines](AGENTS.md)
