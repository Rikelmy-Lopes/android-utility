# Architecture

This document defines the architecture and design principles of Android Utility.

## Clean Architecture

The application follows Clean Architecture principles.

- Divide the application into layers with clear responsibilities.
- Keep business rules at the center of the application.
- Keep the UI, operating-system APIs, process execution, and external tools at the boundaries.
- Inner layers must not depend on outer layers or on concrete UI/framework implementations.
- Dependencies should point toward the business rules.

## Layer Responsibilities

### Rust Workspace and Core / Business Logic

The application uses a Cargo workspace with two logical parts:

- **`core`** — a separate crate containing GUI-independent business logic.
- **Application package** — contains application orchestration, ADB integration, internationalization, startup, and the `egui`/`eframe` UI.

The core crate must not depend on `egui`, `eframe`, or other presentation-layer components.

#### Core Responsibilities

Responsible for:

- Application-independent business rules.
- Domain models and value types.
- Analysis and interpretation of collected device information.
- Application-level validation and domain errors.

Rules:

- Must not depend on the GUI framework.
- Must not execute ADB commands or spawn operating-system processes.
- Must not contain rendering, layout, or presentation logic.
- Should expose clear APIs that can be tested without a GUI or connected device.

### ADB Integration

Responsible for:

- Locating and invoking the ADB executable.
- Managing process arguments, standard output, standard error, exit codes, and timeouts.
- Discovering connected devices and retrieving device data.
- Parsing and validating ADB output into application-level types.
- Representing connection, authorization, availability, and command failures.

Rules:

- Keep process execution and ADB-specific details inside this layer.
- Prefer argument-based process invocation over building shell command strings.
- Never interpolate untrusted input into shell commands.
- Do not assume output format or command success without validation.
- Keep ADB-specific output types separate from domain models where practical.
- Make the integration testable by allowing process execution or the ADB client to be substituted with a test double.

### Application / Commands

Responsible for:

- Coordinating user actions and application workflows.
- Calling core services and ADB integration through defined interfaces.
- Managing asynchronous execution, cancellation, and progress reporting.
- Returning structured results for the UI to display.

Rules:

- The UI must not execute ADB processes directly.
- The UI must not modify business state through hidden side effects.
- Each action should have one clear responsibility.
- Use stable command identifiers when commands are externally referenced, logged, or persisted. Once published, an identifier must not be renamed casually.

### UI / Presentation

The graphical interface uses `egui` through `eframe`. UI code belongs in the application layer and must consume application services and commands.

Responsible for:

- Displaying application state and analysis results.
- Collecting user input.
- Dispatching application actions.
- Showing progress, success, warnings, and errors.

Rules:

- Must not contain business rules or ADB process logic.
- Must not block while waiting for device operations.
- Must consume application-level results rather than parse raw ADB output.

## Core / GUI Decoupling

- `core` must not depend on `egui` or `eframe`.
- The core crate must not contain rendering, layout, windowing, or event-loop code.
- The GUI consumes the core through exposed APIs and application commands.
- A change to the GUI framework must not require changes to business rules.

## Dependency Direction

The intended dependency direction is:

```text
UI
 |
 v
Application / Commands
 |             |
 v             v
Core       ADB Integration
```

The core must remain independent. Where an outer integration needs to provide data or behavior to the core, use interfaces or application services so that the core does not depend on the concrete integration.

## Command Pattern

Represent meaningful user actions as commands or equivalent explicit application actions.

- The UI dispatches an action to an application-level handler or execution queue.
- The handler coordinates the required services.
- The UI receives structured results and state updates.
- Long-running actions execute asynchronously.
- Commands should be independently testable without rendering a UI.

When stable command identifiers are needed:

- Define each identifier as a constant.
- Use a consistent, lowercase, domain.action naming format.
- Treat published identifiers as stable contracts.

## Asynchronous Execution and Cancellation

- Never perform blocking process execution, file I/O, device polling, or expensive analysis on the main UI thread.
- Apply timeouts to external commands and operations that may hang.
- Support cancellation for long-running operations when the underlying operation allows it.
- Ensure cancellation and completion update application state consistently.
- Avoid concurrent operations that conflict over the same device unless explicitly supported.
- Report progress when an operation has meaningful stages or measurable progress.

## Error Handling

- Handle errors explicitly; do not silently discard failures.
- Avoid unchecked operations that can crash production code.
- Use structured error types at architectural boundaries.
- Preserve useful diagnostic details in logs, but show clear and actionable messages to users.
- Distinguish between ADB unavailable, no device connected, device unauthorized, device offline, command failure, timeout, cancellation, and malformed output where applicable.
- Never treat partial or invalid device data as a successful complete result.

## Performance

- Keep the UI responsive during device discovery, data collection, and analysis.
- Avoid unnecessary allocations, copies, and repeated parsing.
- Do not poll devices more frequently than necessary.
- Prefer bounded output handling for commands that may return large amounts of data.
- Cache data only when its validity and invalidation rules are clear.
- Measure before introducing complex concurrency or optimization.

## Internationalization (i18n)

- Build the application so additional languages can be added without changing business logic.
- Store translations in separate locale files.
- Do not hardcode user-facing text in the core, application, or UI layers.
- Use a defined fallback locale when a translation is missing.
- Format numbers, dates, times, and plural forms according to the active locale.

## Cross-Platform Compatibility

The application must compile for Windows and Linux.

- Isolate operating-system-specific behavior behind small interfaces or modules.
- Use platform-aware APIs or libraries for application data and configuration paths.
- Do not assume Windows drive letters, path separators, registry locations, or PowerShell.
- Do not assume a particular Linux distribution, shell, or package manager.
- Handle platform differences in process invocation, executable discovery, permissions, and environment variables explicitly.
- Keep the ADB integration behavior consistent across supported platforms.

## Logging and Configuration

- Store logs and user configuration in an appropriate per-user application data directory.
- Do not log sensitive device data, personal information, or command output indiscriminately.
- Make log verbosity configurable where appropriate.
- Keep configuration loading and persistence outside business logic.
- Handle missing, invalid, or outdated configuration safely.

## Single Responsibility Principle (SRP)

Apply SRP throughout the project.

### What Is a Responsibility

A responsibility is one reason to change: one cohesive concept or capability.

Examples:

- ADB process execution.
- Device discovery.
- Parsing a specific category of device information.
- A single analysis operation.
- A command handler.
- A UI component.
- Translation loading.

A useful test: if a file cannot be described in one sentence without using "and", it may contain more than one responsibility.

### Granularity

- A folder represents a cohesive feature or area.
- A file represents one cohesive concept, such as a type and its closely related implementation.
- A function should have one clear reason to change.
- Keep closely related types and private helpers together when they always change for the same reason.
- Do not create one file per trivial function or split code into tiny files without a clear benefit.
- Split a file when it gains a second responsibility or contains unrelated concepts.

Do not mix responsibilities merely to reduce the number of files. Do not create unnecessary abstractions or layers without a concrete need.

## Testing and Extensibility

- Core logic must be testable without ADB or a physical device.
- Keep parsing tests independent from live device tests.
- Design interfaces around actual application needs rather than mirroring every ADB command.
