//! Centralized design tokens for the application interface.
//!
//! All palettes, semantic colors, font sizes, spacing, corner radii, and
//! control dimensions live here so the whole interface can be restyled in one
//! place.

use eframe::egui;
use egui::Color32;

use crate::startup::settings::ThemeMode;

/// Primary, semantic, and surface colors for one theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    /// Whether this palette is the dark variant.
    pub dark: bool,
    /// Window and panel background.
    pub background: Color32,
    /// Background of raised surfaces such as toolbars and dialogs.
    pub surface: Color32,
    /// Background of a hovered surface.
    pub surface_hover: Color32,
    /// Background of an active or pressed surface.
    pub surface_active: Color32,
    /// Alternating row background.
    pub stripe: Color32,
    /// Background of text and numeric inputs.
    pub input_background: Color32,
    /// Standard border and separator color.
    pub border: Color32,
    /// Primary text color.
    pub text_primary: Color32,
    /// Secondary and muted text color.
    pub text_secondary: Color32,
    /// Selection highlight color.
    pub selection: Color32,
    /// Primary action color.
    pub accent: Color32,
    /// Primary action hover color.
    pub accent_hover: Color32,
    /// Text drawn on the primary action color.
    pub on_accent: Color32,
    /// Destructive and error action color.
    pub danger: Color32,
    /// Destructive action hover color.
    pub danger_hover: Color32,
    /// Text drawn on the destructive action color.
    pub on_danger: Color32,
    /// Destructive and error text drawn on a normal surface.
    pub danger_text: Color32,
    /// Warning action color.
    pub warning: Color32,
    /// Warning action hover color.
    pub warning_hover: Color32,
    /// Text drawn on the warning action color.
    pub on_warning: Color32,
    /// Warning text drawn on a normal surface.
    pub warning_text: Color32,
    /// Confirmation and success action color.
    pub success: Color32,
    /// Confirmation action hover color.
    pub success_hover: Color32,
    /// Text drawn on the confirmation action color.
    pub on_success: Color32,
    /// Confirmation text drawn on a normal surface.
    pub success_text: Color32,
}

impl Palette {
    /// Palette used by the dark theme.
    pub const DARK: Self = Self {
        dark: true,
        background: Color32::from_rgb(18, 21, 27),
        surface: Color32::from_rgb(30, 35, 45),
        surface_hover: Color32::from_rgb(38, 44, 56),
        surface_active: Color32::from_rgb(46, 53, 67),
        stripe: Color32::from_rgb(24, 28, 36),
        input_background: Color32::from_rgb(14, 17, 22),
        border: Color32::from_rgb(51, 58, 71),
        text_primary: Color32::from_rgb(232, 235, 240),
        text_secondary: Color32::from_rgb(154, 164, 178),
        selection: Color32::from_rgb(52, 84, 138),
        accent: Color32::from_rgb(37, 99, 235),
        accent_hover: Color32::from_rgb(29, 78, 216),
        on_accent: Color32::WHITE,
        danger: Color32::from_rgb(214, 69, 69),
        danger_hover: Color32::from_rgb(226, 92, 92),
        on_danger: Color32::WHITE,
        danger_text: Color32::from_rgb(240, 122, 122),
        warning: Color32::from_rgb(240, 180, 41),
        warning_hover: Color32::from_rgb(245, 194, 74),
        on_warning: Color32::from_rgb(27, 20, 5),
        warning_text: Color32::from_rgb(240, 180, 41),
        success: Color32::from_rgb(46, 158, 91),
        success_hover: Color32::from_rgb(55, 178, 104),
        on_success: Color32::WHITE,
        success_text: Color32::from_rgb(76, 203, 124),
    };

    /// Palette used by the light theme.
    pub const LIGHT: Self = Self {
        dark: false,
        background: Color32::from_rgb(244, 246, 250),
        surface: Color32::WHITE,
        surface_hover: Color32::from_rgb(238, 242, 248),
        surface_active: Color32::from_rgb(226, 233, 243),
        stripe: Color32::from_rgb(248, 250, 253),
        input_background: Color32::WHITE,
        border: Color32::from_rgb(213, 220, 230),
        text_primary: Color32::from_rgb(27, 35, 48),
        text_secondary: Color32::from_rgb(91, 102, 117),
        selection: Color32::from_rgb(191, 214, 255),
        accent: Color32::from_rgb(31, 111, 235),
        accent_hover: Color32::from_rgb(59, 130, 246),
        on_accent: Color32::WHITE,
        danger: Color32::from_rgb(198, 40, 40),
        danger_hover: Color32::from_rgb(211, 47, 47),
        on_danger: Color32::WHITE,
        danger_text: Color32::from_rgb(179, 38, 30),
        warning: Color32::from_rgb(240, 180, 41),
        warning_hover: Color32::from_rgb(245, 194, 74),
        on_warning: Color32::from_rgb(27, 20, 5),
        warning_text: Color32::from_rgb(138, 90, 0),
        success: Color32::from_rgb(30, 142, 78),
        success_hover: Color32::from_rgb(36, 161, 89),
        on_success: Color32::WHITE,
        success_text: Color32::from_rgb(20, 122, 63),
    };

    /// Returns the palette for `mode`.
    pub fn for_mode(mode: ThemeMode) -> &'static Self {
        match mode {
            ThemeMode::Dark => &Self::DARK,
            ThemeMode::Light => &Self::LIGHT,
        }
    }
}

/// Font sizes, in points.
pub mod font_size {
    /// Regular body text.
    pub const BODY: f32 = 15.0;
    /// Small supporting text.
    pub const SMALL: f32 = 12.0;
    /// Button labels.
    pub const BUTTON: f32 = 14.0;
    /// Section headings.
    pub const HEADING: f32 = 20.0;
    /// Monospaced identifiers.
    pub const MONOSPACE: f32 = 13.0;
}

/// Corner radii, in points.
pub mod radius {
    /// Buttons and inputs.
    pub const CONTROL: u8 = 8;
    /// Panels and windows.
    pub const PANEL: u8 = 12;
}

/// Spacing and control dimensions, in points.
pub mod size {
    /// Extra-small spacing.
    pub const SPACING_XS: f32 = 4.0;
    /// Small spacing.
    pub const SPACING_SM: f32 = 8.0;
    /// Medium spacing.
    pub const SPACING_MD: f32 = 12.0;
    /// Height of buttons, inputs, and other interactive controls.
    pub const CONTROL_HEIGHT: f32 = 30.0;
    /// Minimum width of a standard button.
    pub const BUTTON_WIDTH: f32 = 96.0;
    /// Minimum width of a compact row action button.
    pub const BUTTON_WIDTH_COMPACT: f32 = 86.0;
    /// Width of the application search field.
    pub const SEARCH_WIDTH: f32 = 280.0;
    /// Width of the device selector.
    pub const DEVICE_SELECTOR_WIDTH: f32 = 260.0;
    /// Height of one application row.
    pub const ROW_HEIGHT_APPS: f32 = 48.0;
    /// Height of one process row.
    pub const ROW_HEIGHT_PROCESSES: f32 = 28.0;
    /// Horizontal padding inside a list row.
    pub const ROW_PADDING: f32 = 8.0;
    /// Padding inside panels.
    pub const PANEL_PADDING: i8 = 12;
    /// Horizontal padding of an input field.
    pub const INPUT_PADDING_X: i8 = 8;
    /// Vertical padding of an input field.
    pub const INPUT_PADDING_Y: i8 = 6;
}

/// Semantic role of a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonTone {
    /// Primary action, such as refresh or load.
    Primary,
    /// Routine or neutral action.
    Neutral,
    /// Destructive or risky action.
    Danger,
    /// Action that may cause a warning or loss of functionality.
    Warning,
    /// Confirmation or enabling action.
    Success,
}

impl ButtonTone {
    /// Background and text colors for this tone.
    fn colors(self, palette: &Palette) -> (Color32, Color32) {
        match self {
            Self::Primary => (palette.accent, palette.on_accent),
            Self::Neutral => (palette.surface, palette.text_primary),
            Self::Danger => (palette.danger, palette.on_danger),
            Self::Warning => (palette.warning, palette.on_warning),
            Self::Success => (palette.success, palette.on_success),
        }
    }
}

/// Builds a rounded button using the semantic color for `tone`.
pub fn toned_button(text: &str, tone: ButtonTone, palette: &Palette) -> egui::Button<'static> {
    let (fill, foreground) = tone.colors(palette);

    egui::Button::new(
        egui::RichText::new(text)
            .color(foreground)
            .size(font_size::BUTTON),
    )
    .fill(fill)
    .corner_radius(egui::CornerRadius::same(radius::CONTROL))
    .min_size(egui::vec2(size::BUTTON_WIDTH, size::CONTROL_HEIGHT))
}

/// Returns the palette of the currently active theme.
pub fn palette(ui: &egui::Ui) -> &'static Palette {
    if ui.visuals().dark_mode {
        Palette::for_mode(ThemeMode::Dark)
    } else {
        Palette::for_mode(ThemeMode::Light)
    }
}

/// Applies the selected theme, spacing, typography, and rounded controls.
pub fn apply(context: &egui::Context, mode: ThemeMode) {
    context.set_visuals_of(egui::Theme::Dark, visuals(&Palette::DARK));
    context.set_visuals_of(egui::Theme::Light, visuals(&Palette::LIGHT));

    context.all_styles_mut(|style| {
        style.spacing.item_spacing = egui::vec2(size::SPACING_SM, size::SPACING_SM);
        style.spacing.button_padding = egui::vec2(size::SPACING_MD, size::SPACING_XS + 2.0);
        style.spacing.interact_size =
            egui::vec2(style.spacing.interact_size.x, size::CONTROL_HEIGHT);
        style.spacing.text_edit_width = size::SEARCH_WIDTH;
        style.spacing.window_margin = egui::Margin::same(size::PANEL_PADDING);
        style.spacing.menu_margin = egui::Margin::same(size::SPACING_SM as i8);
        style.text_styles = text_styles();
    });

    context.set_theme(match mode {
        ThemeMode::Dark => egui::Theme::Dark,
        ThemeMode::Light => egui::Theme::Light,
    });
}

fn text_styles() -> std::collections::BTreeMap<egui::TextStyle, egui::FontId> {
    use egui::{FontFamily, FontId, TextStyle};

    [
        (
            TextStyle::Heading,
            FontId::new(font_size::HEADING, FontFamily::Proportional),
        ),
        (
            TextStyle::Body,
            FontId::new(font_size::BODY, FontFamily::Proportional),
        ),
        (
            TextStyle::Button,
            FontId::new(font_size::BUTTON, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(font_size::SMALL, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(font_size::MONOSPACE, FontFamily::Proportional),
        ),
    ]
    .into()
}

fn visuals(palette: &Palette) -> egui::Visuals {
    let mut visuals = if palette.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    visuals.panel_fill = palette.background;
    visuals.window_fill = palette.surface;
    visuals.window_stroke = egui::Stroke::new(1.0, palette.border);
    visuals.faint_bg_color = palette.stripe;
    visuals.extreme_bg_color = palette.input_background;
    visuals.text_edit_bg_color = Some(palette.input_background);
    visuals.code_bg_color = palette.surface;
    visuals.selection.bg_fill = palette.selection;
    visuals.selection.stroke = egui::Stroke::new(1.0, palette.text_primary);
    visuals.hyperlink_color = palette.accent;
    visuals.warn_fg_color = palette.warning_text;
    visuals.error_fg_color = palette.danger_text;
    visuals.weak_text_color = Some(palette.text_secondary);

    visuals.widgets.noninteractive.bg_fill = palette.surface;
    visuals.widgets.noninteractive.weak_bg_fill = palette.surface;
    visuals.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, palette.border);
    visuals.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, palette.text_primary);
    visuals.widgets.noninteractive.corner_radius = egui::CornerRadius::same(radius::CONTROL);

    visuals.widgets.inactive.bg_fill = palette.surface;
    visuals.widgets.inactive.weak_bg_fill = palette.surface;
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, palette.border);
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, palette.text_primary);
    visuals.widgets.inactive.corner_radius = egui::CornerRadius::same(radius::CONTROL);

    visuals.widgets.hovered.bg_fill = palette.surface_hover;
    visuals.widgets.hovered.weak_bg_fill = palette.surface_hover;
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, palette.accent);
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, palette.text_primary);
    visuals.widgets.hovered.corner_radius = egui::CornerRadius::same(radius::CONTROL);

    visuals.widgets.active.bg_fill = palette.surface_active;
    visuals.widgets.active.weak_bg_fill = palette.surface_active;
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, palette.accent);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, palette.text_primary);
    visuals.widgets.active.corner_radius = egui::CornerRadius::same(radius::CONTROL);

    visuals.widgets.open.bg_fill = palette.surface_hover;
    visuals.widgets.open.weak_bg_fill = palette.surface_hover;
    visuals.widgets.open.bg_stroke = egui::Stroke::new(1.0, palette.accent);
    visuals.widgets.open.fg_stroke = egui::Stroke::new(1.0, palette.text_primary);
    visuals.widgets.open.corner_radius = egui::CornerRadius::same(radius::CONTROL);

    visuals.window_corner_radius = egui::CornerRadius::same(radius::PANEL);
    visuals.menu_corner_radius = egui::CornerRadius::same(radius::CONTROL);

    visuals
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_palette_is_dark_and_light_palette_is_not() {
        let dark = Palette::for_mode(ThemeMode::Dark);
        let light = Palette::for_mode(ThemeMode::Light);

        assert!(dark.dark);
        assert!(!light.dark);
    }

    #[test]
    fn dark_and_light_palettes_use_distinct_backgrounds() {
        assert_ne!(Palette::DARK.background, Palette::LIGHT.background);
        assert_ne!(Palette::DARK.text_primary, Palette::LIGHT.text_primary);
    }

    #[test]
    fn semantic_colors_contrast_with_their_foregrounds() {
        for palette in [&Palette::DARK, &Palette::LIGHT] {
            assert_ne!(palette.danger, palette.on_danger);
            assert_ne!(palette.warning, palette.on_warning);
            assert_ne!(palette.success, palette.on_success);
            assert_ne!(palette.accent, palette.on_accent);
        }
    }

    #[test]
    fn primary_button_text_reaches_readable_contrast() {
        for palette in [&Palette::DARK, &Palette::LIGHT] {
            let ratio = contrast_ratio(palette.accent, palette.on_accent);
            assert!(ratio >= 4.5, "primary button contrast is only {ratio:.2}");
        }
    }

    fn contrast_ratio(first: Color32, second: Color32) -> f64 {
        let first = relative_luminance(first);
        let second = relative_luminance(second);
        let (lighter, darker) = if first > second {
            (first, second)
        } else {
            (second, first)
        };

        (lighter + 0.05) / (darker + 0.05)
    }

    fn relative_luminance(color: Color32) -> f64 {
        let channel = |value: u8| {
            let value = f64::from(value) / 255.0;

            if value <= 0.04045 {
                value / 12.92
            } else {
                ((value + 0.055) / 1.055).powf(2.4)
            }
        };

        0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
    }

    #[test]
    fn button_tones_use_the_semantic_palette_colors() {
        let palette = &Palette::DARK;

        assert_eq!(
            ButtonTone::Danger.colors(palette),
            (palette.danger, palette.on_danger)
        );
        assert_eq!(
            ButtonTone::Warning.colors(palette),
            (palette.warning, palette.on_warning)
        );
        assert_eq!(
            ButtonTone::Success.colors(palette),
            (palette.success, palette.on_success)
        );
        assert_eq!(
            ButtonTone::Primary.colors(palette),
            (palette.accent, palette.on_accent)
        );
    }
}
