use crate::error::AppError;
use crate::setup::prompter::Prompter;

use super::types::InvoicePeriod;

/// Compute the default invoice period (previous month).
///
/// January wraps to December of the previous year.
pub fn default_period(current_month: u32, current_year: u32) -> (u32, u32) {
    if current_month == 1 {
        (12, current_year - 1)
    } else {
        (current_month - 1, current_year)
    }
}

/// Interactively collect an invoice period from the user.
///
/// Shows a header, computes defaults from the current date, then prompts
/// for month and year with validation loops.
pub fn collect_invoice_period(
    prompter: &dyn Prompter,
    current_month: u32,
    current_year: u32,
) -> Result<InvoicePeriod, AppError> {
    prompter.message("\nINVOICE GENERATOR\n");

    let (default_month, default_year) = default_period(current_month, current_year);

    // Prompt for month with validation
    let month = loop {
        let m = prompter.u32_with_default("Invoice month (1-12):", default_month)?;
        if (1..=12).contains(&m) {
            break m;
        }
        prompter.message("Month must be between 1 and 12.");
    };

    // Prompt for year with validation
    let year = loop {
        let y = prompter.u32_with_default("Invoice year:", default_year)?;
        if (2000..=2099).contains(&y) {
            break y;
        }
        prompter.message("Year must be between 2000 and 2099.");
    };

    Ok(InvoicePeriod::new(month, year).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::mock_prompter::{MockPrompter, MockResponse};

    // Phase 1 — default_period

    #[test]
    fn default_period_normal_month() {
        // Arrange
        let current_month = 4;
        let current_year = 2026;

        // Act
        let (month, year) = default_period(current_month, current_year);

        // Assert
        assert_eq!(month, 3);
        assert_eq!(year, 2026);
    }

    #[test]
    fn default_period_january_wraps() {
        // Arrange
        let current_month = 1;
        let current_year = 2026;

        // Act
        let (month, year) = default_period(current_month, current_year);

        // Assert
        assert_eq!(month, 12);
        assert_eq!(year, 2025);
    }

    #[test]
    fn default_period_february() {
        // Arrange
        let current_month = 2;
        let current_year = 2026;

        // Act
        let (month, year) = default_period(current_month, current_year);

        // Assert
        assert_eq!(month, 1);
        assert_eq!(year, 2026);
    }

    #[test]
    fn default_period_december() {
        // Arrange
        let current_month = 12;
        let current_year = 2026;

        // Act
        let (month, year) = default_period(current_month, current_year);

        // Assert
        assert_eq!(month, 11);
        assert_eq!(year, 2026);
    }

    // Phase 2 — collect happy path

    #[test]
    fn collect_displays_header() {
        // Arrange
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(3),
            MockResponse::U32(2026),
        ]);

        // Act
        collect_invoice_period(&prompter, 4, 2026).unwrap();

        // Assert
        let messages = prompter.messages.borrow();
        assert!(
            messages.iter().any(|m| m.contains("INVOICE GENERATOR")),
            "Expected header containing 'INVOICE GENERATOR', got: {messages:?}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn collect_accepts_default_values() {
        // Arrange
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(3),
            MockResponse::U32(2026),
        ]);

        // Act
        let period = collect_invoice_period(&prompter, 4, 2026).unwrap();

        // Assert
        assert_eq!(period.month(), 3);
        assert_eq!(period.year(), 2026);
        prompter.assert_exhausted();
    }

    #[test]
    fn collect_accepts_custom_values() {
        // Arrange
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(7),
            MockResponse::U32(2024),
        ]);

        // Act
        let period = collect_invoice_period(&prompter, 4, 2026).unwrap();

        // Assert
        assert_eq!(period.month(), 7);
        assert_eq!(period.year(), 2024);
        prompter.assert_exhausted();
    }

    #[test]
    fn collect_january_defaults() {
        // Arrange
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(12),
            MockResponse::U32(2025),
        ]);

        // Act
        let period = collect_invoice_period(&prompter, 1, 2026).unwrap();

        // Assert
        assert_eq!(period.month(), 12);
        assert_eq!(period.year(), 2025);
        prompter.assert_exhausted();
    }

    // Phase 3 — validation loops

    #[test]
    fn collect_reprompts_on_month_too_high() {
        // Arrange
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(13), // invalid
            MockResponse::U32(6),  // valid
            MockResponse::U32(2026),
        ]);

        // Act
        let period = collect_invoice_period(&prompter, 4, 2026).unwrap();

        // Assert
        assert_eq!(period.month(), 6);
        let messages = prompter.messages.borrow();
        let error_msg = messages.iter().find(|m| m.contains("1") && m.contains("12"));
        assert!(error_msg.is_some(), "Expected error mentioning 1 and 12, got: {messages:?}");
        prompter.assert_exhausted();
    }

    #[test]
    fn collect_reprompts_on_month_zero() {
        // Arrange
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(0), // invalid
            MockResponse::U32(1), // valid
            MockResponse::U32(2026),
        ]);

        // Act
        let period = collect_invoice_period(&prompter, 4, 2026).unwrap();

        // Assert
        assert_eq!(period.month(), 1);
        prompter.assert_exhausted();
    }

    #[test]
    fn collect_reprompts_on_year_below_2000() {
        // Arrange
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(3),
            MockResponse::U32(1999), // invalid
            MockResponse::U32(2000), // valid
        ]);

        // Act
        let period = collect_invoice_period(&prompter, 4, 2026).unwrap();

        // Assert
        assert_eq!(period.year(), 2000);
        let messages = prompter.messages.borrow();
        let error_msg = messages.iter().find(|m| m.contains("2000") && m.contains("2099"));
        assert!(error_msg.is_some(), "Expected error mentioning 2000 and 2099, got: {messages:?}");
        prompter.assert_exhausted();
    }

    #[test]
    fn collect_reprompts_on_year_above_2099() {
        // Arrange
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(3),
            MockResponse::U32(2100), // invalid
            MockResponse::U32(2099), // valid
        ]);

        // Act
        let period = collect_invoice_period(&prompter, 4, 2026).unwrap();

        // Assert
        assert_eq!(period.year(), 2099);
        prompter.assert_exhausted();
    }

    #[test]
    fn collect_reprompts_on_invalid_month_then_invalid_year() {
        // Arrange
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(0),    // invalid month
            MockResponse::U32(6),    // valid month
            MockResponse::U32(1999), // invalid year
            MockResponse::U32(2026), // valid year
        ]);

        // Act
        let period = collect_invoice_period(&prompter, 4, 2026).unwrap();

        // Assert
        assert_eq!(period.month(), 6);
        assert_eq!(period.year(), 2026);
        prompter.assert_exhausted();
    }
}
