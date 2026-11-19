# AGENTS.md — AI Agent Guidelines (Android Utility)

Guidelines for AI agents working on **Android Utility**. Read the relevant documentation before changing code.

## 1. Project Overview

- **Name:** Android Utility
- **Language:** Rust
- **GUI:** `egui` (via `eframe`)
- **Target platforms:** Windows and Linux
- **Objective:** Provide a desktop application for inspecting and analyzing Android devices through Android Debug Bridge (ADB).

## 2. Rules

All project rules are defined in the documents below. Read them before changing code.

- [docs/architecture.md](docs/architecture.md) — Architecture, layer boundaries, error handling, i18n, and cross-platform rules.
- [docs/development.md](docs/development.md) — Code rules, unit testing, quality checks, and the checklist for changes.
- [docs/ui-design.md](docs/ui-design.md) — User interface guidelines.

## 3. Instructions for Changes

Follow the checklist in [docs/development.md](docs/development.md#checklist-for-changes). Read [docs/STATUS.md](docs/STATUS.md) before starting work and update it after completing a task.

## 4. Related Documentation

- [README.md](README.md) — Project overview and structure.
- [docs/adb-commands.md](docs/adb-commands.md) — ADB commands, outputs, and minimum Android versions.
- [docs/STATUS.md](docs/STATUS.md) — Current project status.
