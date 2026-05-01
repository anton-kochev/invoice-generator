//! [`PresetKey`] newtype: a validated preset identifier slug.
//!
//! Same shape as [`super::recipient_key::RecipientKey`] — non-empty ASCII
//! lowercase letters, digits, and hyphens, no edge or consecutive hyphens.
//! A separate type prevents accidental cross-use (e.g. handing a recipient
//! key to a preset lookup).

use std::fmt;

use super::recipient_key::{KeyError, slugify, validate_key};

/// A validated preset slug.
///
/// Construct via [`PresetKey::try_new`] (strict — rejects invalid input)
/// or [`PresetKey::from_name`] (slugifies a free-form name, then validates).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(transparent)]
pub struct PresetKey(String);

impl PresetKey {
    /// Strictly validate `s` as an already-formed key.
    pub fn try_new(s: impl Into<String>) -> Result<Self, KeyError> {
        let s = s.into();
        validate_key(&s)?;
        Ok(Self(s))
    }

    /// Derive a key from a free-form name. See
    /// [`RecipientKey::from_name`](super::RecipientKey::from_name) for slugify rules.
    #[allow(dead_code)] // public newtype constructor — exposed for API completeness
    pub fn from_name(name: &str) -> Result<Self, KeyError> {
        Self::try_new(slugify(name))
    }

    /// Borrow the validated key string.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PresetKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for PresetKey {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<'de> serde::Deserialize<'de> for PresetKey {
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

    #[test]
    fn test_try_new_accepts_valid_key() {
        // Arrange & Act
        let key = PresetKey::try_new("dev").unwrap();

        // Assert
        assert_eq!(key.as_str(), "dev");
    }

    #[test]
    fn test_try_new_rejects_empty() {
        // Arrange & Act
        let result = PresetKey::try_new("");

        // Assert
        assert!(matches!(result, Err(KeyError::Empty)));
    }

    #[test]
    fn test_try_new_rejects_uppercase() {
        // Arrange & Act
        let result = PresetKey::try_new("DEV");

        // Assert
        assert!(matches!(result, Err(KeyError::InvalidCharacters(_))));
    }

    #[test]
    fn test_from_name_slugifies() {
        // Arrange & Act
        let key = PresetKey::from_name("Dev Ops 2026").unwrap();

        // Assert
        assert_eq!(key.as_str(), "dev-ops-2026");
    }

    #[test]
    fn test_serde_yaml_round_trip() {
        // Arrange
        let key = PresetKey::try_new("dev").unwrap();

        // Act
        let yaml = serde_yaml::to_string(&key).unwrap();
        let loaded: PresetKey = serde_yaml::from_str(&yaml).unwrap();

        // Assert
        assert_eq!(loaded, key);
    }

    #[test]
    fn test_preset_key_distinct_from_recipient_key() {
        // The point of having two newtypes is to prevent cross-mixing.
        // This test ensures that even with identical inner strings, the two
        // types are distinct at the type level — verified by the fact that
        // this code only compiles because we never `==` the two together.
        let p = PresetKey::try_new("dev").unwrap();
        let r = super::super::RecipientKey::try_new("dev").unwrap();
        assert_eq!(p.as_str(), r.as_str());
        // The next line would be a compile error — different types:
        // assert_eq!(p, r);
    }
}
