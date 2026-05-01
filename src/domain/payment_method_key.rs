//! [`PaymentMethodKey`] newtype: a validated payment-method identifier slug.
//!
//! Mirrors [`crate::domain::RecipientKey`]: same slug rules, same surface
//! (`try_new` / `from_name` / `as_str` / `Display` / serde). Validation is
//! shared with the recipient key via the `validate_key` / `slugify` helpers in
//! [`crate::domain::recipient_key`].

use std::fmt;

pub use super::recipient_key::KeyError;
use super::recipient_key::{slugify, validate_key};

/// A validated payment-method slug.
///
/// Construct via [`PaymentMethodKey::try_new`] (strict — rejects invalid input)
/// or [`PaymentMethodKey::from_name`] (slugifies a free-form label, then
/// validates).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(transparent)]
pub struct PaymentMethodKey(String);

impl PaymentMethodKey {
    /// Strictly validate `s` as an already-formed key. Does not transform.
    pub fn try_new(s: impl Into<String>) -> Result<Self, KeyError> {
        let s = s.into();
        validate_key(&s)?;
        Ok(Self(s))
    }

    /// Derive a key from a free-form name by slugifying it (lowercasing,
    /// replacing non-`[a-z0-9]` runs with `-`, trimming edges), then
    /// validate. Non-ASCII characters are dropped — the resulting slug is
    /// pure ASCII.
    pub fn from_name(name: &str) -> Result<Self, KeyError> {
        Self::try_new(slugify(name))
    }

    /// Borrow the validated key string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PaymentMethodKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for PaymentMethodKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<'de> serde::Deserialize<'de> for PaymentMethodKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::try_new(s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── try_new ──

    #[test]
    fn test_try_new_accepts_simple_key() {
        // Arrange & Act
        let key = PaymentMethodKey::try_new("sepa").unwrap();

        // Assert
        assert_eq!(key.as_str(), "sepa");
    }

    #[test]
    fn test_try_new_accepts_hyphenated_key() {
        // Arrange & Act
        let key = PaymentMethodKey::try_new("mono-eur-sepa").unwrap();

        // Assert
        assert_eq!(key.as_str(), "mono-eur-sepa");
    }

    #[test]
    fn test_try_new_accepts_digits() {
        // Arrange & Act
        let key = PaymentMethodKey::try_new("acc-2026").unwrap();

        // Assert
        assert_eq!(key.as_str(), "acc-2026");
    }

    #[test]
    fn test_try_new_rejects_empty() {
        // Arrange & Act
        let result = PaymentMethodKey::try_new("");

        // Assert
        assert!(matches!(result, Err(KeyError::Empty)));
    }

    #[test]
    fn test_try_new_rejects_uppercase() {
        // Arrange & Act
        let result = PaymentMethodKey::try_new("SEPA");

        // Assert
        assert!(matches!(result, Err(KeyError::InvalidCharacters(_))));
    }

    #[test]
    fn test_try_new_rejects_leading_hyphen() {
        // Arrange & Act
        let result = PaymentMethodKey::try_new("-sepa");

        // Assert
        assert!(matches!(result, Err(KeyError::EdgeHyphen(_))));
    }

    #[test]
    fn test_try_new_rejects_trailing_hyphen() {
        // Arrange & Act
        let result = PaymentMethodKey::try_new("sepa-");

        // Assert
        assert!(matches!(result, Err(KeyError::EdgeHyphen(_))));
    }

    #[test]
    fn test_try_new_rejects_consecutive_hyphens() {
        // Arrange & Act
        let result = PaymentMethodKey::try_new("sepa--transfer");

        // Assert
        assert!(matches!(result, Err(KeyError::ConsecutiveHyphens(_))));
    }

    #[test]
    fn test_try_new_rejects_underscore() {
        // Arrange & Act
        let result = PaymentMethodKey::try_new("sepa_transfer");

        // Assert
        assert!(matches!(result, Err(KeyError::InvalidCharacters(_))));
    }

    #[test]
    fn test_try_new_rejects_non_ascii_letter() {
        // Arrange & Act — `ü` is not ASCII.
        let result = PaymentMethodKey::try_new("über");

        // Assert
        assert!(matches!(result, Err(KeyError::InvalidCharacters(_))));
    }

    #[test]
    fn test_try_new_rejects_whitespace() {
        // Arrange & Act
        let result = PaymentMethodKey::try_new("sepa transfer");

        // Assert
        assert!(matches!(result, Err(KeyError::InvalidCharacters(_))));
    }

    // ── from_name (slugify path) ──

    #[test]
    fn test_from_name_two_words() {
        // Arrange
        let name = "SEPA Transfer";

        // Act
        let key = PaymentMethodKey::from_name(name).unwrap();

        // Assert
        assert_eq!(key.as_str(), "sepa-transfer");
    }

    #[test]
    fn test_from_name_single_word() {
        // Arrange
        let name = "SEPA";

        // Act
        let key = PaymentMethodKey::from_name(name).unwrap();

        // Assert
        assert_eq!(key.as_str(), "sepa");
    }

    #[test]
    fn test_from_name_punctuation_stripped() {
        // Arrange
        let name = "Wire/SWIFT (USD)";

        // Act
        let key = PaymentMethodKey::from_name(name).unwrap();

        // Assert
        assert_eq!(key.as_str(), "wire-swift-usd");
    }

    #[test]
    fn test_from_name_whitespace_only_returns_empty_error() {
        // Arrange
        let name = "   ";

        // Act
        let result = PaymentMethodKey::from_name(name);

        // Assert
        assert!(matches!(result, Err(KeyError::Empty)));
    }

    #[test]
    fn test_from_name_empty_returns_empty_error() {
        // Arrange
        let name = "";

        // Act
        let result = PaymentMethodKey::from_name(name);

        // Assert
        assert!(matches!(result, Err(KeyError::Empty)));
    }

    #[test]
    fn test_from_name_pure_punctuation_returns_empty_error() {
        // Arrange — no alphanumerics → slugify produces empty string.
        let name = "!!!";

        // Act
        let result = PaymentMethodKey::from_name(name);

        // Assert
        assert!(matches!(result, Err(KeyError::Empty)));
    }

    // ── serde ──

    #[test]
    fn test_serde_yaml_round_trip() {
        // Arrange
        let key = PaymentMethodKey::try_new("mono-eur-sepa").unwrap();

        // Act
        let yaml = serde_yaml::to_string(&key).unwrap();
        let loaded: PaymentMethodKey = serde_yaml::from_str(&yaml).unwrap();

        // Assert
        assert_eq!(loaded, key);
    }

    #[test]
    fn test_serde_yaml_rejects_invalid_input() {
        // Arrange
        let yaml = "SEPA\n";

        // Act
        let result: Result<PaymentMethodKey, _> = serde_yaml::from_str(yaml);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_display_matches_as_str() {
        // Arrange
        let key = PaymentMethodKey::try_new("mono-eur-sepa").unwrap();

        // Act & Assert
        assert_eq!(format!("{key}"), "mono-eur-sepa");
    }
}
