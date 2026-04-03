use crate::config::types::Preset;
use crate::error::AppError;
use crate::setup::prompter::Prompter;

use super::types::LineItem;

/// Interactively collect line item details for a given preset.
pub fn collect_line_item_details(
    prompter: &dyn Prompter,
    preset: &Preset,
    item_number: u32,
) -> Result<LineItem, AppError> {
    prompter.message(&format!(
        "\nLine item #{}: {}",
        item_number, preset.description
    ));

    let days = prompter.positive_f64("Days worked:")?;

    let rate = prompter.positive_f64_with_default(
        &format!("Rate per day [{}]:", preset.default_rate),
        preset.default_rate,
    )?;

    let item = LineItem::new(preset.description.clone(), days, rate);

    prompter.message(&format!(
        "  => {:.2} days x {:.2}/day = {:.2}",
        item.days, item.rate, item.amount
    ));

    Ok(item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::mock_prompter::{MockPrompter, MockResponse};

    fn make_preset() -> Preset {
        Preset {
            key: "dev".into(),
            description: "Software development".into(),
            default_rate: 800.0,
        }
    }

    #[test]
    fn displays_header_with_item_number_and_description() {
        // Arrange
        let preset = make_preset();
        let prompter = MockPrompter::new(vec![
            MockResponse::F64(10.0),
            MockResponse::F64(800.0),
        ]);

        // Act
        let _ = collect_line_item_details(&prompter, &preset, 1).unwrap();

        // Assert
        let messages = prompter.messages.borrow();
        assert!(messages[0].contains("Line item #1"));
        assert!(messages[0].contains("Software development"));
    }

    #[test]
    fn collects_days_and_accepts_default_rate() {
        // Arrange
        let preset = make_preset();
        let prompter = MockPrompter::new(vec![
            MockResponse::F64(10.0),
            MockResponse::F64(800.0),
        ]);

        // Act
        let item = collect_line_item_details(&prompter, &preset, 1).unwrap();

        // Assert
        assert!((item.days - 10.0).abs() < f64::EPSILON);
        assert!((item.rate - 800.0).abs() < f64::EPSILON);
        assert!((item.amount - 8000.0).abs() < f64::EPSILON);
        prompter.assert_exhausted();
    }

    #[test]
    fn collects_days_and_custom_rate() {
        // Arrange
        let preset = make_preset();
        let prompter = MockPrompter::new(vec![
            MockResponse::F64(5.0),
            MockResponse::F64(1200.0),
        ]);

        // Act
        let item = collect_line_item_details(&prompter, &preset, 1).unwrap();

        // Assert
        assert!((item.days - 5.0).abs() < f64::EPSILON);
        assert!((item.rate - 1200.0).abs() < f64::EPSILON);
        assert!((item.amount - 6000.0).abs() < f64::EPSILON);
        prompter.assert_exhausted();
    }

    #[test]
    fn fractional_days_accepted() {
        // Arrange
        let preset = make_preset();
        let prompter = MockPrompter::new(vec![
            MockResponse::F64(12.34),
            MockResponse::F64(100.0),
        ]);

        // Act
        let item = collect_line_item_details(&prompter, &preset, 1).unwrap();

        // Assert
        assert!((item.amount - 1234.0).abs() < f64::EPSILON);
    }

    #[test]
    fn displays_computed_amount_summary() {
        // Arrange
        let preset = make_preset();
        let prompter = MockPrompter::new(vec![
            MockResponse::F64(10.0),
            MockResponse::F64(800.0),
        ]);

        // Act
        let _ = collect_line_item_details(&prompter, &preset, 1).unwrap();

        // Assert
        let messages = prompter.messages.borrow();
        assert!(messages[1].contains("8000.00"));
    }

    #[test]
    fn uses_preset_description_in_line_item() {
        // Arrange
        let preset = make_preset();
        let prompter = MockPrompter::new(vec![
            MockResponse::F64(1.0),
            MockResponse::F64(100.0),
        ]);

        // Act
        let item = collect_line_item_details(&prompter, &preset, 1).unwrap();

        // Assert
        assert_eq!(item.description, "Software development");
    }

    #[test]
    fn item_number_displayed_correctly() {
        // Arrange
        let preset = make_preset();
        let prompter = MockPrompter::new(vec![
            MockResponse::F64(1.0),
            MockResponse::F64(100.0),
        ]);

        // Act
        let _ = collect_line_item_details(&prompter, &preset, 3).unwrap();

        // Assert
        let messages = prompter.messages.borrow();
        assert!(messages[0].contains("Line item #3"));
    }
}
