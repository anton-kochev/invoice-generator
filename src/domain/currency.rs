//! [`Currency`] closed enum: the only currencies the invoice generator
//! supports today are USD, EUR, and UAH.
//!
//! Anything outside this set is rejected at the boundary (deserialization,
//! `FromStr`, or interactive prompts) so the rest of the program can rely
//! on exhaustive `match` checks instead of stringly-typed currency codes.

use std::fmt;
use std::str::FromStr;

use thiserror::Error;

/// Validation error for [`Currency`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum CurrencyError {
    /// Input parsed successfully but is not one of the supported codes.
    #[error("unsupported currency \"{0}\" (supported: USD, EUR, UAH)")]
    Unsupported(String),
}

/// Closed set of currencies supported by the invoice generator.
///
/// Serializes as the 3-letter ISO 4217 code (`"USD"`, `"EUR"`, `"UAH"`).
/// Deserialization is case-insensitive but always fails for codes outside
/// this set.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Currency {
    /// US dollar.
    Usd,
    /// Euro.
    Eur,
    /// Ukrainian hryvnia.
    Uah,
}

impl Currency {
    /// All supported currencies, in declaration order.
    pub const ALL: [Currency; 3] = [Currency::Usd, Currency::Eur, Currency::Uah];

    /// 3-letter ISO 4217 code (uppercase).
    pub fn code(&self) -> &'static str {
        match self {
            Self::Usd => "USD",
            Self::Eur => "EUR",
            Self::Uah => "UAH",
        }
    }

    /// Conventional symbol (`$`, `€`, `₴`).
    #[allow(dead_code)] // exposed for future template/UX use
    pub fn symbol(&self) -> &'static str {
        match self {
            Self::Usd => "$",
            Self::Eur => "€",
            Self::Uah => "₴",
        }
    }
}

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.code())
    }
}

impl FromStr for Currency {
    type Err = CurrencyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_uppercase().as_str() {
            "USD" => Ok(Self::Usd),
            "EUR" => Ok(Self::Eur),
            "UAH" => Ok(Self::Uah),
            other => Err(CurrencyError::Unsupported(other.to_string())),
        }
    }
}

impl serde::Serialize for Currency {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.code())
    }
}

impl<'de> serde::Deserialize<'de> for Currency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::from_str(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── code / symbol ──

    #[test]
    fn test_code_returns_uppercase_iso() {
        // Arrange & Act & Assert
        assert_eq!(Currency::Usd.code(), "USD");
        assert_eq!(Currency::Eur.code(), "EUR");
        assert_eq!(Currency::Uah.code(), "UAH");
    }

    #[test]
    fn test_symbol_returns_currency_symbol() {
        // Arrange & Act & Assert
        assert_eq!(Currency::Usd.symbol(), "$");
        assert_eq!(Currency::Eur.symbol(), "\u{20AC}");
        assert_eq!(Currency::Uah.symbol(), "\u{20B4}");
    }

    #[test]
    fn test_all_contains_three_variants() {
        // Arrange & Act & Assert
        assert_eq!(Currency::ALL.len(), 3);
        assert!(Currency::ALL.contains(&Currency::Usd));
        assert!(Currency::ALL.contains(&Currency::Eur));
        assert!(Currency::ALL.contains(&Currency::Uah));
    }

    // ── Display ──

    #[test]
    fn test_display_outputs_uppercase_code() {
        // Arrange & Act & Assert
        assert_eq!(format!("{}", Currency::Usd), "USD");
        assert_eq!(format!("{}", Currency::Eur), "EUR");
        assert_eq!(format!("{}", Currency::Uah), "UAH");
    }

    // ── FromStr ──

    #[test]
    fn test_from_str_accepts_uppercase() {
        // Arrange & Act & Assert
        assert_eq!("USD".parse::<Currency>().unwrap(), Currency::Usd);
        assert_eq!("EUR".parse::<Currency>().unwrap(), Currency::Eur);
        assert_eq!("UAH".parse::<Currency>().unwrap(), Currency::Uah);
    }

    #[test]
    fn test_from_str_accepts_lowercase() {
        // Arrange & Act & Assert
        assert_eq!("usd".parse::<Currency>().unwrap(), Currency::Usd);
        assert_eq!("eur".parse::<Currency>().unwrap(), Currency::Eur);
        assert_eq!("uah".parse::<Currency>().unwrap(), Currency::Uah);
    }

    #[test]
    fn test_from_str_accepts_mixed_case() {
        // Arrange & Act & Assert
        assert_eq!("Usd".parse::<Currency>().unwrap(), Currency::Usd);
        assert_eq!("eUr".parse::<Currency>().unwrap(), Currency::Eur);
    }

    #[test]
    fn test_from_str_trims_whitespace() {
        // Arrange & Act & Assert
        assert_eq!("  USD  ".parse::<Currency>().unwrap(), Currency::Usd);
    }

    #[test]
    fn test_from_str_rejects_unsupported_code() {
        // Arrange & Act
        let result: Result<Currency, _> = "GBP".parse();

        // Assert
        assert!(matches!(result, Err(CurrencyError::Unsupported(s)) if s == "GBP"));
    }

    #[test]
    fn test_from_str_rejects_czk() {
        // Arrange — CZK was supported as a free-form string before this enum.
        // Now it is rejected at the boundary.
        let result: Result<Currency, _> = "CZK".parse();

        // Assert
        assert!(matches!(result, Err(CurrencyError::Unsupported(_))));
    }

    #[test]
    fn test_from_str_rejects_empty() {
        // Arrange & Act
        let result: Result<Currency, _> = "".parse();

        // Assert
        assert!(matches!(result, Err(CurrencyError::Unsupported(_))));
    }

    // ── serde ──

    #[test]
    fn test_serializes_as_uppercase_code_yaml() {
        // Arrange
        let c = Currency::Eur;

        // Act
        let yaml = serde_yaml::to_string(&c).unwrap();

        // Assert
        assert_eq!(yaml.trim(), "EUR");
    }

    #[test]
    fn test_serializes_as_uppercase_code_json() {
        // Arrange
        let c = Currency::Uah;

        // Act
        let json = serde_json::to_string(&c).unwrap();

        // Assert
        assert_eq!(json, "\"UAH\"");
    }

    #[test]
    fn test_yaml_round_trip_all_variants() {
        // Arrange & Act & Assert
        for c in Currency::ALL {
            let yaml = serde_yaml::to_string(&c).unwrap();
            let loaded: Currency = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(loaded, c);
        }
    }

    #[test]
    fn test_json_round_trip_all_variants() {
        // Arrange & Act & Assert
        for c in Currency::ALL {
            let json = serde_json::to_string(&c).unwrap();
            let loaded: Currency = serde_json::from_str(&json).unwrap();
            assert_eq!(loaded, c);
        }
    }

    #[test]
    fn test_deserialize_lowercase_code_succeeds() {
        // Arrange
        let yaml = "eur\n";

        // Act
        let c: Currency = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert_eq!(c, Currency::Eur);
    }

    #[test]
    fn test_deserialize_unsupported_code_fails() {
        // Arrange
        let yaml = "GBP\n";

        // Act
        let result: Result<Currency, _> = serde_yaml::from_str(yaml);

        // Assert
        assert!(result.is_err(), "Expected deserialize failure for GBP");
    }

    #[test]
    fn test_deserialize_czk_fails() {
        // Arrange — CZK is no longer accepted; existing configs with CZK
        // will fail to load on next parse.
        let yaml = "CZK\n";

        // Act
        let result: Result<Currency, _> = serde_yaml::from_str(yaml);

        // Assert
        assert!(result.is_err(), "Expected deserialize failure for CZK");
    }

    // ── Copy semantics ──

    #[test]
    fn test_currency_is_copy() {
        // Arrange
        let c = Currency::Usd;

        // Act — moving here would compile-error if Currency weren't Copy.
        let c2 = c;
        let c3 = c;

        // Assert
        assert_eq!(c2, Currency::Usd);
        assert_eq!(c3, Currency::Usd);
    }

    #[test]
    fn test_distinct_variants_not_equal() {
        // Arrange & Act & Assert
        assert_ne!(Currency::Usd, Currency::Eur);
        assert_ne!(Currency::Eur, Currency::Uah);
        assert_ne!(Currency::Usd, Currency::Uah);
    }
}
