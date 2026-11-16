# Development Guide

Rules and practices for working on Android Utility. Architectural rules are defined in [architecture.md](architecture.md) and are not repeated here.

## Language and Documentation Rules

- All code comments and documentation (including README files and API documentation) must be written entirely in English.
- Use clear, descriptive names.
- Follow Rust naming conventions:
  - `snake_case` for functions, variables, and modules.
  - `UpperCamelCase` for types and traits.
  - `SCREAMING_SNAKE_CASE` for constants.
- Use `cargo fmt` formatting conventions.

## Code Rules

- Prefer small, cohesive functions and avoid unnecessary abstractions.
- Avoid global mutable state unless there is a clear, documented reason.
- Validate external input, including user input, process output, and device-provided data. Handle unknown fields and format variations.
- Capture and interpret exit status, standard output, and standard error of every process.
- Handle missing ADB, missing devices, multiple devices, unauthorized devices, offline devices, and disconnected devices.

## Adding UI Text

1. Add a translation key to the locale files.
2. Use the internationalization loader to retrieve the text.

## Testing

Only unit tests are written in this project. Integration tests are not used.

- Every new feature, behavior change, or bug fix must include automated unit tests where practical.
- Unit tests must not depend on ADB, a connected Android device, or process execution.
- Test core logic independently from the UI and ADB.
- Test ADB argument construction, process results, timeouts, exit codes, and output parsing with controlled fixtures or test doubles.
- Include cases for empty, malformed, incomplete, and unexpected external data.
- Include error and cancellation scenarios.
- Keep tests deterministic.
- Put unit tests in the conventional location for Rust.

## Quality Checks

Before finishing a change, run formatting, static analysis, build, and automated test checks. Resolve warnings and errors introduced by the change.

Run the following checks from the workspace root:

```text
cargo fmt --check
cargo check
cargo test
cargo clippy --all-targets --all-features -- -D warnings
```

Verify compilation for the supported Windows and Linux targets in CI or an equivalent build environment. Cross-compilation may require target-specific toolchains and system dependencies; document those prerequisites and the build commands for both platforms.

## Checklist for Changes

1. Read [STATUS.md](STATUS.md) to understand the current project state.
2. Identify the correct architectural layer and responsibility (`core`, application/commands, ADB integration, i18n, startup, or UI).
3. Check whether an existing abstraction or feature should be extended before adding a new one.
4. Implement the change following [architecture.md](architecture.md).
5. Add or update automated unit tests.
6. Add translations for any new user-facing text.
7. Run the quality checks; verify Windows and Linux builds when the change affects platform compatibility.
8. Update [STATUS.md](STATUS.md) with completed work and remaining tasks.
9. Update [AGENTS.md](../AGENTS.md) only when important project-wide agent guidance changes.

If a requirement is unclear or an implementation decision would establish a significant project convention, ask for clarification instead of guessing.
