use crate::config::types::Preset;
use crate::error::AppError;
use crate::setup::prompter::Prompter;

use super::preset_selection::select_preset;
use super::types::{LineItem, PresetSelection};

/// Collect one or more line items, each with its own preset selection.
pub fn collect_all_line_items(
    prompter: &dyn Prompter,
    presets: &[Preset],
    currency: &str,
) -> Result<Vec<LineItem>, AppError> {
    let mut items = Vec::new();

    loop {
        let selection = select_preset(prompter, presets, currency)?;

        match selection {
            PresetSelection::Existing(preset) => {
                let item_number = (items.len() + 1) as u32;
                let item = collect_line_item_details(prompter, &preset, item_number)?;
                items.push(item);
            }
            PresetSelection::CreateNew => {
                prompter.message("Create new preset (not yet implemented)");
                continue;
            }
        }

        if !prompter.confirm("Add another line item?", false)? {
            break;
        }
    }

    Ok(items)
}

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

    fn make_presets() -> Vec<Preset> {
        vec![
            Preset {
                key: "dev".into(),
                description: "Software development".into(),
                default_rate: 800.0,
            },
            Preset {
                key: "consulting".into(),
                description: "Technical consulting".into(),
                default_rate: 1000.0,
            },
        ]
    }

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

    // --- collect_all_line_items tests ---

    #[test]
    fn collect_all_single_item_decline_more() {
        // Arrange
        let presets = make_presets();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(1),          // select preset #1
            MockResponse::F64(10.0),       // days
            MockResponse::F64(800.0),      // rate
            MockResponse::Confirm(false),  // add another? no
        ]);

        // Act
        let items = collect_all_line_items(&prompter, &presets, "EUR").unwrap();

        // Assert
        assert_eq!(items.len(), 1);
        assert!((items[0].amount - 8000.0).abs() < f64::EPSILON);
        prompter.assert_exhausted();
    }

    #[test]
    fn collect_all_two_items_then_decline() {
        // Arrange
        let presets = make_presets();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(1),          // select preset #1
            MockResponse::F64(10.0),       // days
            MockResponse::F64(800.0),      // rate
            MockResponse::Confirm(true),   // add another? yes
            MockResponse::U32(2),          // select preset #2
            MockResponse::F64(5.0),        // days
            MockResponse::F64(1000.0),     // rate
            MockResponse::Confirm(false),  // add another? no
        ]);

        // Act
        let items = collect_all_line_items(&prompter, &presets, "EUR").unwrap();

        // Assert
        assert_eq!(items.len(), 2);
        assert!((items[0].amount - 8000.0).abs() < f64::EPSILON);
        assert!((items[1].amount - 5000.0).abs() < f64::EPSILON);
        prompter.assert_exhausted();
    }

    #[test]
    fn collect_all_three_items_in_order() {
        // Arrange
        let presets = make_presets();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(1), MockResponse::F64(1.0), MockResponse::F64(100.0), MockResponse::Confirm(true),
            MockResponse::U32(1), MockResponse::F64(2.0), MockResponse::F64(200.0), MockResponse::Confirm(true),
            MockResponse::U32(1), MockResponse::F64(3.0), MockResponse::F64(300.0), MockResponse::Confirm(false),
        ]);

        // Act
        let items = collect_all_line_items(&prompter, &presets, "EUR").unwrap();

        // Assert
        assert_eq!(items.len(), 3);
        assert!((items[0].amount - 100.0).abs() < f64::EPSILON);
        assert!((items[1].amount - 400.0).abs() < f64::EPSILON);
        assert!((items[2].amount - 900.0).abs() < f64::EPSILON);
        prompter.assert_exhausted();
    }

    #[test]
    fn collect_all_increments_item_numbers_in_headers() {
        // Arrange
        let presets = make_presets();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(1), MockResponse::F64(10.0), MockResponse::F64(800.0), MockResponse::Confirm(true),
            MockResponse::U32(1), MockResponse::F64(5.0), MockResponse::F64(800.0), MockResponse::Confirm(false),
        ]);

        // Act
        let _ = collect_all_line_items(&prompter, &presets, "EUR").unwrap();

        // Assert
        let messages = prompter.messages.borrow();
        let all = messages.join("\n");
        assert!(all.contains("Line item #1"), "Expected 'Line item #1' in messages");
        assert!(all.contains("Line item #2"), "Expected 'Line item #2' in messages");
    }

    #[test]
    fn collect_all_different_presets_per_item() {
        // Arrange
        let presets = make_presets();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(1), MockResponse::F64(10.0), MockResponse::F64(800.0), MockResponse::Confirm(true),
            MockResponse::U32(2), MockResponse::F64(5.0), MockResponse::F64(1000.0), MockResponse::Confirm(false),
        ]);

        // Act
        let items = collect_all_line_items(&prompter, &presets, "EUR").unwrap();

        // Assert
        assert_eq!(items[0].description, "Software development");
        assert_eq!(items[1].description, "Technical consulting");
    }

    #[test]
    fn collect_all_create_new_skips_and_retries() {
        // Arrange
        let presets = make_presets();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(3),          // CreateNew (2 presets + 1)
            MockResponse::U32(1),          // retry: select preset #1
            MockResponse::F64(10.0),       // days
            MockResponse::F64(800.0),      // rate
            MockResponse::Confirm(false),  // add another? no
        ]);

        // Act
        let items = collect_all_line_items(&prompter, &presets, "EUR").unwrap();

        // Assert
        assert_eq!(items.len(), 1);
        let messages = prompter.messages.borrow();
        let all = messages.join("\n");
        assert!(all.contains("not yet implemented"), "Expected 'not yet implemented' message");
        prompter.assert_exhausted();
    }

    #[test]
    fn collect_all_preserves_amounts() {
        // Arrange
        let presets = make_presets();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(1), MockResponse::F64(12.5), MockResponse::F64(800.0), MockResponse::Confirm(true),
            MockResponse::U32(2), MockResponse::F64(3.0), MockResponse::F64(1500.0), MockResponse::Confirm(false),
        ]);

        // Act
        let items = collect_all_line_items(&prompter, &presets, "EUR").unwrap();

        // Assert
        assert!((items[0].amount - 10000.0).abs() < f64::EPSILON);
        assert!((items[1].amount - 4500.0).abs() < f64::EPSILON);
    }
}
