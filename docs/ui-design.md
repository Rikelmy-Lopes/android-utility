# UI Design

Guidelines for the Android Utility graphical interface.

## Technology

- GUI framework: `egui` via `eframe`.
- UI code belongs in the application package, separate from the GUI-independent core.
- UI components communicate with application services and commands; they do not execute ADB processes directly.

## General Principles

- The interface must be clear, consistent, predictable, and easy to navigate.
- Prioritize information hierarchy and readability over decoration.
- Use consistent spacing, alignment, typography, and component behavior.
- Avoid unnecessary visual elements, duplicated information, and excessive dialogs.
- Use familiar interaction patterns and provide clear feedback for user actions.

## Architecture and Responsiveness

- The UI is a consumer of application-level services and commands.
- UI components must not execute ADB processes or contain business rules.
- The main UI thread must never be blocked by device communication, file I/O, or expensive analysis.
- Long-running operations must run asynchronously or in background workers.
- Display progress when useful, and provide cancellation when supported.
- Keep the interface usable when no device is connected or when a device becomes unavailable during an operation.

## Device State

Make device state visible and unambiguous.

- Distinguish no device detected, one device connected, multiple devices connected, unauthorized, offline, and unavailable states where applicable.
- Identify the selected device clearly before running device-specific actions.
- Do not imply that an operation succeeded until its result has been confirmed.
- Refresh displayed device information when it may have become stale.
- Explain when an action is unavailable and why.

## Feedback and Errors

- Every user action should provide appropriate feedback.
- Show progress for operations that take noticeable time.
- Present success, warning, and error states consistently.
- Error messages must be understandable and actionable.
- Avoid exposing raw stack traces or overwhelming process output in normal UI messages.
- Make detailed diagnostic information available through logs or an appropriate details view.
- Confirm destructive or state-changing actions before execution.

## Themes

- Support dark and light themes when the chosen UI framework allows it.
- Use a consistent, restrained color palette in both themes.
- Ensure sufficient contrast and readable text in every theme.
- Persist the user's selected theme in application configuration.

## Text and Internationalization

- Do not hardcode user-facing text in UI components.
- Labels, messages, tooltips, accessibility descriptions, and notifications must use the i18n system.
- Design layouts to accommodate longer translations.
- Format dates, times, numbers, and units according to the active locale.

## Data Presentation

- Present device information in clearly grouped sections.
- Use tables, lists, or detail panels when they improve comparison and scanning.
- Distinguish unavailable values from empty values and zero values.
- Avoid overwhelming the user with raw ADB output; provide structured information first and raw details only when useful.
- Make copying or exporting information explicit when those features are available.

## Accessibility and Usability

- Use clear labels and meaningful control names.
- Provide keyboard navigation and visible focus states where supported.
- Do not rely on color alone to communicate status.
- Use tooltips for secondary explanations, not as the only source of essential information.
- Keep destructive actions visually and behaviorally distinct from routine actions.

## Goal

The interface must remain responsive and understandable while inspecting devices, collecting information, and presenting analysis results.
