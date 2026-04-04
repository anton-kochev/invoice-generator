use crate::config::types::Preset;
use crate::error::AppError;

use super::types::LineItem;

/// Resolve the effective currency for a preset.
/// Preset-level currency wins; falls back to the global default.
pub fn effective_currency<'a>(preset: &'a Preset, default: &'a str) -> &'a str {
    preset.currency.as_deref().unwrap_or(default)
}

/// Validate that all line items share the same currency.
/// Returns the common currency on success, or `MixedCurrency` error listing conflicts.
///
/// # Panics
/// Panics if `items` is empty (callers guarantee at least one item).
pub fn validate_uniform_currency(items: &[LineItem]) -> Result<String, AppError> {
    let first = &items[0].currency;
    let mut seen = vec![first.as_str()];
    for item in &items[1..] {
        if !seen.contains(&item.currency.as_str()) {
            seen.push(&item.currency);
        }
    }
    if seen.len() == 1 {
        Ok(first.clone())
    } else {
        Err(AppError::MixedCurrency(seen.join(", ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invoice::types::LineItem;

    fn make_preset(currency: Option<String>) -> Preset {
        Preset {
            key: "dev".into(),
            description: "Development".into(),
            default_rate: 800.0,
            currency,
        }
    }

    #[test]
    fn test_effective_currency_with_override_returns_override() {
        // Arrange
        let preset = make_preset(Some("USD".into()));
        let default = "EUR";

        // Act
        let result = effective_currency(&preset, default);

        // Assert
        assert_eq!(result, "USD");
    }

    #[test]
    fn test_validate_single_item_passes() {
        // Arrange
        let items = vec![LineItem::new("Dev".into(), 10.0, 800.0, "EUR".into())];

        // Act
        let result = validate_uniform_currency(&items);

        // Assert
        assert_eq!(result.unwrap(), "EUR");
    }

    #[test]
    fn test_validate_same_currency_passes() {
        // Arrange
        let items = vec![
            LineItem::new("Dev".into(), 10.0, 800.0, "EUR".into()),
            LineItem::new("QA".into(), 5.0, 600.0, "EUR".into()),
        ];

        // Act
        let result = validate_uniform_currency(&items);

        // Assert
        assert_eq!(result.unwrap(), "EUR");
    }

    #[test]
    fn test_validate_mixed_returns_error() {
        // Arrange
        let items = vec![
            LineItem::new("Dev".into(), 10.0, 800.0, "EUR".into()),
            LineItem::new("QA".into(), 5.0, 600.0, "USD".into()),
        ];

        // Act
        let result = validate_uniform_currency(&items);

        // Assert
        match result {
            Err(AppError::MixedCurrency(msg)) => {
                assert!(msg.contains("EUR"), "Expected 'EUR' in: {msg}");
                assert!(msg.contains("USD"), "Expected 'USD' in: {msg}");
            }
            other => panic!("Expected MixedCurrency, got {other:?}"),
        }
    }

    #[test]
    fn test_validate_three_currencies_lists_all() {
        // Arrange
        let items = vec![
            LineItem::new("A".into(), 1.0, 100.0, "EUR".into()),
            LineItem::new("B".into(), 1.0, 100.0, "USD".into()),
            LineItem::new("C".into(), 1.0, 100.0, "CZK".into()),
        ];

        // Act
        let result = validate_uniform_currency(&items);

        // Assert
        match result {
            Err(AppError::MixedCurrency(msg)) => {
                assert!(msg.contains("EUR"), "Expected 'EUR' in: {msg}");
                assert!(msg.contains("USD"), "Expected 'USD' in: {msg}");
                assert!(msg.contains("CZK"), "Expected 'CZK' in: {msg}");
            }
            other => panic!("Expected MixedCurrency, got {other:?}"),
        }
    }

    #[test]
    fn test_validate_explicit_and_implicit_same_passes() {
        // Arrange — both resolve to "EUR" (one from None preset, one from Some("EUR") preset)
        // At the LineItem level, both just carry "EUR" as their currency string
        let items = vec![
            LineItem::new("Dev".into(), 10.0, 800.0, "EUR".into()),
            LineItem::new("QA".into(), 5.0, 600.0, "EUR".into()),
        ];

        // Act
        let result = validate_uniform_currency(&items);

        // Assert
        assert_eq!(result.unwrap(), "EUR");
    }

    #[test]
    fn test_effective_currency_without_override_returns_default() {
        // Arrange
        let preset = make_preset(None);
        let default = "EUR";

        // Act
        let result = effective_currency(&preset, default);

        // Assert
        assert_eq!(result, "EUR");
    }
}
