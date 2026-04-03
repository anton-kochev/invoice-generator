use time::{Date, Duration, Month};

use crate::config::types::Defaults;
use crate::error::AppError;

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
    let total = round_half_up_2dp(line_items.iter().map(|item| item.amount).sum());

    Ok(InvoiceSummary {
        invoice_number,
        period,
        invoice_date,
        due_date,
        currency: defaults.currency.clone(),
        line_items,
        total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_defaults() -> Defaults {
        Defaults {
            currency: "EUR".into(),
            invoice_date_day: 9,
            payment_terms_days: 30,
        }
    }

    fn make_items() -> Vec<LineItem> {
        vec![
            LineItem::new("Software development".into(), 10.0, 800.0),
            LineItem::new("Technical consulting".into(), 5.0, 1000.0),
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
            currency: "EUR".into(),
            invoice_date_day: 9,
            payment_terms_days: 14,
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
        let items = vec![LineItem::new("Dev".into(), 10.0, 800.0)];

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
            LineItem::new("A".into(), 1.0, 33.33),
            LineItem::new("B".into(), 1.0, 66.67),
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
            currency: "USD".into(),
            invoice_date_day: 9,
            payment_terms_days: 30,
        };

        // Act
        let summary = build_summary(
            period,
            vec![LineItem::new("Dev".into(), 1.0, 100.0)],
            &defaults,
        )
        .unwrap();

        // Assert
        assert_eq!(summary.currency, "USD");
    }

    #[test]
    fn build_summary_stores_period() {
        // Arrange
        let period = InvoicePeriod::new(6, 2025).unwrap();

        // Act
        let summary = build_summary(
            period,
            vec![LineItem::new("Dev".into(), 1.0, 100.0)],
            &make_defaults(),
        )
        .unwrap();

        // Assert
        assert_eq!(summary.period, period);
    }
}
