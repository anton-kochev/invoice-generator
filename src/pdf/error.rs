//! Errors produced by the `pdf` subsystem.
//!
//! These cover Typst compilation, PDF export, and IO failures while writing
//! PDF output. The `Write` variant intentionally absorbs the `current_dir()`
//! failure that previously masqueraded as a config-IO error in `main.rs`.
//!
//! Composes into [`crate::error::AppError`] via `#[from]`.

use thiserror::Error;

/// Errors produced by the PDF subsystem.
#[derive(Debug, Error)]
pub enum PdfError {
    /// PDF compilation failed (typst compilation error).
    #[error("compile: {0}")]
    Compile(String),

    /// PDF export failed.
    #[error("export: {0}")]
    Export(String),

    /// IO failure while resolving the output directory or writing the PDF
    /// to disk.
    ///
    /// The blanket `From<io::Error>` is scoped to this enum so that `?` on an
    /// `io::Error` inside PDF-output code lands here, not in unrelated
    /// subsystems.
    #[error("write: {0}")]
    Write(#[from] std::io::Error),

    /// Failure operating on the template manifest or local templates dir
    /// (resolving cache paths, reading the on-disk manifest, writing built-in
    /// templates to disk, etc.).
    #[error("manifest: {0}")]
    Manifest(String),

    /// Failure fetching a template or manifest from the upstream repository
    /// (network error, non-2xx response, malformed body).
    #[error("remote: {0}")]
    Remote(String),

    /// User asked for a template that is neither installed locally nor known
    /// to the manifest cache.
    #[error("template '{name}' not found locally. Available: {}", available.join(", "))]
    TemplateNotFound {
        name: String,
        available: Vec<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_error_displays_message() {
        // Arrange
        let err = PdfError::Compile("syntax error at line 5".into());

        // Act
        let msg = format!("{err}");

        // Assert
        assert!(msg.contains("compile"), "Expected 'compile' in: {msg}");
        assert!(msg.contains("syntax error"), "Expected payload in: {msg}");
    }

    #[test]
    fn test_export_error_displays_message() {
        // Arrange
        let err = PdfError::Export("export failed".into());

        // Act
        let msg = format!("{err}");

        // Assert
        assert!(msg.contains("export"), "Expected 'export' in: {msg}");
    }

    #[test]
    fn test_write_error_from_io_error() {
        // Arrange — verify From<io::Error> is wired up
        let io = std::io::Error::other("disk full");

        // Act
        let err: PdfError = io.into();

        // Assert
        let msg = format!("{err}");
        assert!(
            msg.starts_with("write:"),
            "Expected 'write:' prefix in: {msg}"
        );
    }
}
