use std::fmt;

use crate::config::types::Preset;

/// Result of the preset selection prompt.
#[derive(Debug, Clone, PartialEq)]
pub enum PresetSelection {
    /// User selected an existing preset.
    Existing(Preset),
    /// User wants to create a new preset (Story 3.3).
    CreateNew,
}

/// A validated invoice period (month + year).
///
/// Month is constrained to 1..=12, year to 2000..=2099.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InvoicePeriod {
    month: u32,
    year: u32,
}

impl InvoicePeriod {
    /// Create a new `InvoicePeriod` if month is 1..=12 and year is 2000..=2099.
    pub fn new(month: u32, year: u32) -> Option<Self> {
        if (1..=12).contains(&month) && (2000..=2099).contains(&year) {
            Some(Self { month, year })
        } else {
            None
        }
    }

    /// The month (1-12).
    pub fn month(&self) -> u32 {
        self.month
    }

    /// The year (2000-2099).
    pub fn year(&self) -> u32 {
        self.year
    }
}

impl fmt::Display for InvoicePeriod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{:02}", self.year, self.month)
    }
}

/// Round to 2 decimal places using half-up (round half away from zero).
pub fn round_half_up_2dp(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

/// A completed line item on the invoice.
#[derive(Debug, Clone, PartialEq)]
pub struct LineItem {
    /// Human-readable description (from preset).
    pub description: String,
    /// Number of days worked.
    pub days: f64,
    /// Rate per day.
    pub rate: f64,
    /// Computed amount: days * rate, rounded to 2 decimal places.
    pub amount: f64,
}

impl LineItem {
    /// Create a `LineItem`, computing amount as `days * rate` rounded to 2dp.
    pub fn new(description: String, days: f64, rate: f64) -> Self {
        Self {
            description,
            days,
            rate,
            amount: round_half_up_2dp(days * rate),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_construction() {
        // Arrange
        let month = 3;
        let year = 2026;

        // Act
        let period = InvoicePeriod::new(month, year);

        // Assert
        assert!(period.is_some());
        let p = period.unwrap();
        assert_eq!(p.month(), 3);
        assert_eq!(p.year(), 2026);
    }

    #[test]
    fn month_zero_rejected() {
        // Arrange
        let month = 0;
        let year = 2026;

        // Act
        let period = InvoicePeriod::new(month, year);

        // Assert
        assert!(period.is_none());
    }

    #[test]
    fn month_thirteen_rejected() {
        // Arrange
        let month = 13;
        let year = 2026;

        // Act
        let period = InvoicePeriod::new(month, year);

        // Assert
        assert!(period.is_none());
    }

    #[test]
    fn year_1999_rejected() {
        // Arrange
        let month = 6;
        let year = 1999;

        // Act
        let period = InvoicePeriod::new(month, year);

        // Assert
        assert!(period.is_none());
    }

    #[test]
    fn year_2100_rejected() {
        // Arrange
        let month = 6;
        let year = 2100;

        // Act
        let period = InvoicePeriod::new(month, year);

        // Assert
        assert!(period.is_none());
    }

    #[test]
    fn display_format() {
        // Arrange
        let period = InvoicePeriod::new(3, 2025).unwrap();

        // Act
        let formatted = format!("{period}");

        // Assert
        assert_eq!(formatted, "2025-03");
    }

    #[test]
    fn line_item_new_computes_amount() {
        // Arrange
        let days = 10.0;
        let rate = 800.0;

        // Act
        let item = LineItem::new("Software development".into(), days, rate);

        // Assert
        assert!((item.amount - 8000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn line_item_new_stores_description() {
        // Arrange
        let description = "Consulting work";

        // Act
        let item = LineItem::new(description.into(), 5.0, 100.0);

        // Assert
        assert_eq!(item.description, "Consulting work");
    }

    #[test]
    fn line_item_new_stores_days_and_rate() {
        // Arrange
        let days = 12.5;
        let rate = 750.0;

        // Act
        let item = LineItem::new("Dev work".into(), days, rate);

        // Assert
        assert!((item.days - 12.5).abs() < f64::EPSILON);
        assert!((item.rate - 750.0).abs() < f64::EPSILON);
    }

    #[test]
    fn line_item_new_rounds_half_up() {
        // Arrange — 10.5 * 100.03 = 1050.315 → 1050.32
        let days = 10.5;
        let rate = 100.03;

        // Act
        let item = LineItem::new("Dev".into(), days, rate);

        // Assert
        assert!((item.amount - 1050.32).abs() < f64::EPSILON);
    }

    #[test]
    fn line_item_new_rounds_down_below_five() {
        // Arrange — 10.0 * 1.111 = 11.11
        let days = 10.0;
        let rate = 1.111;

        // Act
        let item = LineItem::new("Dev".into(), days, rate);

        // Assert
        assert!((item.amount - 11.11).abs() < f64::EPSILON);
    }

    #[test]
    fn line_item_new_rounds_up_above_five() {
        // Arrange — 1.0 * 1.119 = 1.119 → 1.12
        let days = 1.0;
        let rate = 1.119;

        // Act
        let item = LineItem::new("Dev".into(), days, rate);

        // Assert
        assert!((item.amount - 1.12).abs() < f64::EPSILON);
    }

    #[test]
    fn line_item_new_exact_two_decimals_unchanged() {
        // Arrange — 5.0 * 100.0 = 500.00
        let days = 5.0;
        let rate = 100.0;

        // Act
        let item = LineItem::new("Dev".into(), days, rate);

        // Assert
        assert!((item.amount - 500.0).abs() < f64::EPSILON);
    }

    #[test]
    fn line_item_new_fractional_days() {
        // Arrange — 12.34 * 100.0 = 1234.0
        let days = 12.34;
        let rate = 100.0;

        // Act
        let item = LineItem::new("Dev".into(), days, rate);

        // Assert
        assert!((item.amount - 1234.0).abs() < f64::EPSILON);
    }

    #[test]
    fn line_item_new_small_fractional() {
        // Arrange — 0.5 * 0.01 = 0.005 → 0.01
        let days = 0.5;
        let rate = 0.01;

        // Act
        let item = LineItem::new("Dev".into(), days, rate);

        // Assert
        assert!((item.amount - 0.01).abs() < f64::EPSILON);
    }
}
