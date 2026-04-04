use thiserror::Error;

/// Application-level errors for the invoice generator.
#[derive(Debug, Error)]
pub enum AppError {
    /// YAML config file could not be parsed.
    #[error("Failed to parse config: {0}")]
    ConfigParse(#[from] serde_yaml::Error),

    /// IO error while reading the config file.
    #[error("Failed to read config: {0}")]
    ConfigIo(#[from] std::io::Error),

    /// User cancelled the setup wizard (Escape / Ctrl-C).
    #[error("Setup cancelled by user.")]
    SetupCancelled,

    /// Invalid date during invoice computation.
    #[error("Invalid date: {0}")]
    InvalidDate(String),

    /// PDF compilation failed (typst compilation error).
    #[error("PDF compilation failed: {0}")]
    PdfCompile(String),

    /// PDF export failed.
    #[error("PDF export failed: {0}")]
    PdfExport(String),

    /// Requested preset key does not exist.
    #[error("Unknown preset: \"{0}\"")]
    PresetNotFound(String),

    /// Cannot delete the last remaining preset.
    #[error("Cannot delete — at least one preset must exist.")]
    LastPreset,

    /// Config file is required but not found (for non-interactive subcommands).
    #[error("No config file found. Run `invoice` first to set up.")]
    ConfigNotFound,

    /// Invalid days value in `--days` or `--items` JSON.
    #[error("Invalid days value: {0} (must be > 0)")]
    InvalidDays(String),

    /// Failed to parse `--items` JSON.
    #[error("Failed to parse --items JSON: {0}")]
    ItemsParse(String),

    /// default_recipient references a key not found in recipients list.
    #[error("Default recipient key \"{0}\" not found in recipients list")]
    InvalidDefaultRecipient(String),

    /// default_recipient is missing but recipients are defined.
    #[error("default_recipient is required when recipients are defined")]
    MissingDefaultRecipient,

    /// Two recipients share the same key.
    #[error("Duplicate recipient key: \"{0}\"")]
    DuplicateRecipientKey(String),

    /// Requested recipient key does not exist.
    #[error("Unknown recipient: \"{key}\". Available: {}", available.join(", "))]
    RecipientNotFound {
        key: String,
        available: Vec<String>,
    },

    /// Cannot delete the last remaining recipient.
    #[error("Cannot delete — at least one recipient must exist.")]
    LastRecipient,

    /// Line items have conflicting currencies.
    #[error("Mixed currencies in line items: {0}")]
    MixedCurrency(String),

    /// Invalid tax rate (must be >= 0).
    #[error("Invalid tax rate: {0} (must be >= 0)")]
    InvalidTaxRate(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_not_found_error_displays_key() {
        // Arrange
        let err = AppError::PresetNotFound("xyz".into());

        // Act
        let msg = format!("{err}");

        // Assert
        assert!(msg.contains("xyz"), "Expected 'xyz' in: {msg}");
    }

    #[test]
    fn test_last_preset_error_displays_message() {
        // Arrange
        let err = AppError::LastPreset;

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
        let err = AppError::InvalidDefaultRecipient("bogus".into());

        // Act
        let msg = format!("{err}");

        // Assert
        assert!(msg.contains("bogus"), "Expected 'bogus' in: {msg}");
    }

    #[test]
    fn test_duplicate_recipient_key_displays_key() {
        // Arrange
        let err = AppError::DuplicateRecipientKey("acme".into());

        // Act
        let msg = format!("{err}");

        // Assert
        assert!(msg.contains("acme"), "Expected 'acme' in: {msg}");
    }

    #[test]
    fn test_recipient_not_found_displays_key() {
        // Arrange
        let err = AppError::RecipientNotFound {
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
    fn test_mixed_currency_error_displays_currencies() {
        // Arrange
        let err = AppError::MixedCurrency("EUR, USD".into());

        // Act
        let msg = format!("{err}");

        // Assert
        assert!(msg.contains("EUR"), "Expected 'EUR' in: {msg}");
        assert!(msg.contains("USD"), "Expected 'USD' in: {msg}");
    }

    #[test]
    fn test_invalid_tax_rate_displays_value() {
        // Arrange
        let err = AppError::InvalidTaxRate("-5.0".into());

        // Act
        let msg = format!("{err}");

        // Assert
        assert!(msg.contains("-5.0"), "Expected '-5.0' in: {msg}");
        assert!(msg.contains(">= 0"), "Expected '>= 0' in: {msg}");
    }

    #[test]
    fn test_missing_default_recipient_displays_message() {
        // Arrange
        let err = AppError::MissingDefaultRecipient;
        // Act
        let msg = format!("{err}");
        // Assert
        assert!(msg.contains("required"), "Expected 'required' in: {msg}");
    }

    #[test]
    fn test_last_recipient_displays_message() {
        // Arrange
        let err = AppError::LastRecipient;

        // Act
        let msg = format!("{err}");

        // Assert
        assert!(
            msg.contains("at least one recipient"),
            "Expected 'at least one recipient' in: {msg}"
        );
    }
}
