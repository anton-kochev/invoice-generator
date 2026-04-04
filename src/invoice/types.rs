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

    /// Full month name for display (e.g., "March").
    pub fn month_name(&self) -> &'static str {
        match self.month {
            1 => "January",
            2 => "February",
            3 => "March",
            4 => "April",
            5 => "May",
            6 => "June",
            7 => "July",
            8 => "August",
            9 => "September",
            10 => "October",
            11 => "November",
            12 => "December",
            _ => unreachable!("InvoicePeriod month is always 1..=12"),
        }
    }

    /// Format as "March 2026" for invoice display.
    pub fn display_long(&self) -> String {
        format!("{} {}", self.month_name(), self.year)
    }

    /// Three-letter month abbreviation (e.g., "Mar").
    pub fn month_abbrev(&self) -> &'static str {
        match self.month {
            1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr",
            5 => "May", 6 => "Jun", 7 => "Jul", 8 => "Aug",
            9 => "Sep", 10 => "Oct", 11 => "Nov", 12 => "Dec",
            _ => unreachable!("InvoicePeriod month is always 1..=12"),
        }
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
    /// Currency code (e.g. "EUR", "USD").
    pub currency: String,
    /// Tax rate as a percentage (e.g. 21.0 for 21%).
    pub tax_rate: f64,
    /// Computed tax amount: amount * tax_rate / 100, rounded to 2dp.
    pub tax_amount: f64,
}

impl LineItem {
    /// Create a `LineItem`, computing amount as `days * rate` rounded to 2dp.
    /// Tax fields default to 0.0 (no tax).
    pub fn new(description: String, days: f64, rate: f64, currency: String) -> Self {
        Self {
            description,
            days,
            rate,
            amount: round_half_up_2dp(days * rate),
            currency,
            tax_rate: 0.0,
            tax_amount: 0.0,
        }
    }

    /// Create a `LineItem` with a tax rate applied.
    /// Computes `tax_amount = amount * tax_rate / 100`, rounded to 2dp.
    pub fn with_tax(description: String, days: f64, rate: f64, currency: String, tax_rate: f64) -> Self {
        let amount = round_half_up_2dp(days * rate);
        let tax_amount = round_half_up_2dp(amount * tax_rate / 100.0);
        Self {
            description,
            days,
            rate,
            amount,
            currency,
            tax_rate,
            tax_amount,
        }
    }
}

/// A fully computed invoice summary, ready for display or PDF generation.
#[derive(Debug, Clone, PartialEq)]
pub struct InvoiceSummary {
    /// e.g. "INV-2025-12"
    pub invoice_number: String,
    /// The billed period (month + year).
    pub period: InvoicePeriod,
    /// Invoice issue date (day after billing period month).
    pub invoice_date: time::Date,
    /// Payment due date (invoice_date + payment_terms_days).
    pub due_date: time::Date,
    /// Currency code, e.g. "EUR".
    pub currency: String,
    /// The individual line items.
    pub line_items: Vec<LineItem>,
    /// Sum of all line item base amounts (before tax), rounded to 2dp.
    pub subtotal: f64,
    /// Sum of all line item tax amounts, rounded to 2dp.
    pub tax_total: f64,
    /// Grand total: subtotal + tax_total, rounded to 2dp.
    pub total: f64,
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
    fn month_name_all_months() {
        // Arrange & Act & Assert
        let names = [
            (1, "January"),
            (2, "February"),
            (3, "March"),
            (4, "April"),
            (5, "May"),
            (6, "June"),
            (7, "July"),
            (8, "August"),
            (9, "September"),
            (10, "October"),
            (11, "November"),
            (12, "December"),
        ];
        for (month, expected) in names {
            let period = InvoicePeriod::new(month, 2026).unwrap();
            assert_eq!(period.month_name(), expected);
        }
    }

    #[test]
    fn display_long_format() {
        // Arrange
        let period = InvoicePeriod::new(3, 2026).unwrap();

        // Act
        let result = period.display_long();

        // Assert
        assert_eq!(result, "March 2026");
    }

    #[test]
    fn line_item_new_computes_amount() {
        // Arrange
        let days = 10.0;
        let rate = 800.0;

        // Act
        let item = LineItem::new("Software development".into(), days, rate, "EUR".into());

        // Assert
        assert!((item.amount - 8000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn line_item_new_stores_description() {
        // Arrange
        let description = "Consulting work";

        // Act
        let item = LineItem::new(description.into(), 5.0, 100.0, "EUR".into());

        // Assert
        assert_eq!(item.description, "Consulting work");
    }

    #[test]
    fn line_item_new_stores_days_and_rate() {
        // Arrange
        let days = 12.5;
        let rate = 750.0;

        // Act
        let item = LineItem::new("Dev work".into(), days, rate, "EUR".into());

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
        let item = LineItem::new("Dev".into(), days, rate, "EUR".into());

        // Assert
        assert!((item.amount - 1050.32).abs() < f64::EPSILON);
    }

    #[test]
    fn line_item_new_rounds_down_below_five() {
        // Arrange — 10.0 * 1.111 = 11.11
        let days = 10.0;
        let rate = 1.111;

        // Act
        let item = LineItem::new("Dev".into(), days, rate, "EUR".into());

        // Assert
        assert!((item.amount - 11.11).abs() < f64::EPSILON);
    }

    #[test]
    fn line_item_new_rounds_up_above_five() {
        // Arrange — 1.0 * 1.119 = 1.119 → 1.12
        let days = 1.0;
        let rate = 1.119;

        // Act
        let item = LineItem::new("Dev".into(), days, rate, "EUR".into());

        // Assert
        assert!((item.amount - 1.12).abs() < f64::EPSILON);
    }

    #[test]
    fn line_item_new_exact_two_decimals_unchanged() {
        // Arrange — 5.0 * 100.0 = 500.00
        let days = 5.0;
        let rate = 100.0;

        // Act
        let item = LineItem::new("Dev".into(), days, rate, "EUR".into());

        // Assert
        assert!((item.amount - 500.0).abs() < f64::EPSILON);
    }

    #[test]
    fn line_item_new_fractional_days() {
        // Arrange — 12.34 * 100.0 = 1234.0
        let days = 12.34;
        let rate = 100.0;

        // Act
        let item = LineItem::new("Dev".into(), days, rate, "EUR".into());

        // Assert
        assert!((item.amount - 1234.0).abs() < f64::EPSILON);
    }

    #[test]
    fn line_item_new_small_fractional() {
        // Arrange — 0.5 * 0.01 = 0.005 → 0.01
        let days = 0.5;
        let rate = 0.01;

        // Act
        let item = LineItem::new("Dev".into(), days, rate, "EUR".into());

        // Assert
        assert!((item.amount - 0.01).abs() < f64::EPSILON);
    }

    #[test]
    fn line_item_new_stores_currency() {
        // Arrange
        let days = 10.0;
        let rate = 800.0;

        // Act
        let item = LineItem::new("Dev".into(), days, rate, "USD".into());

        // Assert
        assert_eq!(item.currency, "USD");
    }

    #[test]
    fn line_item_new_still_computes_amount_with_currency() {
        // Arrange
        let days = 10.5;
        let rate = 100.03;

        // Act
        let item = LineItem::new("Dev".into(), days, rate, "CZK".into());

        // Assert
        assert!((item.amount - 1050.32).abs() < f64::EPSILON);
        assert_eq!(item.currency, "CZK");
    }

    #[test]
    fn month_abbrev_all_months() {
        // Arrange & Act & Assert
        let abbrevs = [
            (1, "Jan"), (2, "Feb"), (3, "Mar"), (4, "Apr"),
            (5, "May"), (6, "Jun"), (7, "Jul"), (8, "Aug"),
            (9, "Sep"), (10, "Oct"), (11, "Nov"), (12, "Dec"),
        ];
        for (month, expected) in abbrevs {
            let period = InvoicePeriod::new(month, 2026).unwrap();
            assert_eq!(period.month_abbrev(), expected);
        }
    }

    // --- LineItem tax field tests ---

    #[test]
    fn line_item_new_default_tax_fields_are_zero() {
        // Arrange
        let days = 10.0;
        let rate = 800.0;

        // Act
        let item = LineItem::new("Dev".into(), days, rate, "EUR".into());

        // Assert
        assert!((item.tax_rate - 0.0).abs() < f64::EPSILON);
        assert!((item.tax_amount - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn line_item_with_tax_zero_rate_gives_zero_tax_amount() {
        // Arrange & Act
        let item = LineItem::with_tax("Dev".into(), 10.0, 800.0, "EUR".into(), 0.0);

        // Assert
        assert!((item.tax_amount - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn line_item_with_tax_computes_tax_amount() {
        // Arrange & Act
        let item = LineItem::with_tax("Dev".into(), 10.0, 800.0, "EUR".into(), 21.0);

        // Assert
        assert!((item.amount - 8000.0).abs() < f64::EPSILON);
        assert!((item.tax_amount - 1680.0).abs() < f64::EPSILON);
    }

    #[test]
    fn line_item_with_tax_amount_rounds_half_up() {
        // Arrange — amount = 100.03, tax = 100.03 * 21 / 100 = 21.0063 → 21.01
        // Act
        let item = LineItem::with_tax("Dev".into(), 1.0, 100.03, "EUR".into(), 21.0);

        // Assert
        assert!((item.tax_amount - 21.01).abs() < f64::EPSILON);
    }

    #[test]
    fn line_item_with_tax_does_not_affect_base_amount() {
        // Arrange
        let without = LineItem::new("Dev".into(), 10.0, 800.0, "EUR".into());

        // Act
        let with = LineItem::with_tax("Dev".into(), 10.0, 800.0, "EUR".into(), 21.0);

        // Assert
        assert!((with.amount - without.amount).abs() < f64::EPSILON);
    }

    #[test]
    fn line_item_with_tax_high_rate_100_percent() {
        // Arrange & Act
        let item = LineItem::with_tax("Dev".into(), 5.0, 200.0, "EUR".into(), 100.0);

        // Assert — amount = 1000.0, tax = 1000.0
        assert!((item.amount - 1000.0).abs() < f64::EPSILON);
        assert!((item.tax_amount - 1000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn line_item_with_tax_fractional_rate() {
        // Arrange & Act
        let item = LineItem::with_tax("Dev".into(), 10.0, 800.0, "EUR".into(), 7.5);

        // Assert — amount = 8000.0, tax = 8000.0 * 7.5 / 100 = 600.0
        assert!((item.tax_amount - 600.0).abs() < f64::EPSILON);
    }
}
