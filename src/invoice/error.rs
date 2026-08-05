//! Errors produced by the `invoice` subsystem.
//!
//! These cover invoice computation: period validation, line-item parsing,
//! currency uniformity checks, tax-rate validation, and the
//! template/locale lookups performed when assembling an invoice.
//!
//! Composes into [`crate::error::AppError`] via `#[from]`.

use thiserror::Error;

use crate::domain::{BillingUnit, Currency};

/// Errors produced by the invoice subsystem.
#[derive(Debug, Error)]
pub enum InvoiceError {
    /// Invalid date during invoice computation (e.g. nonexistent calendar date).
    #[error("invalid date: {0}")]
    InvalidDate(String),

    /// Invalid quantity in `--quantity`/`--days`/`--hours` or `--items` JSON.
    #[error("invalid quantity: {0} (must be > 0)")]
    InvalidQuantity(String),

    /// A unit assertion — a `--days`/`--hours` flag, or the `days` key of an
    /// `--items` entry — contradicts the referenced preset's billing unit.
    ///
    /// `--quantity` (and the `quantity` key) is unit-agnostic and can never
    /// produce this error; the point of the stricter spellings is to refuse to
    /// silently bill hours as days.
    ///
    /// `remedy` is carried rather than derived because it depends on where the
    /// assertion came from: an `--items` entry has no `--hours` flag to reach
    /// for, it has the unit-agnostic `quantity` key instead.
    #[error("{flag} cannot be used with preset \"{preset}\" (billed in {unit}) — {remedy}")]
    UnitMismatch {
        flag: &'static str,
        preset: String,
        unit: BillingUnit,
        remedy: String,
    },

    /// An `--items` entry set both `days` and `quantity`.
    ///
    /// The two can disagree, and `days` additionally asserts a unit, so there
    /// is no safe way to pick a winner on a money document.
    #[error("--items entry for preset \"{preset}\" sets both \"days\" and \"quantity\" — use one")]
    ConflictingItemAmount { preset: String },

    /// An `--items` entry set neither `days` nor `quantity`.
    #[error("--items entry for preset \"{preset}\" needs a \"quantity\" (or \"days\") amount")]
    MissingItemAmount { preset: String },

    /// `--preset` was given without any of `--quantity`/`--days`/`--hours`.
    ///
    /// clap's `amount` group normally rejects this before the handler runs;
    /// the variant exists so the resolution path can return an error instead
    /// of panicking if that wiring ever regresses.
    #[error("--preset requires one of --quantity, --days, or --hours")]
    MissingQuantity,

    /// Invalid tax rate (must be >= 0).
    #[error("invalid tax rate: {0} (must be >= 0)")]
    InvalidTaxRate(String),

    /// Line items have conflicting currencies — first conflict reported.
    #[error("mixed currencies in line items: {first} and {second}")]
    MixedCurrency { first: Currency, second: Currency },

    /// Failed to parse `--items` JSON.
    ///
    /// Stored as `serde_json::Error` so `?` works on JSON parse calls inside
    /// invoice/cli code.
    #[error("failed to parse --items JSON: {0}")]
    ItemsParse(#[from] serde_json::Error),

    /// `--items` parsed successfully but contained no entries.
    #[error("--items array must not be empty")]
    EmptyItems,

    /// Unknown locale code.
    #[error("unknown locale: \"{key}\". Available: {}", available.join(", "))]
    InvalidLocale { key: String, available: Vec<String> },
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
    fn test_invalid_quantity_displays_value() {
        // Arrange
        let err = InvoiceError::InvalidQuantity("0".into());

        // Act
        let msg = format!("{err}");

        // Assert
        assert_eq!(msg, "invalid quantity: 0 (must be > 0)");
    }

    #[test]
    fn test_unit_mismatch_names_flag_preset_and_remedy() {
        // Arrange
        let err = InvoiceError::UnitMismatch {
            flag: "--hours",
            preset: "dev".into(),
            unit: BillingUnit::Day,
            remedy: "use --days or --quantity".into(),
        };

        // Act
        let msg = format!("{err}");

        // Assert
        assert_eq!(
            msg,
            "--hours cannot be used with preset \"dev\" (billed in days) — use --days or --quantity"
        );
    }

    #[test]
    fn test_unit_mismatch_suggests_hours_for_hourly_preset() {
        // Arrange
        let err = InvoiceError::UnitMismatch {
            flag: "--days",
            preset: "support".into(),
            unit: BillingUnit::Hour,
            remedy: "use --hours or --quantity".into(),
        };

        // Act
        let msg = format!("{err}");

        // Assert
        assert_eq!(
            msg,
            "--days cannot be used with preset \"support\" (billed in hours) — use --hours or --quantity"
        );
    }

    #[test]
    fn test_unit_mismatch_from_items_points_at_the_quantity_key() {
        // Arrange — the --items spelling must not suggest flags that cannot be
        // combined with --items.
        let err = InvoiceError::UnitMismatch {
            flag: "the \"days\" key in --items",
            preset: "support".into(),
            unit: BillingUnit::Hour,
            remedy: "use \"quantity\" instead".into(),
        };

        // Act
        let msg = format!("{err}");

        // Assert
        assert_eq!(
            msg,
            "the \"days\" key in --items cannot be used with preset \"support\" (billed in hours) — use \"quantity\" instead"
        );
    }

    #[test]
    fn test_conflicting_item_amount_names_preset() {
        // Arrange
        let err = InvoiceError::ConflictingItemAmount {
            preset: "dev".into(),
        };

        // Act
        let msg = format!("{err}");

        // Assert
        assert_eq!(
            msg,
            "--items entry for preset \"dev\" sets both \"days\" and \"quantity\" — use one"
        );
    }

    #[test]
    fn test_missing_item_amount_names_preset() {
        // Arrange
        let err = InvoiceError::MissingItemAmount {
            preset: "dev".into(),
        };

        // Act
        let msg = format!("{err}");

        // Assert
        assert_eq!(
            msg,
            "--items entry for preset \"dev\" needs a \"quantity\" (or \"days\") amount"
        );
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
}
