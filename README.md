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

Create modules as responsibilities are implemented; do not add empty folders or unnecessary abstractions.

## Documentation

- [Architecture](docs/architecture.md)
- [Development guide](docs/development.md)
- [UI design](docs/ui-design.md)
- [Status](docs/STATUS.md)
- [Agent guidelines](AGENTS.md)
