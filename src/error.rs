//! Top-level application error type.
//!
//! Each subsystem (`config`, `invoice`, `pdf`) owns its own error enum.
//! [`AppError`] composes them via `#[from]` so that `?` propagates cleanly
//! from any subsystem up to `main`.
//!
//! The only top-level variant that doesn't belong to a subsystem is
//! [`AppError::SetupCancelled`], which is signalled by the interactive prompt
//! layer when the user aborts (Esc / Ctrl-C). It's carried at the top level
//! because it's a control-flow signal more than a domain error.

use thiserror::Error;

use crate::config::ConfigError;
use crate::invoice::InvoiceError;
use crate::pdf::PdfError;

/// Application-level errors for the invoice generator.
#[derive(Debug, Error)]
pub enum AppError {
    /// An error originating from the config subsystem.
    #[error("config error: {0}")]
    Config(#[from] ConfigError),

    /// An error originating from the invoice subsystem.
    #[error("invoice error: {0}")]
    Invoice(#[from] InvoiceError),

    /// An error originating from the PDF subsystem.
    #[error("pdf error: {0}")]
    Pdf(#[from] PdfError),

    /// User cancelled the setup wizard (Escape / Ctrl-C).
    #[error("Setup cancelled by user.")]
    SetupCancelled,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Smoke test for issue C2: a PDF-write IO failure must surface as
    /// `pdf error: write: …`, not as a config-flavoured error. This pins the
    /// behaviour change introduced when the `From<io::Error> for AppError`
    /// blanket was removed and `PdfError::Write` absorbed the `current_dir()`
    /// failure that used to be miscategorised as `ConfigIo`.
    #[test]
    fn pdf_write_io_error_renders_with_pdf_layer_prefix() {
        // Arrange — the kind of io::Error main.rs:41 would observe.
        let io = std::io::Error::other("permission denied: output.pdf");
        let app_err: AppError = PdfError::Write(io).into();

        // Act
        let msg = format!("{app_err}");

        // Assert
        assert!(
            msg.starts_with("pdf error: write:"),
            "Expected 'pdf error: write:' prefix, got: {msg}"
        );
        assert!(
            !msg.contains("config"),
            "PDF write error must not mention 'config', got: {msg}"
        );
    }

    #[test]
    fn config_parse_error_renders_with_config_layer_prefix() {
        // Arrange — a synthetic YAML parse error.
        let yaml_err: serde_yaml::Error =
            serde_yaml::from_str::<crate::config::types::Config>("not: valid: yaml: [")
                .unwrap_err();
        let app_err: AppError = ConfigError::Parse(yaml_err).into();

        // Act
        let msg = format!("{app_err}");

        // Assert
        assert!(
            msg.starts_with("config error: parse:"),
            "Expected 'config error: parse:' prefix, got: {msg}"
        );
    }
}
