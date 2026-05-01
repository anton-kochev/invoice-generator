use crate::config::types::Preset;
use crate::domain::Currency;
use crate::error::AppError;
use crate::setup::prompter::Prompter;
use crate::setup::prompts::prompt_u32_in_range;

use super::currency::effective_currency;
use super::types::PresetSelection;

/// Interactively select a preset for a line item.
///
/// Displays all available presets in a numbered list, plus a
/// "Create new preset" option at the end.
pub fn select_preset(
    prompter: &dyn Prompter,
    presets: &[Preset],
    currency: Currency,
) -> Result<PresetSelection, AppError> {
    prompter.message("\nSelect a preset for this line item:\n");

    for (i, preset) in presets.iter().enumerate() {
        let effective = effective_currency(preset, currency);
        prompter.message(&format!(
            "  [{}] {} \u{2014} {} ({} {:.2}/day)",
            i + 1,
            preset.key,
            preset.description,
            effective,
            preset.default_rate,
        ));
    }

    let max = (presets.len() + 1) as u32;
    prompter.message(&format!("  [{max}] + Create new preset"));

    let choice = prompt_u32_in_range(prompter, "Select preset number:", 1..=max, 1)?;

    if choice == max {
        Ok(PresetSelection::CreateNew)
    } else {
        Ok(PresetSelection::Existing(
            presets[choice as usize - 1].clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::PresetKey;
    use crate::setup::mock_prompter::{MockPrompter, MockResponse};

    fn make_presets() -> Vec<Preset> {
        vec![
            Preset {
                key: PresetKey::try_new("dev").unwrap(),
                description: "Software development".into(),
                default_rate: 800.0,
                currency: None,
                tax_rate: None,
            },
            Preset {
                key: PresetKey::try_new("consulting").unwrap(),
                description: "Technical consulting".into(),
                default_rate: 1000.0,
                currency: None,
                tax_rate: None,
            },
        ]
    }

    #[test]
    fn displays_all_presets_in_numbered_list() {
        // Arrange
        let presets = make_presets();
        let prompter = MockPrompter::new(vec![MockResponse::U32(1)]);

        // Act
        select_preset(&prompter, &presets, Currency::Eur).unwrap();

        // Assert
        let messages = prompter.messages.borrow();
        let all = messages.join("\n");
        assert!(all.contains("[1]"), "Expected [1] in messages, got: {all}");
        assert!(all.contains("[2]"), "Expected [2] in messages, got: {all}");
        assert!(
            all.contains("dev"),
            "Expected 'dev' in messages, got: {all}"
        );
        assert!(
            all.contains("consulting"),
            "Expected 'consulting' in messages, got: {all}"
        );
        assert!(
            all.contains("[3] + Create new preset"),
            "Expected '[3] + Create new preset' in messages, got: {all}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn displays_currency_and_rate() {
        // Arrange
        let presets = vec![Preset {
            key: PresetKey::try_new("design").unwrap(),
            description: "Graphic design".into(),
            default_rate: 500.0,
            currency: None,
            tax_rate: None,
        }];
        let prompter = MockPrompter::new(vec![MockResponse::U32(1)]);

        // Act
        select_preset(&prompter, &presets, Currency::Usd).unwrap();

        // Assert
        let messages = prompter.messages.borrow();
        let all = messages.join("\n");
        assert!(
            all.contains("USD"),
            "Expected 'USD' in messages, got: {all}"
        );
        assert!(
            all.contains("500.00"),
            "Expected '500.00' in messages, got: {all}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn selects_first_preset() {
        // Arrange
        let presets = make_presets();
        let prompter = MockPrompter::new(vec![MockResponse::U32(1)]);

        // Act
        let result = select_preset(&prompter, &presets, Currency::Eur).unwrap();

        // Assert
        assert_eq!(result, PresetSelection::Existing(presets[0].clone()));
        prompter.assert_exhausted();
    }

    #[test]
    fn selects_last_preset() {
        // Arrange
        let presets = make_presets();
        let prompter = MockPrompter::new(vec![MockResponse::U32(2)]);

        // Act
        let result = select_preset(&prompter, &presets, Currency::Eur).unwrap();

        // Assert
        assert_eq!(result, PresetSelection::Existing(presets[1].clone()));
        prompter.assert_exhausted();
    }

    #[test]
    fn selects_create_new() {
        // Arrange
        let presets = make_presets();
        let prompter = MockPrompter::new(vec![MockResponse::U32(3)]);

        // Act
        let result = select_preset(&prompter, &presets, Currency::Eur).unwrap();

        // Assert
        assert_eq!(result, PresetSelection::CreateNew);
        prompter.assert_exhausted();
    }

    #[test]
    fn single_preset_shows_create_new_as_option_two() {
        // Arrange
        let presets = vec![Preset {
            key: PresetKey::try_new("solo").unwrap(),
            description: "Solo work".into(),
            default_rate: 600.0,
            currency: None,
            tax_rate: None,
        }];
        let prompter = MockPrompter::new(vec![MockResponse::U32(2)]);

        // Act
        let result = select_preset(&prompter, &presets, Currency::Eur).unwrap();

        // Assert
        assert_eq!(result, PresetSelection::CreateNew);
        let messages = prompter.messages.borrow();
        let all = messages.join("\n");
        assert!(
            all.contains("[2] + Create new preset"),
            "Expected '[2] + Create new preset' in messages, got: {all}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn reprompts_on_zero() {
        // Arrange
        let presets = make_presets();
        let prompter = MockPrompter::new(vec![MockResponse::U32(0), MockResponse::U32(1)]);

        // Act
        let result = select_preset(&prompter, &presets, Currency::Eur).unwrap();

        // Assert
        assert_eq!(result, PresetSelection::Existing(presets[0].clone()));
        let messages = prompter.messages.borrow();
        let all = messages.join("\n");
        assert!(
            all.contains("1") && all.contains("3"),
            "Expected error message mentioning '1 and 3', got: {all}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn reprompts_on_too_high() {
        // Arrange
        let presets = make_presets();
        let prompter = MockPrompter::new(vec![MockResponse::U32(99), MockResponse::U32(2)]);

        // Act
        let result = select_preset(&prompter, &presets, Currency::Eur).unwrap();

        // Assert
        assert_eq!(result, PresetSelection::Existing(presets[1].clone()));
        prompter.assert_exhausted();
    }

    #[test]
    fn reprompts_on_number_beyond_create_new() {
        // Arrange
        let presets = make_presets();
        let prompter = MockPrompter::new(vec![MockResponse::U32(4), MockResponse::U32(3)]);

        // Act
        let result = select_preset(&prompter, &presets, Currency::Eur).unwrap();

        // Assert
        assert_eq!(result, PresetSelection::CreateNew);
        prompter.assert_exhausted();
    }

    #[test]
    fn reprompts_multiple_invalid_then_valid() {
        // Arrange
        let presets = make_presets();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(0),
            MockResponse::U32(99),
            MockResponse::U32(1),
        ]);

        // Act
        let result = select_preset(&prompter, &presets, Currency::Eur).unwrap();

        // Assert
        assert_eq!(result, PresetSelection::Existing(presets[0].clone()));
        let messages = prompter.messages.borrow();
        let error_count = messages
            .iter()
            .filter(|m| m.contains("Please enter a number between"))
            .count();
        assert_eq!(
            error_count, 2,
            "Expected 2 error messages, got {error_count}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn preserves_preset_data_in_return() {
        // Arrange
        let presets = vec![Preset {
            key: PresetKey::try_new("special").unwrap(),
            description: "Special project work".into(),
            default_rate: 1234.56,
            currency: None,
            tax_rate: None,
        }];
        let prompter = MockPrompter::new(vec![MockResponse::U32(1)]);

        // Act
        let result = select_preset(&prompter, &presets, Currency::Eur).unwrap();

        // Assert
        match result {
            PresetSelection::Existing(p) => {
                assert_eq!(p.key.as_str(), "special");
                assert_eq!(p.description, "Special project work");
                assert!((p.default_rate - 1234.56).abs() < f64::EPSILON);
            }
            PresetSelection::CreateNew => panic!("Expected Existing, got CreateNew"),
        }
        prompter.assert_exhausted();
    }

    #[test]
    fn with_many_presets_shows_all() {
        // Arrange
        let presets: Vec<Preset> = (1..=5)
            .map(|i| Preset {
                key: PresetKey::try_new(format!("preset{i}")).unwrap(),
                description: format!("Preset number {i}"),
                default_rate: i as f64 * 100.0,
                currency: None,
                tax_rate: None,
            })
            .collect();
        let prompter = MockPrompter::new(vec![MockResponse::U32(6)]);

        // Act
        let result = select_preset(&prompter, &presets, Currency::Eur).unwrap();

        // Assert
        assert_eq!(result, PresetSelection::CreateNew);
        let messages = prompter.messages.borrow();
        let all = messages.join("\n");
        assert!(
            all.contains("[6] + Create new preset"),
            "Expected '[6] + Create new preset' in messages, got: {all}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn displays_preset_override_currency() {
        // Arrange
        let presets = vec![Preset {
            key: PresetKey::try_new("dev").unwrap(),
            description: "Development".into(),
            default_rate: 800.0,
            currency: Some(Currency::Usd),
            tax_rate: None,
        }];
        let prompter = MockPrompter::new(vec![MockResponse::U32(1)]);

        // Act
        let _ = select_preset(&prompter, &presets, Currency::Eur);

        // Assert
        let messages = prompter.messages.borrow();
        let display = messages.iter().find(|m| m.contains("dev")).unwrap();
        assert!(display.contains("USD"), "Expected 'USD' in: {display}");
        assert!(
            !display.contains("EUR"),
            "Should not contain 'EUR' in: {display}"
        );
    }

    #[test]
    fn displays_default_currency_when_preset_has_none() {
        // Arrange
        let presets = vec![Preset {
            key: PresetKey::try_new("dev").unwrap(),
            description: "Development".into(),
            default_rate: 800.0,
            currency: None,
            tax_rate: None,
        }];
        let prompter = MockPrompter::new(vec![MockResponse::U32(1)]);

        // Act
        let _ = select_preset(&prompter, &presets, Currency::Eur);

        // Assert
        let messages = prompter.messages.borrow();
        let display = messages.iter().find(|m| m.contains("dev")).unwrap();
        assert!(display.contains("EUR"), "Expected 'EUR' in: {display}");
    }
}
