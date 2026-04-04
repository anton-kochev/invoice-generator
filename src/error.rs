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
}
