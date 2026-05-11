//! [`SenderKey`] newtype: a validated sender identifier slug.
//!
//! Same validation rules as [`crate::domain::RecipientKey`] — non-empty ASCII
//! lowercase letters, digits, and hyphens, no leading/trailing or consecutive
//! hyphens. The validation helpers are reused from `recipient_key` rather than
//! duplicated; extracting a shared `key_validation` module is deferred.

use std::fmt;

use super::recipient_key::{KeyError, slugify, validate_key};

/// A validated sender slug.
///
/// Construct via [`SenderKey::try_new`] (strict — rejects invalid input)
/// or [`SenderKey::from_name`] (slugifies a free-form name, then validates).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(transparent)]
pub struct SenderKey(String);

impl SenderKey {
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

impl fmt::Display for SenderKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for SenderKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<'de> serde::Deserialize<'de> for SenderKey {
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
        let key = SenderKey::try_new("acme").unwrap();

        // Assert
        assert_eq!(key.as_str(), "acme");
    }

    #[test]
    fn test_try_new_accepts_hyphenated_key() {
        // Arrange & Act
        let key = SenderKey::try_new("acme-sender").unwrap();

        // Assert
        assert_eq!(key.as_str(), "acme-sender");
    }

    #[test]
    fn test_try_new_accepts_digits() {
        // Arrange & Act
        let key = SenderKey::try_new("acme-2026").unwrap();

        // Assert
        assert_eq!(key.as_str(), "acme-2026");
    }

    #[test]
    fn test_try_new_rejects_empty() {
        // Arrange & Act
        let result = SenderKey::try_new("");

        // Assert
        assert!(matches!(result, Err(KeyError::Empty)));
    }

    #[test]
    fn test_try_new_rejects_uppercase() {
        // Arrange & Act
        let result = SenderKey::try_new("Acme");

        // Assert
        assert!(matches!(result, Err(KeyError::InvalidCharacters(_))));
    }

    #[test]
    fn test_try_new_rejects_leading_hyphen() {
        // Arrange & Act
        let result = SenderKey::try_new("-acme");

        // Assert
        assert!(matches!(result, Err(KeyError::EdgeHyphen(_))));
    }

    #[test]
    fn test_try_new_rejects_trailing_hyphen() {
        // Arrange & Act
        let result = SenderKey::try_new("acme-");

        // Assert
        assert!(matches!(result, Err(KeyError::EdgeHyphen(_))));
    }

    #[test]
    fn test_try_new_rejects_consecutive_hyphens() {
        // Arrange & Act
        let result = SenderKey::try_new("acme--sender");

        // Assert
        assert!(matches!(result, Err(KeyError::ConsecutiveHyphens(_))));
    }

    #[test]
    fn test_try_new_rejects_underscore() {
        // Arrange & Act
        let result = SenderKey::try_new("acme_sender");

        // Assert
        assert!(matches!(result, Err(KeyError::InvalidCharacters(_))));
    }

    #[test]
    fn test_try_new_rejects_non_ascii_letter() {
        // Arrange & Act — `ü` is not ASCII.
        let result = SenderKey::try_new("müller");

        // Assert
        assert!(matches!(result, Err(KeyError::InvalidCharacters(_))));
    }

    // ── from_name ──

    #[test]
    fn test_from_name_two_words() {
        // Arrange
        let name = "Acme Sender";

        // Act
        let key = SenderKey::from_name(name).unwrap();

        // Assert
        assert_eq!(key.as_str(), "acme-sender");
    }

    #[test]
    fn test_from_name_single_word() {
        // Arrange
        let name = "Alice";

        // Act
        let key = SenderKey::from_name(name).unwrap();

        // Assert
        assert_eq!(key.as_str(), "alice");
    }

    #[test]
    fn test_from_name_punctuation_stripped() {
        // Arrange
        let name = "Foo & Bar, Inc.";

        // Act
        let key = SenderKey::from_name(name).unwrap();

        // Assert
        assert_eq!(key.as_str(), "foo-bar-inc");
    }

    #[test]
    fn test_from_name_whitespace_only_returns_empty_error() {
        // Arrange
        let name = "   ";

        // Act
        let result = SenderKey::from_name(name);

        // Assert
        assert!(matches!(result, Err(KeyError::Empty)));
    }

    #[test]
    fn test_from_name_empty_returns_empty_error() {
        // Arrange
        let name = "";

        // Act
        let result = SenderKey::from_name(name);

        // Assert
        assert!(matches!(result, Err(KeyError::Empty)));
    }

    #[test]
    fn test_from_name_non_ascii_treated_as_separator() {
        // Arrange — non-ASCII letters split tokens; `Müller-Schmidt GmbH`
        // yields tokens `m`, `ller`, `schmidt`, `gmbh`. Slugs are pure ASCII.
        let name = "Müller-Schmidt GmbH";

        // Act
        let key = SenderKey::from_name(name).unwrap();

        // Assert
        assert_eq!(key.as_str(), "m-ller-schmidt-gmbh");
    }

    // ── serde ──

    #[test]
    fn test_serde_yaml_round_trip() {
        // Arrange
        let key = SenderKey::try_new("acme-sender").unwrap();

        // Act
        let yaml = serde_yaml::to_string(&key).unwrap();
        let loaded: SenderKey = serde_yaml::from_str(&yaml).unwrap();

        // Assert
        assert_eq!(loaded, key);
    }

    #[test]
    fn test_serde_yaml_rejects_invalid_input() {
        // Arrange
        let yaml = "ACME\n";

        // Act
        let result: Result<SenderKey, _> = serde_yaml::from_str(yaml);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_display_matches_as_str() {
        // Arrange
        let key = SenderKey::try_new("foo-bar").unwrap();

        // Act & Assert
        assert_eq!(format!("{key}"), "foo-bar");
    }
}
