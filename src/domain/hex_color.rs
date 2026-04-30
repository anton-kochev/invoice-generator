//! `#RRGGBB` hex color newtype with strict format validation.
//!
//! Rejects the short `#RGB` form and any non-hex characters. Stored
//! normalized to lowercase so equality is case-insensitive.

use std::fmt;

use thiserror::Error;

/// Validation errors for [`HexColor`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum HexColorError {
    /// Input did not match `^#[0-9a-fA-F]{6}$`.
    #[error("invalid hex color: \"{0}\" (expected #RRGGBB with 6 hex digits)")]
    InvalidFormat(String),
}

/// A 7-character `#RRGGBB` color literal, normalized to lowercase.
///
/// Construction validates the input; the only way to produce a `HexColor` is
/// through [`HexColor::try_new`] (or its `Deserialize` impl, which delegates to
/// the same checker).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(transparent)]
pub struct HexColor(String);

impl HexColor {
    /// Parse `s` as a `#RRGGBB` color. Rejects `#RGB`, missing leading `#`, and
    /// non-hex digits. The stored form is lowercase.
    pub fn try_new(s: impl Into<String>) -> Result<Self, HexColorError> {
        let s = s.into();
        if !is_valid_hex_color(&s) {
            return Err(HexColorError::InvalidFormat(s));
        }
        Ok(Self(s.to_ascii_lowercase()))
    }

    /// Borrow the canonical lowercase string (e.g. `"#aabbcc"`).
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for HexColor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for HexColor {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<'de> serde::Deserialize<'de> for HexColor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::try_new(s).map_err(serde::de::Error::custom)
    }
}

fn is_valid_hex_color(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_try_new_accepts_lowercase_six_digit() {
        // Arrange
        let input = "#aabbcc";

        // Act
        let color = HexColor::try_new(input).unwrap();

        // Assert
        assert_eq!(color.as_str(), "#aabbcc");
    }

    #[test]
    fn test_try_new_accepts_uppercase_and_normalizes() {
        // Arrange
        let input = "#AABBCC";

        // Act
        let color = HexColor::try_new(input).unwrap();

        // Assert
        assert_eq!(color.as_str(), "#aabbcc");
    }

    #[test]
    fn test_try_new_accepts_mixed_case_and_normalizes() {
        // Arrange
        let input = "#aAbBcC";

        // Act
        let color = HexColor::try_new(input).unwrap();

        // Assert
        assert_eq!(color.as_str(), "#aabbcc");
    }

    #[test]
    fn test_try_new_rejects_short_form() {
        // Arrange
        let input = "#abc";

        // Act
        let result = HexColor::try_new(input);

        // Assert
        assert!(matches!(result, Err(HexColorError::InvalidFormat(_))));
    }

    #[test]
    fn test_try_new_rejects_eight_digit_form() {
        // Arrange
        let input = "#aabbccdd";

        // Act
        let result = HexColor::try_new(input);

        // Assert
        assert!(matches!(result, Err(HexColorError::InvalidFormat(_))));
    }

    #[test]
    fn test_try_new_rejects_missing_hash() {
        // Arrange
        let input = "aabbcc";

        // Act
        let result = HexColor::try_new(input);

        // Assert
        assert!(matches!(result, Err(HexColorError::InvalidFormat(_))));
    }

    #[test]
    fn test_try_new_rejects_non_hex_digits() {
        // Arrange
        let input = "#gghhii";

        // Act
        let result = HexColor::try_new(input);

        // Assert
        assert!(matches!(result, Err(HexColorError::InvalidFormat(_))));
    }

    #[test]
    fn test_try_new_rejects_empty_string() {
        // Arrange
        let input = "";

        // Act
        let result = HexColor::try_new(input);

        // Assert
        assert!(matches!(result, Err(HexColorError::InvalidFormat(_))));
    }

    #[test]
    fn test_try_new_rejects_named_color() {
        // Arrange
        let input = "red";

        // Act
        let result = HexColor::try_new(input);

        // Assert
        assert!(matches!(result, Err(HexColorError::InvalidFormat(_))));
    }

    #[test]
    fn test_equality_is_case_insensitive_after_normalization() {
        // Arrange
        let lower = HexColor::try_new("#aabbcc").unwrap();
        let upper = HexColor::try_new("#AABBCC").unwrap();

        // Act & Assert
        assert_eq!(lower, upper);
    }

    #[test]
    fn test_display_uses_lowercase_form() {
        // Arrange
        let color = HexColor::try_new("#FF5500").unwrap();

        // Act
        let s = format!("{color}");

        // Assert
        assert_eq!(s, "#ff5500");
    }

    #[test]
    fn test_serde_yaml_round_trip() {
        // Arrange
        let color = HexColor::try_new("#3aa9ff").unwrap();

        // Act
        let yaml = serde_yaml::to_string(&color).unwrap();
        let loaded: HexColor = serde_yaml::from_str(&yaml).unwrap();

        // Assert
        assert_eq!(loaded, color);
    }

    #[test]
    fn test_serde_yaml_deserializes_uppercase_to_normalized() {
        // Arrange
        let yaml = "\"#AABBCC\"\n";

        // Act
        let color: HexColor = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert_eq!(color.as_str(), "#aabbcc");
    }

    #[test]
    fn test_serde_yaml_rejects_short_form() {
        // Arrange
        let yaml = "\"#abc\"\n";

        // Act
        let result: Result<HexColor, _> = serde_yaml::from_str(yaml);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_serde_json_round_trip() {
        // Arrange
        let color = HexColor::try_new("#2c3e50").unwrap();

        // Act
        let json = serde_json::to_string(&color).unwrap();
        let loaded: HexColor = serde_json::from_str(&json).unwrap();

        // Assert
        assert_eq!(loaded, color);
        assert_eq!(json, "\"#2c3e50\"");
    }

    #[test]
    fn test_serde_json_rejects_invalid_format() {
        // Arrange
        let json = "\"red\"";

        // Act
        let result: Result<HexColor, _> = serde_json::from_str(json);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_as_ref_str_returns_canonical() {
        // Arrange
        let color = HexColor::try_new("#FF0000").unwrap();

        // Act
        let s: &str = color.as_ref();

        // Assert
        assert_eq!(s, "#ff0000");
    }

    #[test]
    fn test_error_message_contains_invalid_input() {
        // Arrange
        let bad = "#zzz";

        // Act
        let err = HexColor::try_new(bad).unwrap_err();
        let msg = err.to_string();

        // Assert
        assert!(msg.contains("#zzz"), "Expected '#zzz' in: {msg}");
    }
}
