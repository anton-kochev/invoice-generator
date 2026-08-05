//! The tax-rate invariant, shared by every boundary that accepts one.

/// Returns `true` when `rate` is usable as a tax percentage.
///
/// A tax rate is a percentage applied to a line item's amount, so it must be
/// finite and non-negative. `0.0` is valid and means "untaxed".
///
/// This is a free predicate rather than a newtype because `tax_rate` enters
/// the program through two unrelated boundaries — `Preset.tax_rate` in the
/// YAML config and the `tax_rate` key of a `--items` JSON entry — that report
/// failures with different error types (`ConfigError` vs. `InvoiceError`).
/// Sharing the predicate keeps the two checks from drifting apart.
pub fn is_valid_tax_rate(rate: f64) -> bool {
    rate.is_finite() && rate >= 0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_positive_rate_is_valid() {
        // Arrange
        let rate = 21.0;

        // Act
        let valid = is_valid_tax_rate(rate);

        // Assert
        assert!(valid);
    }

    #[test]
    fn test_zero_rate_is_valid() {
        // Arrange — 0.0 is the explicit "untaxed" spelling.
        let rate = 0.0;

        // Act
        let valid = is_valid_tax_rate(rate);

        // Assert
        assert!(valid);
    }

    #[test]
    fn test_negative_rate_is_invalid() {
        // Arrange
        let rate = -21.0;

        // Act
        let valid = is_valid_tax_rate(rate);

        // Assert
        assert!(!valid);
    }

    #[test]
    fn test_non_finite_rates_are_invalid() {
        // Arrange
        let rates = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY];

        // Act & Assert
        for rate in rates {
            assert!(!is_valid_tax_rate(rate), "expected {rate} to be rejected");
        }
    }
}
