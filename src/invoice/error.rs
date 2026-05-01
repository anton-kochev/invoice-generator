//! Errors produced by the `invoice` subsystem.
//!
//! These cover invoice computation: period validation, line-item parsing,
//! currency uniformity checks, tax-rate validation, and the
//! template/locale lookups performed when assembling an invoice.
//!
//! Composes into [`crate::error::AppError`] via `#[from]`.

use thiserror::Error;

use crate::domain::Currency;

/// Errors produced by the invoice subsystem.
#[derive(Debug, Error)]
pub enum InvoiceError {
    /// Invalid date during invoice computation (e.g. nonexistent calendar date).
    #[error("invalid date: {0}")]
    InvalidDate(String),

    /// Invalid days value in `--days` or `--items` JSON.
    #[error("invalid days value: {0} (must be > 0)")]
    InvalidDays(String),

    /// Invalid tax rate (must be >= 0).
    #[error("invalid tax rate: {0} (must be >= 0)")]
    InvalidTaxRate(String),

    /// Line items have conflicting currencies — first conflict reported.
    #[error("mixed currencies in line items: {first} and {second}")]
    MixedCurrency {
        first: Currency,
        second: Currency,
    },

    /// Failed to parse `--items` JSON.
    ///
    /// Stored as `serde_json::Error` so `?` works on JSON parse calls inside
    /// invoice/cli code.
    #[error("failed to parse --items JSON: {0}")]
    ItemsParse(#[from] serde_json::Error),

    /// `--items` parsed successfully but contained no entries.
    #[error("--items array must not be empty")]
    EmptyItems,

    /// Unknown template key.
    #[error("unknown template: \"{key}\". Available: {}", available.join(", "))]
    InvalidTemplateKey {
        key: String,
        available: Vec<String>,
    },

    /// Unknown locale code.
    #[error("unknown locale: \"{key}\". Available: {}", available.join(", "))]
    InvalidLocale {
        key: String,
        available: Vec<String>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mixed_currency_error_displays_currencies() {
        // Arrange
        let err = InvoiceError::MixedCurrency {
            first: Currency::Eur,
            second: Currency::Usd,
        };

        // Act
        let msg = format!("{err}");

        // Assert
        assert!(msg.contains("EUR"), "Expected 'EUR' in: {msg}");
        assert!(msg.contains("USD"), "Expected 'USD' in: {msg}");
    }

    #[test]
    fn test_invalid_tax_rate_displays_value() {
        // Arrange
        let err = InvoiceError::InvalidTaxRate("-5.0".into());

        // Act
        let msg = format!("{err}");

        // Assert
        assert!(msg.contains("-5.0"), "Expected '-5.0' in: {msg}");
        assert!(msg.contains(">= 0"), "Expected '>= 0' in: {msg}");
    }

    #[test]
    fn test_invalid_template_key_displays_key_and_available() {
        // Arrange
        let err = InvoiceError::InvalidTemplateKey {
            key: "ganymede".into(),
            available: vec!["callisto".into(), "leda".into(), "thebe".into()],
        };

        // Act
        let msg = format!("{err}");

        // Assert
        assert!(msg.contains("ganymede"), "Expected 'ganymede' in: {msg}");
        assert!(msg.contains("callisto"), "Expected 'callisto' in: {msg}");
        assert!(msg.contains("leda"), "Expected 'leda' in: {msg}");
        assert!(msg.contains("thebe"), "Expected 'thebe' in: {msg}");
    }
}
