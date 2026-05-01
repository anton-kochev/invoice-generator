use super::types::InvoiceSummary;

/// Format the invoice summary as an ASCII text box for terminal display.
pub fn format_summary(summary: &InvoiceSummary) -> String {
    let width = 48;
    let border = format!("+{}+", "-".repeat(width));
    let mut lines = Vec::new();

    lines.push(border.clone());
    lines.push(format!("| {:^w$} |", "INVOICE SUMMARY", w = width - 2));
    lines.push(border.clone());
    lines.push(format!(
        "| {:<w$} |",
        format!("Invoice:  {}", summary.invoice_number),
        w = width - 2
    ));
    lines.push(format!(
        "| {:<w$} |",
        format!("Period:   {}", summary.period),
        w = width - 2
    ));
    lines.push(format!(
        "| {:<w$} |",
        format!("Date:     {}", summary.invoice_date),
        w = width - 2
    ));
    lines.push(format!(
        "| {:<w$} |",
        format!("Due:      {}", summary.due_date),
        w = width - 2
    ));
    lines.push(border.clone());

    for item in &summary.line_items {
        lines.push(format!("| {:<w$} |", item.description, w = width - 2));
        lines.push(format!(
            "| {:<w$} |",
            format!(
                "  {:.2} days x {:.2} = {:.2} {}",
                item.days, item.rate, item.amount, summary.currency
            ),
            w = width - 2
        ));
        if item.tax_rate > 0.0 {
            lines.push(format!(
                "| {:<w$} |",
                format!(
                    "  tax {:.1}%: {:.2} {}",
                    item.tax_rate, item.tax_amount, summary.currency
                ),
                w = width - 2
            ));
        }
    }

    lines.push(border.clone());
    if summary.tax_total > 0.0 {
        lines.push(format!(
            "| {:<w$} |",
            format!("SUBTOTAL: {:.2} {}", summary.subtotal, summary.currency),
            w = width - 2
        ));
        lines.push(format!(
            "| {:<w$} |",
            format!("TAX: {:.2} {}", summary.tax_total, summary.currency),
            w = width - 2
        ));
        lines.push(format!(
            "| {:<w$} |",
            format!("TOTAL: {:.2} {}", summary.total, summary.currency),
            w = width - 2
        ));
    } else {
        lines.push(format!(
            "| {:<w$} |",
            format!("TOTAL: {:.2} {}", summary.total, summary.currency),
            w = width - 2
        ));
    }
    lines.push(border);

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::invoice::types::{InvoicePeriod, LineItem};
    use time::{Date, Month};

    fn make_summary() -> InvoiceSummary {
        InvoiceSummary {
            invoice_number: "INV-2026-03".into(),
            period: InvoicePeriod::new(3, 2026).unwrap(),
            invoice_date: Date::from_calendar_date(2026, Month::April, 9).unwrap(),
            due_date: Date::from_calendar_date(2026, Month::May, 9).unwrap(),
            currency: crate::domain::Currency::Eur,
            line_items: vec![
                LineItem::new(
                    "Software development".into(),
                    10.0,
                    800.0,
                    crate::domain::Currency::Eur,
                ),
                LineItem::new(
                    "Technical consulting".into(),
                    5.0,
                    1000.0,
                    crate::domain::Currency::Eur,
                ),
            ],
            subtotal: 13000.0,
            tax_total: 0.0,
            total: 13000.0,
        }
    }

    #[test]
    fn format_contains_invoice_number() {
        // Arrange
        let summary = make_summary();

        // Act
        let output = format_summary(&summary);

        // Assert
        assert!(output.contains("INV-2026-03"));
    }

    #[test]
    fn format_contains_period() {
        // Arrange
        let summary = make_summary();

        // Act
        let output = format_summary(&summary);

        // Assert
        assert!(output.contains("2026-03"));
    }

    #[test]
    fn format_contains_invoice_date() {
        // Arrange
        let summary = make_summary();

        // Act
        let output = format_summary(&summary);

        // Assert
        assert!(output.contains("2026-04-09"));
    }

    #[test]
    fn format_contains_due_date() {
        // Arrange
        let summary = make_summary();

        // Act
        let output = format_summary(&summary);

        // Assert
        assert!(output.contains("2026-05-09"));
    }

    #[test]
    fn format_contains_line_item_descriptions() {
        // Arrange
        let summary = make_summary();

        // Act
        let output = format_summary(&summary);

        // Assert
        assert!(output.contains("Software development"));
        assert!(output.contains("Technical consulting"));
    }

    #[test]
    fn format_contains_line_item_amounts() {
        // Arrange
        let summary = make_summary();

        // Act
        let output = format_summary(&summary);

        // Assert
        assert!(output.contains("8000.00"));
        assert!(output.contains("5000.00"));
    }

    #[test]
    fn format_contains_total() {
        // Arrange
        let summary = make_summary();

        // Act
        let output = format_summary(&summary);

        // Assert
        assert!(output.contains("13000.00"));
    }

    #[test]
    fn format_contains_currency() {
        // Arrange
        let summary = make_summary();

        // Act
        let output = format_summary(&summary);

        // Assert
        assert!(output.contains("EUR"));
    }

    fn make_summary_with_tax() -> InvoiceSummary {
        InvoiceSummary {
            invoice_number: "INV-2026-03".into(),
            period: InvoicePeriod::new(3, 2026).unwrap(),
            invoice_date: Date::from_calendar_date(2026, Month::April, 9).unwrap(),
            due_date: Date::from_calendar_date(2026, Month::May, 9).unwrap(),
            currency: crate::domain::Currency::Eur,
            line_items: vec![LineItem::with_tax(
                "Software development".into(),
                10.0,
                800.0,
                crate::domain::Currency::Eur,
                21.0,
            )],
            subtotal: 8000.0,
            tax_total: 1680.0,
            total: 9680.0,
        }
    }

    #[test]
    fn format_summary_without_tax_shows_single_total() {
        // Arrange
        let summary = make_summary();

        // Act
        let output = format_summary(&summary);

        // Assert
        assert!(output.contains("TOTAL:"));
        assert!(!output.contains("SUBTOTAL"));
    }

    #[test]
    fn format_summary_with_tax_shows_subtotal_line() {
        // Arrange
        let summary = make_summary_with_tax();

        // Act
        let output = format_summary(&summary);

        // Assert
        assert!(output.contains("SUBTOTAL:"));
    }

    #[test]
    fn format_summary_with_tax_shows_tax_line() {
        // Arrange
        let summary = make_summary_with_tax();

        // Act
        let output = format_summary(&summary);

        // Assert
        assert!(output.contains("TAX:"));
    }

    #[test]
    fn format_summary_with_tax_shows_total_after_tax() {
        // Arrange
        let summary = make_summary_with_tax();

        // Act
        let output = format_summary(&summary);

        // Assert
        assert!(output.contains("9680.00"));
    }

    #[test]
    fn format_summary_with_tax_shows_per_item_tax_rate() {
        // Arrange
        let summary = make_summary_with_tax();

        // Act
        let output = format_summary(&summary);

        // Assert
        assert!(output.contains("tax 21.0%"));
    }

    #[test]
    fn format_has_box_borders() {
        // Arrange
        let summary = make_summary();

        // Act
        let output = format_summary(&summary);

        // Assert
        assert!(output.starts_with("+---"));
        assert!(output.contains("|"));
        assert!(output.contains("INVOICE SUMMARY"));
    }
}
