//! Android application domain models.

use std::fmt;

/// Maximum accepted length of a package name.
const MAX_PACKAGE_NAME_LENGTH: usize = 255;

/// Maximum accepted length of a single package name segment.
const MAX_SEGMENT_LENGTH: usize = 63;

/// A validated Android package name such as `com.example.application`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct PackageName(String);

impl PackageName {
    /// Validates and wraps a raw package name.
    pub fn parse(raw: &str) -> Result<Self, PackageNameError> {
        if raw.is_empty() {
            return Err(PackageNameError::Empty);
        }

        if raw.len() > MAX_PACKAGE_NAME_LENGTH {
            return Err(PackageNameError::TooLong);
        }

        let mut segment_count = 0;
        for segment in raw.split('.') {
            segment_count += 1;

            if segment.is_empty() {
                return Err(PackageNameError::EmptySegment);
            }

            if segment.len() > MAX_SEGMENT_LENGTH {
                return Err(PackageNameError::SegmentTooLong);
            }

            if segment.starts_with(|character: char| character.is_ascii_digit()) {
                return Err(PackageNameError::SegmentStartsWithDigit);
            }

            for character in segment.chars() {
                if !(character.is_ascii_alphanumeric() || character == '_') {
                    return Err(PackageNameError::InvalidCharacter(character));
                }
            }
        }

        if segment_count < 2 {
            return Err(PackageNameError::MissingSegment);
        }

        Ok(Self(raw.to_owned()))
    }

    /// Returns the package name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PackageName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// Reasons a package name can be rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageNameError {
    /// The package name is empty.
    Empty,
    /// The package name exceeds the maximum length.
    TooLong,
    /// The package name has fewer than two segments.
    MissingSegment,
    /// The package name contains an empty segment.
    EmptySegment,
    /// A segment exceeds the maximum length.
    SegmentTooLong,
    /// A segment starts with a digit.
    SegmentStartsWithDigit,
    /// The package name contains an unsupported character.
    InvalidCharacter(char),
}

impl fmt::Display for PackageNameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("package name is empty"),
            Self::TooLong => formatter.write_str("package name is too long"),
            Self::MissingSegment => formatter.write_str("package name needs at least two segments"),
            Self::EmptySegment => formatter.write_str("package name contains an empty segment"),
            Self::SegmentTooLong => formatter.write_str("package name segment is too long"),
            Self::SegmentStartsWithDigit => {
                formatter.write_str("package name segment starts with a digit")
            }
            Self::InvalidCharacter(character) => {
                write!(
                    formatter,
                    "package name contains invalid character {character:?}"
                )
            }
        }
    }
}

impl std::error::Error for PackageNameError {}

/// An application installed on an Android device.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledApp {
    /// Validated package name.
    pub package: PackageName,
    /// Human-readable application label, when it could be read from the APK.
    pub label: Option<String>,
    /// Remote path of the base APK on the device.
    pub apk_path: Option<String>,
    /// Whether the package is part of the system image.
    pub is_system: bool,
    /// Whether the package is disabled for the current user.
    pub is_disabled: bool,
}

impl InstalledApp {
    /// Returns the application label when available, otherwise the package name.
    pub fn display_name(&self) -> &str {
        match &self.label {
            Some(label) if !label.trim().is_empty() => label.trim(),
            _ => self.package.as_str(),
        }
    }
}

/// Sorts applications by display name, falling back to the package name.
pub fn sort_apps(apps: &mut [InstalledApp]) {
    apps.sort_by_key(|app| (app.display_name().to_lowercase(), app.package.clone()));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(package: &str, label: Option<&str>) -> InstalledApp {
        InstalledApp {
            package: PackageName::parse(package).expect("valid package name"),
            label: label.map(str::to_owned),
            apk_path: None,
            is_system: false,
            is_disabled: false,
        }
    }

    #[test]
    fn parses_valid_package_names() {
        for name in [
            "com.example",
            "com.example.app",
            "com.example_2.app_name",
            "a.b",
        ] {
            assert!(PackageName::parse(name).is_ok(), "{name} should be valid");
        }
    }

    #[test]
    fn rejects_invalid_package_names() {
        let cases = [
            ("", PackageNameError::Empty),
            (".", PackageNameError::EmptySegment),
            ("single", PackageNameError::MissingSegment),
            ("com..app", PackageNameError::EmptySegment),
            ("com.1app", PackageNameError::SegmentStartsWithDigit),
            ("com.exa-mple", PackageNameError::InvalidCharacter('-')),
            ("com.exa mple", PackageNameError::InvalidCharacter(' ')),
            ("com.exa/mple", PackageNameError::InvalidCharacter('/')),
        ];

        for (raw, expected) in cases {
            assert_eq!(PackageName::parse(raw), Err(expected), "{raw:?}");
        }
    }

    #[test]
    fn rejects_overlong_names() {
        let segment = "a".repeat(MAX_SEGMENT_LENGTH + 1);
        let raw = format!("com.{segment}");
        assert_eq!(
            PackageName::parse(&raw),
            Err(PackageNameError::SegmentTooLong)
        );

        let raw = "a".repeat(MAX_PACKAGE_NAME_LENGTH + 1);
        assert_eq!(PackageName::parse(&raw), Err(PackageNameError::TooLong));
    }

    #[test]
    fn display_name_prefers_non_empty_label() {
        assert_eq!(
            app("com.example", Some("Example")).display_name(),
            "Example"
        );
        assert_eq!(
            app("com.example", Some("   ")).display_name(),
            "com.example"
        );
        assert_eq!(app("com.example", None).display_name(), "com.example");
    }

    #[test]
    fn sorts_by_label_then_package() {
        let mut apps = vec![
            app("com.zeta", None),
            app("com.alpha", Some("beta")),
            app("com.beta", Some("Alpha")),
        ];

        sort_apps(&mut apps);

        let names: Vec<_> = apps.iter().map(InstalledApp::display_name).collect();
        assert_eq!(names, ["Alpha", "beta", "com.zeta"]);
    }
}
