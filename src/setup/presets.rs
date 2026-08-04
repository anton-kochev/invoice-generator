use std::path::Path;
use std::str::FromStr;

use super::prompter::Prompter;
use super::prompts::{prompt_optional_parsed, prompt_parsed};
use crate::config::types::{Config, Preset};
use crate::config::writer::save_config;
use crate::domain::{BillingUnit, Currency, PresetKey};
use crate::error::AppError;

/// Collect invoice presets interactively and persist them to disk.
pub fn collect_presets(
    prompter: &dyn Prompter,
    config: &mut Config,
    config_path: &Path,
) -> Result<(), AppError> {
    prompter.message("\n--- Presets ---\n");

    let mut presets = Vec::new();
    let mut count = 1;

    loop {
        prompter.message(&format!("Preset #{count}:"));

        let key = prompt_parsed(
            prompter,
            |p| p.required_text("Short key (e.g. 'dev'):"),
            |raw: String| PresetKey::try_new(raw).map_err(|e| e.to_string()),
        )?;
        let description = prompter.required_text("Description:")?;
        let default_rate = prompter.positive_f64("Default daily rate:")?;
        let currency = prompt_optional_parsed(
            prompter,
            |p| p.optional_text("Currency  (blank to use default)"),
            |s| {
                Currency::from_str(s.trim()).map_err(|_| {
                    format!(
                        "Unsupported currency. Available: {}",
                        Currency::ALL
                            .iter()
                            .map(|c| c.code())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                })
            },
        )?;

        presets.push(Preset {
            key,
            description,
            default_rate,
            currency,
            tax_rate: None,
            unit: BillingUnit::Day,
        });

        if !prompter.confirm("Add another preset?", false)? {
            break;
        }

        count += 1;
    }

    config.presets = Some(presets);
    save_config(config_path, config)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::loader::load_config;
    use crate::domain::Currency;
    use crate::setup::mock_prompter::{MockPrompter, MockResponse};
    use crate::setup::test_helpers::*;

    #[test]
    fn test_collect_presets_single_preset_decline_more() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("dev".into()),
            MockResponse::Text("Development Services".into()),
            MockResponse::F64(100.0),
            MockResponse::OptionalText(None),
            MockResponse::Confirm(false),
        ]);

        // Act
        collect_presets(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let presets = config.presets.as_ref().unwrap();
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].key.as_str(), "dev");
        assert_eq!(presets[0].description, "Development Services");
        assert_eq!(presets[0].default_rate, 100.0);
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_presets_two_presets_via_add_another() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("dev".into()),
            MockResponse::Text("Development".into()),
            MockResponse::F64(100.0),
            MockResponse::OptionalText(None),
            MockResponse::Confirm(true),
            MockResponse::Text("design".into()),
            MockResponse::Text("Design Work".into()),
            MockResponse::F64(80.0),
            MockResponse::OptionalText(None),
            MockResponse::Confirm(false),
        ]);

        // Act
        collect_presets(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let presets = config.presets.unwrap();
        assert_eq!(presets.len(), 2);
        assert_eq!(presets[0].key.as_str(), "dev");
        assert_eq!(presets[1].key.as_str(), "design");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_presets_three_via_loop() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("a".into()),
            MockResponse::Text("A".into()),
            MockResponse::F64(10.0),
            MockResponse::OptionalText(None),
            MockResponse::Confirm(true),
            MockResponse::Text("b".into()),
            MockResponse::Text("B".into()),
            MockResponse::F64(20.0),
            MockResponse::OptionalText(None),
            MockResponse::Confirm(true),
            MockResponse::Text("c".into()),
            MockResponse::Text("C".into()),
            MockResponse::F64(30.0),
            MockResponse::OptionalText(None),
            MockResponse::Confirm(false),
        ]);

        // Act
        collect_presets(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let presets = config.presets.unwrap();
        assert_eq!(presets.len(), 3);
        assert_eq!(presets[2].key.as_str(), "c");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_presets_decimal_rate() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("qa".into()),
            MockResponse::Text("QA".into()),
            MockResponse::F64(99.50),
            MockResponse::OptionalText(None),
            MockResponse::Confirm(false),
        ]);

        // Act
        collect_presets(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let presets = config.presets.unwrap();
        assert!((presets[0].default_rate - 99.50).abs() < f64::EPSILON);
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_presets_persists_to_disk() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("dev".into()),
            MockResponse::Text("Dev".into()),
            MockResponse::F64(100.0),
            MockResponse::OptionalText(None),
            MockResponse::Confirm(false),
        ]);

        // Act
        collect_presets(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        let presets = loaded.presets.unwrap();
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].key.as_str(), "dev");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_presets_currency_provided_sets_currency_some() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("dev".into()),
            MockResponse::Text("Development Services".into()),
            MockResponse::F64(100.0),
            MockResponse::OptionalText(Some("USD".into())),
            MockResponse::Confirm(false),
        ]);

        // Act
        collect_presets(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let presets = config.presets.as_ref().unwrap();
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].currency, Some(Currency::Usd));
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_presets_currency_blank_leaves_currency_none() {
        // Arrange — blank means "inherit Defaults.currency at invoice time".
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("dev".into()),
            MockResponse::Text("Development Services".into()),
            MockResponse::F64(100.0),
            MockResponse::OptionalText(None),
            MockResponse::Confirm(false),
        ]);

        // Act
        collect_presets(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let presets = config.presets.as_ref().unwrap();
        assert_eq!(presets.len(), 1);
        assert!(presets[0].currency.is_none());
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_presets_unsupported_currency_reprompts() {
        // Arrange — CHF is not in the closed Currency set; user is reprompted
        // until a valid code is entered. Mirrors the Defaults reprompt test.
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("dev".into()),
            MockResponse::Text("Development Services".into()),
            MockResponse::F64(100.0),
            MockResponse::OptionalText(Some("CHF".into())),
            MockResponse::OptionalText(Some("EUR".into())),
            MockResponse::Confirm(false),
        ]);

        // Act
        collect_presets(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let presets = config.presets.as_ref().unwrap();
        assert_eq!(presets[0].currency, Some(Currency::Eur));
        let messages = prompter.messages.borrow();
        assert!(
            messages.iter().any(|m| m.contains("Unsupported currency")),
            "Expected 'Unsupported currency' message, got: {messages:?}"
        );
        prompter.assert_exhausted();
    }
}
