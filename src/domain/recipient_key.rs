//! [`RecipientKey`] newtype: a validated recipient identifier slug.
//!
//! Rule: non-empty ASCII lowercase letters, digits, and hyphens — no leading
//! or trailing hyphen, no consecutive hyphens. Validation happens at
//! construction time so the rest of the program can compare keys directly
//! without re-parsing.

use std::fmt;

use thiserror::Error;

/// Validation errors for [`RecipientKey`] (and its sibling newtypes).
#[derive(Debug, Error, PartialEq, Eq)]
pub enum KeyError {
    /// Empty input or pure-whitespace input that slugifies to nothing.
    #[error("key must not be empty")]
    Empty,

    /// Input contains characters outside `[a-z0-9-]`.
    #[error("key \"{0}\" contains invalid characters (only a-z, 0-9, and '-' allowed)")]
    InvalidCharacters(String),

    /// Leading or trailing hyphen.
    #[error("key \"{0}\" must not start or end with '-'")]
    EdgeHyphen(String),

    /// Two or more consecutive hyphens.
    #[error("key \"{0}\" must not contain consecutive hyphens")]
    ConsecutiveHyphens(String),
}

/// A validated recipient slug.
///
/// Construct via [`RecipientKey::try_new`] (strict — rejects invalid input)
/// or [`RecipientKey::from_name`] (slugifies a free-form name, then validates).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(transparent)]
pub struct RecipientKey(String);

impl RecipientKey {
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

impl fmt::Display for RecipientKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for RecipientKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<'de> serde::Deserialize<'de> for RecipientKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::try_new(s).map_err(serde::de::Error::custom)
    }
}

/// Validate `s` as a [`RecipientKey`] (and any sibling key newtype).
pub(crate) fn validate_key(s: &str) -> Result<(), KeyError> {
    if s.is_empty() {
        return Err(KeyError::Empty);
    }
    if !s
        .bytes()
        .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(KeyError::InvalidCharacters(s.to_string()));
    }
    if s.starts_with('-') || s.ends_with('-') {
        return Err(KeyError::EdgeHyphen(s.to_string()));
    }
    if s.contains("--") {
        return Err(KeyError::ConsecutiveHyphens(s.to_string()));
    }
    Ok(())
}

/// Slugify a free-form name into the key shape. Non-ASCII characters and any
/// runs of non-`[a-z0-9]` (including hyphens) are treated as separators;
/// resulting tokens are joined by a single `-`. Result is lowercased ASCII.
pub(crate) fn slugify(name: &str) -> String {
    name.chars()
        .map(|c| c.to_ascii_lowercase())
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── try_new ──

    #[test]
    fn test_try_new_accepts_simple_key() {
        // Arrange & Act
        let key = RecipientKey::try_new("acme").unwrap();

        // Assert
        assert_eq!(key.as_str(), "acme");
    }

    #[test]
    fn test_try_new_accepts_hyphenated_key() {
        // Arrange & Act
        let key = RecipientKey::try_new("acme-corp").unwrap();

        // Assert
        assert_eq!(key.as_str(), "acme-corp");
    }

    #[test]
    fn test_try_new_accepts_digits() {
        // Arrange & Act
        let key = RecipientKey::try_new("acme-2026").unwrap();

        // Assert
        assert_eq!(key.as_str(), "acme-2026");
    }

    #[test]
    fn test_try_new_rejects_empty() {
        // Arrange & Act
        let result = RecipientKey::try_new("");

        // Assert
        assert!(matches!(result, Err(KeyError::Empty)));
    }

    #[test]
    fn test_try_new_rejects_uppercase() {
        // Arrange & Act
        let result = RecipientKey::try_new("Acme");

        // Assert
        assert!(matches!(result, Err(KeyError::InvalidCharacters(_))));
    }

    #[test]
    fn test_try_new_rejects_leading_hyphen() {
        // Arrange & Act
        let result = RecipientKey::try_new("-acme");

        // Assert
        assert!(matches!(result, Err(KeyError::EdgeHyphen(_))));
    }

    #[test]
    fn test_try_new_rejects_trailing_hyphen() {
        // Arrange & Act
        let result = RecipientKey::try_new("acme-");

        // Assert
        assert!(matches!(result, Err(KeyError::EdgeHyphen(_))));
    }

    #[test]
    fn test_try_new_rejects_consecutive_hyphens() {
        // Arrange & Act
        let result = RecipientKey::try_new("acme--corp");

        // Assert
        assert!(matches!(result, Err(KeyError::ConsecutiveHyphens(_))));
    }

    #[test]
    fn test_try_new_rejects_underscore() {
        // Arrange & Act
        let result = RecipientKey::try_new("acme_corp");

        // Assert
        assert!(matches!(result, Err(KeyError::InvalidCharacters(_))));
    }

    #[test]
    fn test_try_new_rejects_non_ascii_letter() {
        // Arrange & Act — `ü` is not ASCII.
        let result = RecipientKey::try_new("müller");

        // Assert
        assert!(matches!(result, Err(KeyError::InvalidCharacters(_))));
    }

    // ── from_name (lifts the existing derive_recipient_key tests) ──

    #[test]
    fn test_from_name_two_words() {
        // Arrange
        let name = "Acme Corp";

        // Act
        let key = RecipientKey::from_name(name).unwrap();

        // Assert
        assert_eq!(key.as_str(), "acme-corp");
    }

    #[test]
    fn test_from_name_single_word() {
        // Arrange
        let name = "Bob";

        // Act
        let key = RecipientKey::from_name(name).unwrap();

        // Assert
        assert_eq!(key.as_str(), "bob");
    }

    #[test]
    fn test_from_name_punctuation_stripped() {
        // Arrange
        let name = "Foo & Bar, Inc.";

        // Act
        let key = RecipientKey::from_name(name).unwrap();

        // Assert
        assert_eq!(key.as_str(), "foo-bar-inc");
    }

    #[test]
    fn test_from_name_whitespace_only_returns_empty_error() {
        // Arrange
        let name = "   ";

        // Act
        let result = RecipientKey::from_name(name);

        // Assert
        assert!(matches!(result, Err(KeyError::Empty)));
    }

    #[test]
    fn test_from_name_empty_returns_empty_error() {
        // Arrange
        let name = "";

        // Act
        let result = RecipientKey::from_name(name);

        // Assert
        assert!(matches!(result, Err(KeyError::Empty)));
    }

    #[test]
    fn test_from_name_non_ascii_treated_as_separator() {
        // Arrange — non-ASCII letters split tokens; `Müller-Schmidt GmbH`
        // yields tokens `m`, `ller`, `schmidt`, `gmbh`. (Old behavior was to
        // preserve `ü` via Unicode lowercase; now slugs are pure ASCII.)
        let name = "Müller-Schmidt GmbH";

        // Act
        let key = RecipientKey::from_name(name).unwrap();

        // Assert
        assert_eq!(key.as_str(), "m-ller-schmidt-gmbh");
    }

    // ── serde ──

    #[test]
    fn test_serde_yaml_round_trip() {
        // Arrange
        let key = RecipientKey::try_new("acme-corp").unwrap();

        // Act
        let yaml = serde_yaml::to_string(&key).unwrap();
        let loaded: RecipientKey = serde_yaml::from_str(&yaml).unwrap();

        // Assert
        assert_eq!(loaded, key);
    }

    #[test]
    fn test_serde_yaml_rejects_invalid_input() {
        // Arrange
        let yaml = "ACME\n";

        // Act
        let result: Result<RecipientKey, _> = serde_yaml::from_str(yaml);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_display_matches_as_str() {
        // Arrange
        let key = RecipientKey::try_new("foo-bar").unwrap();

        // Act & Assert
        assert_eq!(format!("{key}"), "foo-bar");
    }
}
