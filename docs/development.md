# Development Guide

Rules and practices for working on Android Utility.

## Language and Documentation Rules

- All code comments and documentation (including README files and API documentation) must be written entirely in English.
- User-facing text follows the internationalization rules in [architecture.md](architecture.md#internationalization-i18n).
- Use clear, descriptive names.
- Follow Rust naming conventions:
  - `snake_case` for functions, variables, and modules.
  - `UpperCamelCase` for types and traits.
  - `SCREAMING_SNAKE_CASE` for constants.
- Use `cargo fmt` formatting conventions.
- Keep `core` free of `egui` and `eframe` dependencies.

## Code Rules

- Apply the Single Responsibility Principle (SRP).
- Keep business logic independent of UI and external process execution.
- Keep ADB-specific code inside the ADB integration layer.
- Represent meaningful application actions explicitly through commands or equivalent handlers.
- Use stable identifiers for commands when they form part of a log, automation, persistence, or other external contract.
- Prefer small, cohesive functions and avoid unnecessary abstractions.
- Avoid global mutable state unless there is a clear, documented reason.
- Validate external input, including user input, process output, and device-provided data.
- Follow the error handling rules in [architecture.md](architecture.md#error-handling).

## ADB and Process Execution

- Pass executable arguments as separate arguments; avoid shell command construction.
- Never concatenate untrusted values into commands.
- Apply sensible timeouts to commands that may hang.
- Capture and interpret exit status, standard output, and standard error.
- Handle missing ADB, missing devices, multiple devices, unauthorized devices, offline devices, and disconnected devices.
- Do not assume that a command is supported by every Android version or device.
- Validate parsed output and handle unknown fields or format variations.
- Avoid logging sensitive data or unrestricted command output.
- Make destructive or state-changing device operations explicit and provide appropriate confirmation in the UI.

## Adding UI Text

1. Add a translation key to the locale files.
2. Use the internationalization loader to retrieve the text.
3. Do not hardcode user-facing labels, messages, notifications, or tooltips.

## Testing

- Every new feature, behavior change, or bug fix must include automated unit or integration tests where practical.
- Test core logic independently from the UI and ADB.
- Test ADB argument construction, process results, timeouts, exit codes, and output parsing with controlled fixtures or test doubles.
- Include cases for empty, malformed, incomplete, and unexpected external data.
- Include error and cancellation scenarios.
- Keep tests deterministic; do not require a connected Android device for ordinary unit tests.
- Put tests in the conventionally appropriate test locations for the selected language and framework.
- Device-dependent tests must be separate, clearly marked, and opt-in where necessary.

## Quality Checks

Before finishing a change, run formatting, static analysis, build, and automated test checks. Resolve warnings and errors introduced by the change.

Run the following checks from the workspace root:

```text
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Verify compilation for the supported Windows and Linux targets in CI or an equivalent build environment. Cross-compilation may require target-specific toolchains and system dependencies; document those prerequisites.

## Checklist for Changes

Use this checklist for each change:

1. Read [STATUS.md](STATUS.md) to understand the current project state.
2. Identify the correct architectural layer and responsibility (`core`, application/commands, ADB integration, i18n, startup, or UI).
3. Check whether an existing abstraction or feature should be extended before adding a new one.
4. Implement the change without violating dependency boundaries.
5. Keep long-running work off the main UI thread.
6. Add or update automated tests.
7. Add translations for any new user-facing text.
8. Run `cargo fmt --check`, `cargo check`, `cargo test`, and `cargo clippy`; verify Windows and Linux builds when the change affects platform compatibility.
9. Update [STATUS.md](STATUS.md) with completed work and remaining tasks.
10. Update [AGENTS.md](../AGENTS.md) only when important project-wide agent guidance changes.

If a requirement is unclear or an implementation decision would establish a significant project convention, ask for clarification instead of guessing.
