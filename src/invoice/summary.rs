use time::{Date, Duration, Month};

use crate::config::types::Defaults;
use crate::error::AppError;

use super::currency::validate_uniform_currency;
use super::types::{InvoicePeriod, InvoiceSummary, LineItem, round_half_up_2dp};

/// Format invoice number as "INV-YYYY-MM".
fn format_invoice_number(period: &InvoicePeriod) -> String {
    format!("INV-{}-{:02}", period.year(), period.month())
}

/// Compute the month following the billed period.
/// December rolls over to January of the next year.
fn next_month(period: &InvoicePeriod) -> (i32, Month) {
    if period.month() == 12 {
        (period.year() as i32 + 1, Month::January)
    } else {
        let month = Month::try_from(period.month() as u8 + 1).unwrap();
        (period.year() as i32, month)
    }
}

/// Compute the invoice date: `invoice_date_day` of the month after the billed period.
/// Clamps the day if it exceeds the month's length (e.g., day=31 in a 30-day month).
fn compute_invoice_date(period: &InvoicePeriod, invoice_date_day: u32) -> Result<Date, AppError> {
    let (year, month) = next_month(period);
    let day = invoice_date_day as u8;
    match Date::from_calendar_date(year, month, day) {
        Ok(date) => Ok(date),
        Err(_) => {
            // Clamp to last day of month: try day-1, day-2, ...
            for d in (1..day).rev() {
                if let Ok(date) = Date::from_calendar_date(year, month, d) {
                    return Ok(date);
                }
            }
            Err(AppError::InvalidDate(format!(
                "Cannot construct date for {year}-{month:?}"
            )))
        }
    }
}

/// Build an `InvoiceSummary` from the collected period, line items, and config defaults.
pub fn build_summary(
    period: InvoicePeriod,
    line_items: Vec<LineItem>,
    defaults: &Defaults,
) -> Result<InvoiceSummary, AppError> {
    let invoice_number = format_invoice_number(&period);
    let invoice_date = compute_invoice_date(&period, defaults.invoice_date_day)?;
    let due_date = invoice_date + Duration::days(defaults.payment_terms_days as i64);
    let currency = validate_uniform_currency(&line_items)?;
    let subtotal = round_half_up_2dp(line_items.iter().map(|item| item.amount).sum());
    let tax_total = round_half_up_2dp(line_items.iter().map(|item| item.tax_amount).sum());
    let total = round_half_up_2dp(subtotal + tax_total);

    Ok(InvoiceSummary {
        invoice_number,
        period,
        invoice_date,
        due_date,
        currency,
        line_items,
        subtotal,
        tax_total,
        total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::Currency;

    fn make_defaults() -> Defaults {
        Defaults {
            currency: Currency::Eur,
            invoice_date_day: 9,
            payment_terms_days: 30,
            ..Defaults::default()
        }
    }

    fn make_items() -> Vec<LineItem> {
        vec![
            LineItem::new("Software development".into(), 10.0, 800.0, Currency::Eur),
            LineItem::new("Technical consulting".into(), 5.0, 1000.0, Currency::Eur),
        ]
    }

    // --- Invoice number tests ---

    #[test]
    fn invoice_number_normal() {
        // Arrange
        let period = InvoicePeriod::new(3, 2026).unwrap();

        // Act
        let number = format_invoice_number(&period);

        // Assert
        assert_eq!(number, "INV-2026-03");
    }

    #[test]
    fn invoice_number_december() {
        // Arrange
        let period = InvoicePeriod::new(12, 2025).unwrap();

        // Act
        let number = format_invoice_number(&period);

        // Assert
        assert_eq!(number, "INV-2025-12");
    }

    #[test]
    fn invoice_number_single_digit_month() {
        // Arrange
        let period = InvoicePeriod::new(1, 2026).unwrap();

        // Act
        let number = format_invoice_number(&period);

        // Assert
        assert_eq!(number, "INV-2026-01");
    }

    // --- Next month tests ---

    #[test]
    fn next_month_normal() {
        // Arrange
        let period = InvoicePeriod::new(3, 2026).unwrap();

        // Act
        let (year, month) = next_month(&period);

        // Assert
        assert_eq!(year, 2026);
        assert_eq!(month, Month::April);
    }

    #[test]
    fn next_month_december_rolls_year() {
        // Arrange
        let period = InvoicePeriod::new(12, 2025).unwrap();

        // Act
        let (year, month) = next_month(&period);

        // Assert
        assert_eq!(year, 2026);
        assert_eq!(month, Month::January);
    }

    // --- Invoice date tests ---

    #[test]
    fn invoice_date_normal() {
        // Arrange
        let period = InvoicePeriod::new(3, 2026).unwrap();

        // Act
        let date = compute_invoice_date(&period, 9).unwrap();

        // Assert
        assert_eq!(
            date,
            Date::from_calendar_date(2026, Month::April, 9).unwrap()
        );
    }

    #[test]
    fn invoice_date_december_billing() {
        // Arrange
        let period = InvoicePeriod::new(12, 2025).unwrap();

        // Act
        let date = compute_invoice_date(&period, 9).unwrap();

        // Assert
        assert_eq!(
            date,
            Date::from_calendar_date(2026, Month::January, 9).unwrap()
        );
    }

    #[test]
    fn invoice_date_day_clamped_february() {
        // Arrange — day 31 but Feb 2026 has only 28 days
        let period = InvoicePeriod::new(1, 2026).unwrap();

        // Act
        let date = compute_invoice_date(&period, 31).unwrap();

        // Assert
        assert_eq!(
            date,
            Date::from_calendar_date(2026, Month::February, 28).unwrap()
        );
    }

    #[test]
    fn invoice_date_day_clamped_april() {
        // Arrange — day 31 but April has 30 days
        let period = InvoicePeriod::new(3, 2026).unwrap();

        // Act
        let date = compute_invoice_date(&period, 31).unwrap();

        // Assert
        assert_eq!(
            date,
            Date::from_calendar_date(2026, Month::April, 30).unwrap()
        );
    }

    #[test]
    fn invoice_date_custom_day() {
        // Arrange
        let period = InvoicePeriod::new(3, 2026).unwrap();

        // Act
        let date = compute_invoice_date(&period, 15).unwrap();

        // Assert
        assert_eq!(
            date,
            Date::from_calendar_date(2026, Month::April, 15).unwrap()
        );
    }

    // --- Due date tests (via build_summary) ---

    #[test]
    fn due_date_normal() {
        // Arrange
        let period = InvoicePeriod::new(3, 2026).unwrap();
        let defaults = make_defaults(); // day=9, terms=30

        // Act
        let summary = build_summary(period, make_items(), &defaults).unwrap();

        // Assert — 2026-04-09 + 30 = 2026-05-09
        assert_eq!(
            summary.due_date,
            Date::from_calendar_date(2026, Month::May, 9).unwrap()
        );
    }

    #[test]
    fn due_date_crosses_year() {
        // Arrange — billing Nov 2025, invoice Dec 9, +30 = Jan 8 2026
        let period = InvoicePeriod::new(11, 2025).unwrap();
        let defaults = make_defaults();

        // Act
        let summary = build_summary(period, make_items(), &defaults).unwrap();

        // Assert
        assert_eq!(
            summary.due_date,
            Date::from_calendar_date(2026, Month::January, 8).unwrap()
        );
    }

    #[test]
    fn due_date_custom_payment_terms() {
        // Arrange
        let period = InvoicePeriod::new(3, 2026).unwrap();
        let defaults = Defaults {
            currency: Currency::Eur,
            invoice_date_day: 9,
            payment_terms_days: 14,
            ..Defaults::default()
        };

        // Act
        let summary = build_summary(period, make_items(), &defaults).unwrap();

        // Assert — 2026-04-09 + 14 = 2026-04-23
        assert_eq!(
            summary.due_date,
            Date::from_calendar_date(2026, Month::April, 23).unwrap()
        );
    }

    // --- Total computation tests ---

    #[test]
    fn build_summary_total_single_item() {
        // Arrange
        let period = InvoicePeriod::new(3, 2026).unwrap();
        let items = vec![LineItem::new("Dev".into(), 10.0, 800.0, Currency::Eur)];

        // Act
        let summary = build_summary(period, items, &make_defaults()).unwrap();

        // Assert
        assert!((summary.total - 8000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn build_summary_total_multiple_items() {
        // Arrange
        let period = InvoicePeriod::new(3, 2026).unwrap();
        let items = make_items(); // 8000 + 5000

        // Act
        let summary = build_summary(period, items, &make_defaults()).unwrap();

        // Assert
        assert!((summary.total - 13000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn build_summary_total_rounds_2dp() {
        // Arrange
        let period = InvoicePeriod::new(3, 2026).unwrap();
        let items = vec![
            LineItem::new("A".into(), 1.0, 33.33, Currency::Eur),
            LineItem::new("B".into(), 1.0, 66.67, Currency::Eur),
        ];

        // Act
        let summary = build_summary(period, items, &make_defaults()).unwrap();

        // Assert
        assert!((summary.total - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn build_summary_stores_currency() {
        // Arrange
        let period = InvoicePeriod::new(3, 2026).unwrap();
        let defaults = Defaults {
            currency: Currency::Usd,
            invoice_date_day: 9,
            payment_terms_days: 30,
            ..Defaults::default()
        };

        // Act
        let summary = build_summary(
            period,
            vec![LineItem::new("Dev".into(), 1.0, 100.0, Currency::Eur)],
            &defaults,
        )
        .unwrap();

        // Assert — currency comes from line items, not defaults
        assert_eq!(summary.currency, Currency::Eur);
    }

    #[test]
    fn build_summary_stores_period() {
        // Arrange
        let period = InvoicePeriod::new(6, 2025).unwrap();

        // Act
        let summary = build_summary(
            period,
            vec![LineItem::new("Dev".into(), 1.0, 100.0, Currency::Eur)],
            &make_defaults(),
        )
        .unwrap();

        // Assert
        assert_eq!(summary.period, period);
    }

    #[test]
    fn build_summary_derives_currency_from_line_items() {
        // Arrange — UAH replaces the old CZK fixture (closed Currency enum).
        let period = InvoicePeriod::new(3, 2026).unwrap();
        let defaults = Defaults {
            currency: Currency::Eur,
            invoice_date_day: 9,
            payment_terms_days: 30,
            ..Defaults::default()
        };
        let items = vec![LineItem::new("Dev".into(), 10.0, 800.0, Currency::Uah)];

        // Act
        let summary = build_summary(period, items, &defaults).unwrap();

        // Assert — currency comes from items, not defaults
        assert_eq!(summary.currency, Currency::Uah);
    }

    #[test]
    fn build_summary_mixed_currency_returns_error() {
        // Arrange
        let period = InvoicePeriod::new(3, 2026).unwrap();
        let items = vec![
            LineItem::new("Dev".into(), 10.0, 800.0, Currency::Eur),
            LineItem::new("QA".into(), 5.0, 600.0, Currency::Usd),
        ];

        // Act
        let result = build_summary(period, items, &make_defaults());

        // Assert
        assert!(matches!(result, Err(crate::error::AppError::MixedCurrency { .. })));
    }

    // --- subtotal / tax_total tests ---

    #[test]
    fn build_summary_zero_tax_subtotal_equals_total() {
        // Arrange
        let period = InvoicePeriod::new(3, 2026).unwrap();
        let items = make_items(); // all tax_rate 0

        // Act
        let summary = build_summary(period, items, &make_defaults()).unwrap();

        // Assert
        assert!((summary.subtotal - summary.total).abs() < f64::EPSILON);
    }

    #[test]
    fn build_summary_zero_tax_tax_total_is_zero() {
        // Arrange
        let period = InvoicePeriod::new(3, 2026).unwrap();
        let items = make_items();

        // Act
        let summary = build_summary(period, items, &make_defaults()).unwrap();

        // Assert
        assert!((summary.tax_total - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn build_summary_with_tax_computes_subtotal() {
        // Arrange
        let period = InvoicePeriod::new(3, 2026).unwrap();
        let items = vec![
            LineItem::with_tax("Dev".into(), 10.0, 800.0, Currency::Eur, 21.0),
            LineItem::with_tax("QA".into(), 5.0, 1000.0, Currency::Eur, 21.0),
        ];

        // Act
        let summary = build_summary(period, items, &make_defaults()).unwrap();

        // Assert — subtotal = 8000 + 5000 = 13000
        assert!((summary.subtotal - 13000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn build_summary_with_tax_computes_tax_total() {
        // Arrange
        let period = InvoicePeriod::new(3, 2026).unwrap();
        let items = vec![
            LineItem::with_tax("Dev".into(), 10.0, 800.0, Currency::Eur, 21.0),
            LineItem::with_tax("QA".into(), 5.0, 1000.0, Currency::Eur, 21.0),
        ];

        // Act
        let summary = build_summary(period, items, &make_defaults()).unwrap();

        // Assert — tax_total = 1680 + 1050 = 2730
        assert!((summary.tax_total - 2730.0).abs() < f64::EPSILON);
    }

    #[test]
    fn build_summary_with_tax_total_equals_subtotal_plus_tax() {
        // Arrange
        let period = InvoicePeriod::new(3, 2026).unwrap();
        let items = vec![
            LineItem::with_tax("Dev".into(), 10.0, 800.0, Currency::Eur, 21.0),
            LineItem::with_tax("QA".into(), 5.0, 1000.0, Currency::Eur, 21.0),
        ];

        // Act
        let summary = build_summary(period, items, &make_defaults()).unwrap();

        // Assert — total = 13000 + 2730 = 15730
        assert!((summary.total - (summary.subtotal + summary.tax_total)).abs() < f64::EPSILON);
        assert!((summary.total - 15730.0).abs() < f64::EPSILON);
    }

    #[test]
    fn build_summary_mixed_tax_and_no_tax_items() {
        // Arrange
        let period = InvoicePeriod::new(3, 2026).unwrap();
        let items = vec![
            LineItem::with_tax("Dev".into(), 10.0, 800.0, Currency::Eur, 21.0),
            LineItem::new("Admin".into(), 2.0, 500.0, Currency::Eur),
        ];

        // Act
        let summary = build_summary(period, items, &make_defaults()).unwrap();

        // Assert — subtotal = 8000 + 1000 = 9000, tax_total = 1680 + 0 = 1680
        assert!((summary.subtotal - 9000.0).abs() < f64::EPSILON);
        assert!((summary.tax_total - 1680.0).abs() < f64::EPSILON);
        assert!((summary.total - 10680.0).abs() < f64::EPSILON);
    }

    #[test]
    fn build_summary_tax_total_rounds_half_up() {
        // Arrange — items with tax_amounts that sum to a value needing rounding
        let period = InvoicePeriod::new(3, 2026).unwrap();
        // item1: amount=100.03, tax=21.0063 -> 21.01
        // item2: amount=100.03, tax=21.0063 -> 21.01
        // sum of tax_amounts = 42.02, which is already rounded
        // Use amounts that produce a sum needing rounding:
        // item1: 1 day * 33.33 = 33.33, tax at 10% = 3.333 -> 3.33
        // item2: 1 day * 66.67 = 66.67, tax at 10% = 6.667 -> 6.67
        // tax_total = round(3.33 + 6.67) = round(10.0) = 10.0
        let items = vec![
            LineItem::with_tax("A".into(), 1.0, 33.33, Currency::Eur, 10.0),
            LineItem::with_tax("B".into(), 1.0, 66.67, Currency::Eur, 10.0),
        ];

        // Act
        let summary = build_summary(period, items, &make_defaults()).unwrap();

        // Assert
        assert!((summary.tax_total - 10.0).abs() < f64::EPSILON);
    }
}
