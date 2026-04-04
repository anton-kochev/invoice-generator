use std::path::Path;

use crate::config::types::Preset;
use crate::config::writer::append_preset;
use crate::error::AppError;
use crate::setup::prompter::Prompter;

use super::preset_creation;
use super::preset_selection::select_preset;
use super::types::{LineItem, PresetSelection};

/// Collect one or more line items, each with its own preset selection.
pub fn collect_all_line_items(
    prompter: &dyn Prompter,
    presets: &[Preset],
    currency: &str,
    dir: &Path,
) -> Result<Vec<LineItem>, AppError> {
    let mut items = Vec::new();
    let mut presets = presets.to_vec();

    loop {
        let selection = select_preset(prompter, &presets, currency)?;

        match selection {
            PresetSelection::Existing(preset) => {
                let item_number = (items.len() + 1) as u32;
                let item = collect_line_item_details(prompter, &preset, item_number)?;
                items.push(item);
            }
            PresetSelection::CreateNew => {
                let new_preset = preset_creation::collect_new_preset(prompter, &presets)?;
                append_preset(dir, new_preset.clone())?;
                prompter.message(&format!(
                    "Preset \"{}\" saved to invoice_config.yaml",
                    new_preset.key
                ));
                let item_number = (items.len() + 1) as u32;
                let item = collect_line_item_details(prompter, &new_preset, item_number)?;
                presets.push(new_preset);
                items.push(item);
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
    use crate::config::types::{Config, Defaults, PaymentMethod, Recipient, Sender};
    use crate::config::writer::save_config;
    use crate::setup::mock_prompter::{MockPrompter, MockResponse};
    use tempfile::TempDir;

    fn setup_test_dir() -> TempDir {
        let dir = TempDir::new().unwrap();
        let config = Config {
            sender: Some(Sender {
                name: "Alice Smith".into(),
                address: vec!["42 Elm Street".into(), "Springfield, IL 62704".into()],
                email: "alice@example.com".into(),
            }),
            recipient: Some(Recipient {
                key: None,
                name: "Bob Corp".into(),
                address: vec!["99 Oak Lane".into(), "Shelbyville, IL 62565".into()],
                company_id: Some("BC-98765".into()),
                vat_number: Some("CZ12345678".into()),
            }),
            recipients: None,
            default_recipient: None,
            payment: Some(vec![PaymentMethod {
                label: "SEPA Transfer".into(),
                iban: "DE89370400440532013000".into(),
                bic_swift: "COBADEFFXXX".into(),
            }]),
            presets: Some(vec![Preset {
                key: "dev".into(),
                description: "Software development".into(),
                default_rate: 800.0,
            }]),
            defaults: Some(Defaults {
                currency: "EUR".into(),
                invoice_date_day: 9,
                payment_terms_days: 30,
            }),
        };
        save_config(dir.path(), &config).unwrap();
        dir
    }

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
        let dir = setup_test_dir();
        let presets = make_presets();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(1),          // select preset #1
            MockResponse::F64(10.0),       // days
            MockResponse::F64(800.0),      // rate
            MockResponse::Confirm(false),  // add another? no
        ]);

        // Act
        let items = collect_all_line_items(&prompter, &presets, "EUR", dir.path()).unwrap();

        // Assert
        assert_eq!(items.len(), 1);
        assert!((items[0].amount - 8000.0).abs() < f64::EPSILON);
        prompter.assert_exhausted();
    }

    #[test]
    fn collect_all_two_items_then_decline() {
        // Arrange
        let dir = setup_test_dir();
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
        let items = collect_all_line_items(&prompter, &presets, "EUR", dir.path()).unwrap();

        // Assert
        assert_eq!(items.len(), 2);
        assert!((items[0].amount - 8000.0).abs() < f64::EPSILON);
        assert!((items[1].amount - 5000.0).abs() < f64::EPSILON);
        prompter.assert_exhausted();
    }

    #[test]
    fn collect_all_three_items_in_order() {
        // Arrange
        let dir = setup_test_dir();
        let presets = make_presets();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(1), MockResponse::F64(1.0), MockResponse::F64(100.0), MockResponse::Confirm(true),
            MockResponse::U32(1), MockResponse::F64(2.0), MockResponse::F64(200.0), MockResponse::Confirm(true),
            MockResponse::U32(1), MockResponse::F64(3.0), MockResponse::F64(300.0), MockResponse::Confirm(false),
        ]);

        // Act
        let items = collect_all_line_items(&prompter, &presets, "EUR", dir.path()).unwrap();

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
        let dir = setup_test_dir();
        let presets = make_presets();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(1), MockResponse::F64(10.0), MockResponse::F64(800.0), MockResponse::Confirm(true),
            MockResponse::U32(1), MockResponse::F64(5.0), MockResponse::F64(800.0), MockResponse::Confirm(false),
        ]);

        // Act
        let _ = collect_all_line_items(&prompter, &presets, "EUR", dir.path()).unwrap();

        // Assert
        let messages = prompter.messages.borrow();
        let all = messages.join("\n");
        assert!(all.contains("Line item #1"), "Expected 'Line item #1' in messages");
        assert!(all.contains("Line item #2"), "Expected 'Line item #2' in messages");
    }

    #[test]
    fn collect_all_different_presets_per_item() {
        // Arrange
        let dir = setup_test_dir();
        let presets = make_presets();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(1), MockResponse::F64(10.0), MockResponse::F64(800.0), MockResponse::Confirm(true),
            MockResponse::U32(2), MockResponse::F64(5.0), MockResponse::F64(1000.0), MockResponse::Confirm(false),
        ]);

        // Act
        let items = collect_all_line_items(&prompter, &presets, "EUR", dir.path()).unwrap();

        // Assert
        assert_eq!(items[0].description, "Software development");
        assert_eq!(items[1].description, "Technical consulting");
    }

    #[test]
    fn collect_all_create_new_creates_and_collects_item() {
        // Arrange
        let dir = setup_test_dir();
        let presets = make_presets(); // has "dev" and "consulting"
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(3),                      // select "Create new" (2 presets + 1)
            MockResponse::Text("design".into()),       // key
            MockResponse::Text("Design work".into()),  // description
            MockResponse::F64(500.0),                  // rate
            MockResponse::F64(5.0),                    // days worked
            MockResponse::F64(500.0),                  // rate (accept default)
            MockResponse::Confirm(false),              // add another? no
        ]);

        // Act
        let items = collect_all_line_items(&prompter, &presets, "EUR", dir.path()).unwrap();

        // Assert
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].description, "Design work");
        assert!((items[0].amount - 2500.0).abs() < f64::EPSILON);
        let messages = prompter.messages.borrow();
        let all = messages.join("\n");
        assert!(all.contains("Preset \"design\" saved to invoice_config.yaml"));
        prompter.assert_exhausted();
    }

    #[test]
    fn collect_all_create_new_preset_appears_in_subsequent_selection() {
        // Arrange
        let dir = setup_test_dir();
        let presets = make_presets(); // "dev", "consulting"
        let prompter = MockPrompter::new(vec![
            // Item 1: create new preset "design"
            MockResponse::U32(3),                      // Create new
            MockResponse::Text("design".into()),
            MockResponse::Text("Design work".into()),
            MockResponse::F64(500.0),
            MockResponse::F64(5.0),                    // days
            MockResponse::F64(500.0),                  // rate
            MockResponse::Confirm(true),               // add another? yes
            // Item 2: select "design" which is now preset #3 in the list
            MockResponse::U32(3),                      // select preset #3 (design)
            MockResponse::F64(2.0),                    // days
            MockResponse::F64(500.0),                  // rate
            MockResponse::Confirm(false),              // add another? no
        ]);

        // Act
        let items = collect_all_line_items(&prompter, &presets, "EUR", dir.path()).unwrap();

        // Assert
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].description, "Design work");
        assert_eq!(items[1].description, "Design work");
        prompter.assert_exhausted();
    }

    #[test]
    fn collect_all_create_new_then_existing_two_items() {
        // Arrange
        let dir = setup_test_dir();
        let presets = make_presets(); // "dev" (800), "consulting" (1000)
        let prompter = MockPrompter::new(vec![
            // Item 1: create new preset "ops"
            MockResponse::U32(3),                      // Create new
            MockResponse::Text("ops".into()),
            MockResponse::Text("Ops work".into()),
            MockResponse::F64(300.0),
            MockResponse::F64(10.0),                   // days
            MockResponse::F64(300.0),                  // rate
            MockResponse::Confirm(true),               // add another? yes
            // Item 2: select existing "dev" (preset #1)
            MockResponse::U32(1),
            MockResponse::F64(5.0),                    // days
            MockResponse::F64(800.0),                  // rate
            MockResponse::Confirm(false),              // no more
        ]);

        // Act
        let items = collect_all_line_items(&prompter, &presets, "EUR", dir.path()).unwrap();

        // Assert
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].description, "Ops work");
        assert!((items[0].amount - 3000.0).abs() < f64::EPSILON);
        assert_eq!(items[1].description, "Software development");
        assert!((items[1].amount - 4000.0).abs() < f64::EPSILON);
        prompter.assert_exhausted();
    }

    #[test]
    fn collect_all_create_new_persists_to_disk() {
        // Arrange
        let dir = setup_test_dir();
        let presets = make_presets();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(3),                       // Create new
            MockResponse::Text("design".into()),
            MockResponse::Text("Design work".into()),
            MockResponse::F64(500.0),
            MockResponse::F64(5.0),
            MockResponse::F64(500.0),
            MockResponse::Confirm(false),
        ]);

        // Act
        collect_all_line_items(&prompter, &presets, "EUR", dir.path()).unwrap();

        // Assert — verify preset was persisted to disk
        use crate::config::loader::{load_config, LoadResult};
        let config = match load_config(dir.path()).unwrap() {
            LoadResult::Loaded(c) => c,
            LoadResult::NotFound => panic!("Config file should exist"),
        };
        let presets_on_disk = config.presets.unwrap();
        assert!(presets_on_disk.iter().any(|p| p.key == "design"));
        prompter.assert_exhausted();
    }

    #[test]
    fn collect_all_create_new_preserves_existing_presets_on_disk() {
        // Arrange
        let dir = setup_test_dir();
        let presets = make_presets();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(3),
            MockResponse::Text("design".into()),
            MockResponse::Text("Design work".into()),
            MockResponse::F64(500.0),
            MockResponse::F64(5.0),
            MockResponse::F64(500.0),
            MockResponse::Confirm(false),
        ]);

        // Act
        collect_all_line_items(&prompter, &presets, "EUR", dir.path()).unwrap();

        // Assert — original "dev" preset still present
        use crate::config::loader::{load_config, LoadResult};
        let config = match load_config(dir.path()).unwrap() {
            LoadResult::Loaded(c) => c,
            LoadResult::NotFound => panic!("Config file should exist"),
        };
        let presets_on_disk = config.presets.unwrap();
        assert!(presets_on_disk.iter().any(|p| p.key == "dev"), "Original preset should still exist");
        prompter.assert_exhausted();
    }

    #[test]
    fn collect_all_preserves_amounts() {
        // Arrange
        let dir = setup_test_dir();
        let presets = make_presets();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(1), MockResponse::F64(12.5), MockResponse::F64(800.0), MockResponse::Confirm(true),
            MockResponse::U32(2), MockResponse::F64(3.0), MockResponse::F64(1500.0), MockResponse::Confirm(false),
        ]);

        // Act
        let items = collect_all_line_items(&prompter, &presets, "EUR", dir.path()).unwrap();

        // Assert
        assert!((items[0].amount - 10000.0).abs() < f64::EPSILON);
        assert!((items[1].amount - 4500.0).abs() < f64::EPSILON);
    }
}
