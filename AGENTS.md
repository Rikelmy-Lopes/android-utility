# AGENTS.md — AI Agent Guidelines (Android Utility)

Guidelines for AI agents working on **Android Utility**. Read the relevant documentation before changing code.

## 1. Project Overview

- **Name:** Android Utility
- **Language:** Rust
- **GUI:** `egui` (via `eframe`)
- **Target platforms:** Windows and Linux
- **Objective:** Provide a desktop application for inspecting and analyzing Android devices through Android Debug Bridge (ADB).
- **Documentation:** See [README.md](README.md) and the documents in `docs/`.

## 2. Key Rules

- Keep business logic independent of the GUI, operating system, and external tools.
- Isolate ADB communication behind a dedicated integration layer. Business logic must not execute shell commands directly.
- The `core` crate must not depend on `egui` or `eframe`.
- Represent user-triggered operations as commands or equivalent application actions with clear responsibilities and stable identifiers where applicable.
- Never freeze or block the main UI thread. Long-running device operations must execute asynchronously or in background workers.
- Do not use unchecked error handling or terminate the application because an expected operation failed. Propagate and handle errors explicitly.
- Do not hardcode user-facing text. Use the internationalization system.
- Apply the Single Responsibility Principle (SRP) consistently.
- Keep names descriptive and responsibilities cohesive.
- Comments and documentation must be written in English.
- Every new feature, behavior change, or bug fix must include automated tests where practical.
- Do not assume that an ADB command succeeded. Check its exit status, output, and relevant device state.
- Treat device output and data as external input: validate it and handle missing, malformed, or unexpected values safely.
- Keep platform-specific behavior compatible with both Windows and Linux.

## 3. Architecture and Development

Read [docs/architecture.md](docs/architecture.md) for architectural boundaries and [docs/development.md](docs/development.md) for implementation, testing, and quality rules.

## 4. UI Guidelines

Read [docs/ui-design.md](docs/ui-design.md). The UI must remain responsive, communicate operation progress, and present device and command errors clearly.

## 5. Instructions for Changes

Follow the checklist in [docs/development.md](docs/development.md#checklist-for-changes). Read [docs/STATUS.md](docs/STATUS.md) before starting work and update it after completing a task.

## 6. Related Documentation

- [README.md](README.md) — Project overview and structure.
- [docs/architecture.md](docs/architecture.md) — Architecture and design principles.
- [docs/development.md](docs/development.md) — Development rules, testing, and quality checks.
- [docs/ui-design.md](docs/ui-design.md) — User interface guidelines.
- [docs/STATUS.md](docs/STATUS.md) — Current project status.
