//! [`BillingUnit`] closed enum: a preset bills either per day or per hour.
//!
//! Anything outside this set is rejected at the boundary (deserialization,
//! `FromStr`, or interactive prompts) so the rest of the program can rely on
//! exhaustive `match` checks instead of a stringly-typed unit.

use std::fmt;
use std::str::FromStr;

use thiserror::Error;

/// Validation error for [`BillingUnit`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum BillingUnitError {
    /// Input parsed successfully but is not one of the supported units.
    #[error("unsupported billing unit \"{0}\" (supported: days, hours)")]
    Unsupported(String),
}

/// Closed set of billing units supported by the invoice generator.
///
/// Serializes as its lowercase plural key (`"days"`, `"hours"`).
/// Deserialization is case-insensitive and also accepts the singular and
/// single-letter spellings, but always fails for anything outside this set.
///
/// [`Default`] is [`BillingUnit::Day`], and that is load-bearing for
/// backward compatibility: a config written before billing units existed has
/// no unit field, and those presets are daily by definition. Deriving the
/// default here is what lets `#[serde(default)]` on the field do that work.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Default)]
pub enum BillingUnit {
    /// Billed per day.
    #[default]
    Day,
    /// Billed per hour.
    Hour,
}

impl BillingUnit {
    /// All supported billing units, in declaration order.
    // Consumed by the interactive billing-unit prompt (phase 5).
    #[allow(dead_code)]
    pub const ALL: [BillingUnit; 2] = [BillingUnit::Day, BillingUnit::Hour];

    /// Lowercase plural key (`"days"`, `"hours"`).
    ///
    /// This is both the serialized form and the value handed to templates as
    /// the line item's `unit`.
    pub fn key(&self) -> &'static str {
        match self {
            Self::Day => "days",
            Self::Hour => "hours",
        }
    }

    /// Capitalized plural label (`"Days"`, `"Hours"`) for the PDF quantity
    /// column header.
    // Consumed by the PDF data layer's `unit_label` (phase 8).
    #[allow(dead_code)]
    pub fn label(&self) -> &'static str {
        match self {
            Self::Day => "Days",
            Self::Hour => "Hours",
        }
    }

    /// Lowercase singular noun (`"day"`, `"hour"`), used in prompts and
    /// summaries such as `Rate per {singular}`.
    // Consumed by prompts (phase 5) and display surfaces (phase 6).
    #[allow(dead_code)]
    pub fn singular(&self) -> &'static str {
        match self {
            Self::Day => "day",
            Self::Hour => "hour",
        }
    }
}

impl fmt::Display for BillingUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.key())
    }
}

impl FromStr for BillingUnit {
    type Err = BillingUnitError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "day" | "days" | "d" => Ok(Self::Day),
            "hour" | "hours" | "h" => Ok(Self::Hour),
            other => Err(BillingUnitError::Unsupported(other.to_string())),
        }
    }
}

impl serde::Serialize for BillingUnit {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.key())
    }
}

impl<'de> serde::Deserialize<'de> for BillingUnit {
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

    // ── key / label / singular ──

    #[test]
    fn test_key_returns_plural_lowercase() {
        // Arrange & Act & Assert
        assert_eq!(BillingUnit::Day.key(), "days");
        assert_eq!(BillingUnit::Hour.key(), "hours");
    }

    #[test]
    fn test_label_returns_capitalized_plural() {
        // Arrange & Act & Assert
        assert_eq!(BillingUnit::Day.label(), "Days");
        assert_eq!(BillingUnit::Hour.label(), "Hours");
    }

    #[test]
    fn test_singular_returns_lowercase_singular() {
        // Arrange & Act & Assert
        assert_eq!(BillingUnit::Day.singular(), "day");
        assert_eq!(BillingUnit::Hour.singular(), "hour");
    }

    #[test]
    fn test_all_contains_two_variants() {
        // Arrange & Act & Assert
        assert_eq!(BillingUnit::ALL.len(), 2);
        assert!(BillingUnit::ALL.contains(&BillingUnit::Day));
        assert!(BillingUnit::ALL.contains(&BillingUnit::Hour));
    }

    // ── Default ──

    #[test]
    fn test_default_is_day() {
        // Arrange & Act
        let unit = BillingUnit::default();

        // Assert — back-compat lever: configs with no unit field mean daily.
        assert_eq!(unit, BillingUnit::Day);
    }

    // ── Display ──

    #[test]
    fn test_display_outputs_key() {
        // Arrange & Act & Assert
        assert_eq!(format!("{}", BillingUnit::Day), "days");
        assert_eq!(format!("{}", BillingUnit::Hour), "hours");
    }

    // ── FromStr ──

    #[test]
    fn test_from_str_accepts_day_spellings() {
        // Arrange & Act & Assert
        assert_eq!("days".parse::<BillingUnit>().unwrap(), BillingUnit::Day);
        assert_eq!("Day".parse::<BillingUnit>().unwrap(), BillingUnit::Day);
        assert_eq!("D".parse::<BillingUnit>().unwrap(), BillingUnit::Day);
    }

    #[test]
    fn test_from_str_accepts_hour_spellings() {
        // Arrange & Act & Assert
        assert_eq!("hours".parse::<BillingUnit>().unwrap(), BillingUnit::Hour);
        assert_eq!("HOUR".parse::<BillingUnit>().unwrap(), BillingUnit::Hour);
        assert_eq!("h".parse::<BillingUnit>().unwrap(), BillingUnit::Hour);
    }

    #[test]
    fn test_from_str_trims_whitespace() {
        // Arrange & Act & Assert
        assert_eq!(
            "  hours  ".parse::<BillingUnit>().unwrap(),
            BillingUnit::Hour
        );
    }

    #[test]
    fn test_from_str_rejects_unsupported_unit() {
        // Arrange & Act
        let result: Result<BillingUnit, _> = "weeks".parse();

        // Assert
        assert!(matches!(result, Err(BillingUnitError::Unsupported(s)) if s == "weeks"));
    }

    #[test]
    fn test_from_str_rejects_empty() {
        // Arrange & Act
        let result: Result<BillingUnit, _> = "".parse();

        // Assert
        assert!(matches!(result, Err(BillingUnitError::Unsupported(_))));
    }

    // ── serde ──

    #[test]
    fn test_serializes_as_key_yaml() {
        // Arrange
        let unit = BillingUnit::Hour;

        // Act
        let yaml = serde_yaml::to_string(&unit).unwrap();

        // Assert
        assert_eq!(yaml.trim(), "hours");
    }

    #[test]
    fn test_serializes_as_key_json() {
        // Arrange & Act & Assert
        assert_eq!(
            serde_json::to_string(&BillingUnit::Day).unwrap(),
            "\"days\""
        );
        assert_eq!(
            serde_json::to_string(&BillingUnit::Hour).unwrap(),
            "\"hours\""
        );
    }

    #[test]
    fn test_yaml_round_trip_all_variants() {
        // Arrange & Act & Assert
        for unit in BillingUnit::ALL {
            let yaml = serde_yaml::to_string(&unit).unwrap();
            let loaded: BillingUnit = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(loaded, unit);
        }
    }

    #[test]
    fn test_deserialize_mixed_case_succeeds() {
        // Arrange
        let yaml = "Hours\n";

        // Act
        let unit: BillingUnit = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert_eq!(unit, BillingUnit::Hour);
    }

    #[test]
    fn test_deserialize_unsupported_unit_fails() {
        // Arrange
        let yaml = "weeks\n";

        // Act
        let result: Result<BillingUnit, _> = serde_yaml::from_str(yaml);

        // Assert
        assert!(result.is_err(), "Expected deserialize failure for weeks");
    }

    // ── Copy semantics ──

    #[test]
    fn test_billing_unit_is_copy() {
        // Arrange
        let unit = BillingUnit::Hour;

        // Act — moving here would compile-error if BillingUnit weren't Copy.
        let a = unit;
        let b = unit;

        // Assert
        assert_eq!(a, BillingUnit::Hour);
        assert_eq!(b, BillingUnit::Hour);
    }

    #[test]
    fn test_distinct_variants_not_equal() {
        // Arrange & Act & Assert
        assert_ne!(BillingUnit::Day, BillingUnit::Hour);
    }
}
