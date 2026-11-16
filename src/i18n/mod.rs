//! Translation loading and message formatting.
//!
//! English is the fallback locale. Missing keys return the key itself so that
//! omissions are visible during development.

use std::collections::HashMap;

const ENGLISH_MESSAGES: &str = include_str!("../../locales/en.json");

/// Loaded translation messages for one locale.
#[derive(Debug, Clone)]
pub struct Translations {
    messages: HashMap<String, String>,
}

impl Translations {
    /// Loads the English translations bundled with the application.
    pub fn english() -> Self {
        Self::from_json(ENGLISH_MESSAGES).unwrap_or_else(|_| Self {
            messages: HashMap::new(),
        })
    }

    /// Parses translations from a JSON object mapping keys to messages.
    pub fn from_json(source: &str) -> Result<Self, serde_json::Error> {
        let messages: HashMap<String, String> = serde_json::from_str(source)?;
        Ok(Self { messages })
    }

    /// Returns the message for `key`, or the key itself when missing.
    pub fn get<'a>(&'a self, key: &'a str) -> &'a str {
        self.messages.get(key).map(String::as_str).unwrap_or(key)
    }

    /// Formats a message, replacing `{name}` placeholders with values.
    pub fn format(&self, key: &str, values: &[(&str, &str)]) -> String {
        let mut message = self.get(key).to_owned();

        for (name, value) in values {
            message = message.replace(&format!("{{{name}}}"), value);
        }

        message
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_translations_load() {
        let translations = Translations::english();
        assert_eq!(translations.get("app.title"), "Android Utility");
        assert_ne!(translations.get("apps.uninstall"), "apps.uninstall");
    }

    #[test]
    fn missing_key_returns_key() {
        let translations = Translations::english();
        assert_eq!(translations.get("missing.key"), "missing.key");
    }

    #[test]
    fn format_replaces_placeholders() {
        let translations = Translations::english();
        let text = translations.format(
            "confirm.uninstall_message",
            &[("name", "Example"), ("package", "com.example")],
        );

        assert!(text.contains("Example"));
        assert!(text.contains("com.example"));
        assert!(!text.contains("{name}"));
    }

    #[test]
    fn malformed_json_is_rejected() {
        assert!(Translations::from_json("not valid json").is_err());
        assert!(Translations::from_json(r#"{"key": 1}"#).is_err());
    }
}
