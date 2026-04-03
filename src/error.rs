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
}
