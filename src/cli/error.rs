//! Errors produced by the `cli` subsystem.
//!
//! These cover IO failures while writing CLI output (table rows, success
//! messages, list output) to a `Writer` such as stdout.
//!
//! Composes into [`crate::error::AppError`] via `#[from]`.

use thiserror::Error;

/// Errors produced by the CLI subsystem.
#[derive(Debug, Error)]
pub enum CliError {
    /// IO failure while writing CLI output (table rows, success messages,
    /// list output) to a Writer such as stdout.
    #[error("output write: {0}")]
    OutputWrite(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;

    /// Mirror of the layer-prefix smoke tests in `src/error.rs`: a stdout
    /// write failure must surface as `cli error: output write: …`, not as a
    /// PDF or config error.
    #[test]
    fn cli_output_write_renders_with_cli_layer_prefix() {
        // Arrange — the kind of io::Error a broken pipe on stdout would produce.
        let io = std::io::Error::other("broken pipe");
        let app_err: AppError = CliError::OutputWrite(io).into();

        // Act
        let msg = format!("{app_err}");

        // Assert
        assert!(
            msg.starts_with("cli error: output write:"),
            "Expected 'cli error: output write:' prefix, got: {msg}"
        );
        assert!(
            !msg.contains("pdf"),
            "CLI output write must not mention 'pdf', got: {msg}"
        );
    }

    #[test]
    fn output_write_from_io_error() {
        // Arrange — verify From<io::Error> is wired up
        let io = std::io::Error::other("disk full");

        // Act
        let err: CliError = io.into();

        // Assert
        let msg = format!("{err}");
        assert!(
            msg.starts_with("output write:"),
            "Expected 'output write:' prefix in: {msg}"
        );
    }
}
