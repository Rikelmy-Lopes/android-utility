# UI Design

Guidelines for the Android Utility graphical interface. Layer boundaries, asynchronous execution, and internationalization rules are defined in [architecture.md](architecture.md) and are not repeated here.

## General Principles

- The interface must be clear, consistent, predictable, and easy to navigate.
- Prioritize information hierarchy and readability over decoration.
- Use consistent spacing, alignment, typography, and component behavior.
- Avoid unnecessary visual elements, duplicated information, and excessive dialogs.
- Use familiar interaction patterns and provide clear feedback for user actions.
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
- Show progress for operations that take noticeable time, and provide cancellation when supported.
- Present success, warning, and error states consistently.
- Error messages must be understandable and actionable.
- Avoid exposing raw stack traces or overwhelming process output in normal UI messages.
- Make detailed diagnostic information available through logs or an appropriate details view.
- Confirm destructive or state-changing actions before execution.

## Layout and Alignment

- Build every screen from structured horizontal and vertical layout containers. Do not place controls at absolute positions.
- Align related controls on a shared baseline and use the same vertical alignment for rows, toolbars, and action groups.
- Keep toolbars, tab bars, list rows, and status bars vertically centered.
- Right-align numeric columns and row action buttons. Left-align names, labels, and identifiers.
- Let lists and panels fill the available width instead of assuming a fixed window size.
- Use one consistent spacing rhythm:
  - Extra-small spacing: 4 points.
  - Small spacing: 8 points.
  - Medium spacing: 12 points.
  - Panel padding: 12 points.
  - List row horizontal padding: 8 points.
- Use a consistent interactive control height of 30 points for buttons, inputs, and selectors.
- Design for longer translations. Leave room for the longest expected label instead of truncating routine actions.

## Shape and Borders

- Use rounded corners on every button, input field, selector, combo box, and numeric control.
- Use an 8-point corner radius for controls and a 12-point radius for panels, windows, and dialogs.
- Keep border widths consistent. Use the palette border color for resting controls and the accent color for focus, hover, and active states.
- Do not mix sharp and rounded controls in the same view.

## Semantic Colors

Action colors must communicate meaning. Use them consistently across all panels.

- **Primary (blue):** routine positive actions such as refresh and load.
- **Danger (red):** destructive or risky actions such as uninstall, and error messages.
- **Warning (amber):** actions that may reduce functionality, such as disabling an application, and system-application warnings.
- **Success (green):** confirmations and completed operations, such as enabling an application.
- **Neutral (surface):** ordinary actions such as cancel or resolving a name.

Rules:

- Pair every semantic color with a clear label, icon, or message. Never rely on color alone.
- Ensure each semantic fill has a foreground color with sufficient contrast in both themes.
- Use the semantic foreground color from the palette whenever text or an icon is drawn on a semantic fill.
- Keep destructive actions visually distinct and confirm them before execution.

## Themes

Support both dark and light themes.

- The toolbar provides an explicit dark/light theme switcher. Do not rely only on the operating-system theme.
- Apply the selected theme before the first frame is drawn.
- Persist the selected theme in the application configuration file.
- Default to the dark theme when no saved preference exists.
- Use a consistent, restrained palette in both themes. Keep surface, border, text, and semantic colors paired per theme.
- Ensure sufficient contrast for body text, weak text, disabled text, buttons, inputs, and semantic fills in both themes.
- Do not hardcode colors in individual panels. Read them from the active theme palette.

## Design Tokens

All visual constants are defined in one place: `src/ui/theme.rs`.

- The `Palette` type defines every surface, text, accent, danger, warning, success, and selection color for both themes.
- The `font_size` module defines body, small, button, heading, and monospaced text sizes.
- The `size` module defines spacing, control heights, button widths, row heights, input padding, and panel padding.
- The `radius` module defines control and panel corner radii.
- `theme::apply` converts the tokens into framework visuals, spacing, typography, and rounded widget defaults.
- `theme::toned_button` builds a rounded button with a semantic tone.

Rules:

- Do not hardcode a color, font size, corner radius, or control dimension in a panel or dialog.
- Add a new token when a new reusable visual value is needed.
- Update a token in `src/ui/theme.rs` when a visual value must change globally.
- Keep tokens independent of business logic and ADB data.
- Cover palette and tone mapping with unit tests when adding or changing semantic colors.

## Text

- Labels, messages, tooltips, accessibility descriptions, and notifications must use the i18n system.
- Design layouts to accommodate longer translations.

## Data Presentation

- Present device information in clearly grouped sections.
- Use tables, lists, or detail panels when they improve comparison and scanning.
- Distinguish unavailable values from empty values and zero values.
- Provide structured information first and raw details only when useful.
- Make copying or exporting information explicit when those features are available.

## Accessibility and Usability

- Use clear labels and meaningful control names.
- Provide keyboard navigation and visible focus states where supported.
- Do not rely on color alone to communicate status.
- Use tooltips for secondary explanations, not as the only source of essential information.
- Keep destructive actions visually and behaviorally distinct from routine actions.
