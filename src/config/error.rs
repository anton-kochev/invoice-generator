//! Errors produced by the `config` subsystem.
//!
//! These cover loading, parsing, validating, and writing the YAML config file,
//! as well as recipient/preset bookkeeping errors that surface during config
//! mutations.
//!
//! Composes into [`crate::error::AppError`] via `#[from]`.

use thiserror::Error;

/// Errors produced by the config subsystem.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// YAML config file could not be parsed.
    #[error("parse: {0}")]
    Parse(#[from] serde_yaml::Error),

    /// IO error while reading or writing the config file.
    ///
    /// The blanket `From<io::Error>` is scoped to this enum so that `?` on an
    /// `io::Error` inside config code lands here, not in unrelated subsystems.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    /// Failure resolving or preparing the config file path.
    #[error("path: {0}")]
    Path(String),

    /// Config file is required but not found (for non-interactive subcommands).
    #[error("no config file found. Run `invoice` first to set up.")]
    NotFound,

    /// `default_recipient` references a key not found in recipients list,
    /// or the key value itself failed validation.
    #[error("default recipient \"{0}\" not found in recipients list")]
    InvalidDefaultRecipient(String),

    /// `default_recipient` is missing but recipients are defined.
    #[error("default_recipient is required when recipients are defined")]
    MissingDefaultRecipient,

    /// Two recipients share the same key.
    #[error("duplicate recipient key: \"{0}\"")]
    DuplicateRecipientKey(String),

    /// Cannot delete the last remaining recipient.
    #[error("cannot delete \u{2014} at least one recipient must exist.")]
    LastRecipient,

    /// Cannot delete the last remaining preset.
    #[error("cannot delete \u{2014} at least one preset must exist.")]
    LastPreset,

    /// Requested recipient key does not exist.
    #[error("unknown recipient: \"{key}\". Available: {}", available.join(", "))]
    RecipientNotFound { key: String, available: Vec<String> },

    /// Requested preset key does not exist.
    #[error("unknown preset: \"{0}\"")]
    PresetNotFound(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_not_found_error_displays_key() {
        // Arrange
        let err = ConfigError::PresetNotFound("xyz".into());

        // Act
        let msg = format!("{err}");

        // Assert
        assert!(msg.contains("xyz"), "Expected 'xyz' in: {msg}");
    }

    #[test]
    fn test_last_preset_error_displays_message() {
        // Arrange
        let err = ConfigError::LastPreset;

        // Act
        let msg = format!("{err}");

        // Assert
        assert!(
            msg.contains("at least one preset"),
            "Expected 'at least one preset' in: {msg}"
        );
    }

    #[test]
    fn test_invalid_default_recipient_displays_key() {
        // Arrange
        let err = ConfigError::InvalidDefaultRecipient("bogus".into());

        // Act
        let msg = format!("{err}");

        // Assert
        assert!(msg.contains("bogus"), "Expected 'bogus' in: {msg}");
    }

    #[test]
    fn test_duplicate_recipient_key_displays_key() {
        // Arrange
        let err = ConfigError::DuplicateRecipientKey("acme".into());

        // Act
        let msg = format!("{err}");

        // Assert
        assert!(msg.contains("acme"), "Expected 'acme' in: {msg}");
    }

    #[test]
    fn test_recipient_not_found_displays_key() {
        // Arrange
        let err = ConfigError::RecipientNotFound {
            key: "xyz".into(),
            available: vec!["acme".into(), "globex".into()],
        };

        // Act
        let msg = format!("{err}");

        // Assert
        assert!(msg.contains("xyz"), "Expected 'xyz' in: {msg}");
        assert!(msg.contains("acme"), "Expected 'acme' in: {msg}");
        assert!(msg.contains("globex"), "Expected 'globex' in: {msg}");
    }

    #[test]
    fn test_missing_default_recipient_displays_message() {
        // Arrange
        let err = ConfigError::MissingDefaultRecipient;
        // Act
        let msg = format!("{err}");
        // Assert
        assert!(msg.contains("required"), "Expected 'required' in: {msg}");
    }

    #[test]
    fn test_last_recipient_displays_message() {
        // Arrange
        let err = ConfigError::LastRecipient;

        // Act
        let msg = format!("{err}");

        // Assert
        assert!(
            msg.contains("at least one recipient"),
            "Expected 'at least one recipient' in: {msg}"
        );
    }
}
