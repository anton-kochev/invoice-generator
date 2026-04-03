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
}
