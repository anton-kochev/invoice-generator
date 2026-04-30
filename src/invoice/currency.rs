use crate::config::types::Preset;
use crate::domain::Currency;
use crate::error::AppError;

use super::types::LineItem;

/// Resolve the effective currency for a preset.
/// Preset-level currency wins; falls back to the global default.
pub fn effective_currency(preset: &Preset, default: Currency) -> Currency {
    preset.currency.unwrap_or(default)
}

/// Validate that all line items share the same currency.
/// Returns the common currency on success, or `MixedCurrency` error listing the conflict.
///
/// # Panics
/// Panics if `items` is empty (callers guarantee at least one item).
pub fn validate_uniform_currency(items: &[LineItem]) -> Result<Currency, AppError> {
    let first = items[0].currency;
    for item in &items[1..] {
        if item.currency != first {
            return Err(AppError::MixedCurrency {
                first,
                second: item.currency,
            });
        }
    }
    Ok(first)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invoice::types::LineItem;

    fn make_preset(currency: Option<Currency>) -> Preset {
        Preset {
            key: crate::domain::PresetKey::try_new("dev").unwrap(),
            description: "Development".into(),
            default_rate: 800.0,
            currency,
            tax_rate: None,
        }
    }

    #[test]
    fn test_effective_currency_with_override_returns_override() {
        // Arrange
        let preset = make_preset(Some(Currency::Usd));
        let default = Currency::Eur;

        // Act
        let result = effective_currency(&preset, default);

        // Assert
        assert_eq!(result, Currency::Usd);
    }

    #[test]
    fn test_validate_single_item_passes() {
        // Arrange
        let items = vec![LineItem::new("Dev".into(), 10.0, 800.0, Currency::Eur)];

        // Act
        let result = validate_uniform_currency(&items);

        // Assert
        assert_eq!(result.unwrap(), Currency::Eur);
    }

    #[test]
    fn test_validate_same_currency_passes() {
        // Arrange
        let items = vec![
            LineItem::new("Dev".into(), 10.0, 800.0, Currency::Eur),
            LineItem::new("QA".into(), 5.0, 600.0, Currency::Eur),
        ];

        // Act
        let result = validate_uniform_currency(&items);

        // Assert
        assert_eq!(result.unwrap(), Currency::Eur);
    }

    #[test]
    fn test_validate_mixed_returns_error() {
        // Arrange
        let items = vec![
            LineItem::new("Dev".into(), 10.0, 800.0, Currency::Eur),
            LineItem::new("QA".into(), 5.0, 600.0, Currency::Usd),
        ];

        // Act
        let result = validate_uniform_currency(&items);

        // Assert
        match result {
            Err(AppError::MixedCurrency { first, second }) => {
                assert_eq!(first, Currency::Eur);
                assert_eq!(second, Currency::Usd);
            }
            other => panic!("Expected MixedCurrency, got {other:?}"),
        }
    }

    #[test]
    fn test_validate_three_currencies_reports_first_conflict() {
        // Arrange — first conflict (EUR vs USD) is reported; the third item
        // is irrelevant once we've already detected mixed currencies.
        let items = vec![
            LineItem::new("A".into(), 1.0, 100.0, Currency::Eur),
            LineItem::new("B".into(), 1.0, 100.0, Currency::Usd),
            LineItem::new("C".into(), 1.0, 100.0, Currency::Uah),
        ];

        // Act
        let result = validate_uniform_currency(&items);

        // Assert
        match result {
            Err(AppError::MixedCurrency { first, second }) => {
                assert_eq!(first, Currency::Eur);
                assert_eq!(second, Currency::Usd);
            }
            other => panic!("Expected MixedCurrency, got {other:?}"),
        }
    }

    #[test]
    fn test_validate_explicit_and_implicit_same_passes() {
        // Arrange — both resolve to EUR (one from None preset, one from Some(EUR)).
        // At the LineItem level, both just carry Currency::Eur.
        let items = vec![
            LineItem::new("Dev".into(), 10.0, 800.0, Currency::Eur),
            LineItem::new("QA".into(), 5.0, 600.0, Currency::Eur),
        ];

        // Act
        let result = validate_uniform_currency(&items);

        // Assert
        assert_eq!(result.unwrap(), Currency::Eur);
    }

    #[test]
    fn test_effective_currency_without_override_returns_default() {
        // Arrange
        let preset = make_preset(None);
        let default = Currency::Eur;

        // Act
        let result = effective_currency(&preset, default);

        // Assert
        assert_eq!(result, Currency::Eur);
    }
}
