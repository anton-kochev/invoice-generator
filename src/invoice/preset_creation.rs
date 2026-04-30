use crate::config::types::Preset;
use crate::domain::PresetKey;
use crate::error::AppError;
use crate::setup::prompter::Prompter;
use crate::setup::prompts::prompt_parsed;

/// Interactively collect a new preset from the user.
///
/// Prompts for a unique key (validated and checked against `existing_presets`),
/// a description, and a default daily rate. The duplicate-key
/// check is case-sensitive. Returns the assembled [`Preset`]
/// without any filesystem side effects.
pub fn collect_new_preset(
    prompter: &dyn Prompter,
    existing_presets: &[Preset],
) -> Result<Preset, AppError> {
    let key = prompt_parsed(
        prompter,
        |p| p.required_text("Short key (e.g. 'dev'):"),
        |raw: String| {
            let candidate = PresetKey::try_new(raw).map_err(|e| e.to_string())?;
            if existing_presets.iter().any(|p| p.key == candidate) {
                Err(format!(
                    "Key \"{}\" already exists. Choose another:",
                    candidate.as_str()
                ))
            } else {
                Ok(candidate)
            }
        },
    )?;

    let description = prompter.required_text("Description:")?;
    let default_rate = prompter.positive_f64("Default daily rate:")?;

    Ok(Preset {
        key,
        description,
        default_rate,
        currency: None,
        tax_rate: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::mock_prompter::{MockPrompter, MockResponse};

    #[test]
    fn collects_key_description_rate_returns_preset() {
        // Arrange
        let existing = vec![Preset {
            key: PresetKey::try_new("dev").unwrap(),
            description: "Development".into(),
            default_rate: 800.0,
            currency: None,
            tax_rate: None,
        }];
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("analytics".into()),
            MockResponse::Text("Data analytics".into()),
            MockResponse::F64(750.0),
        ]);

        // Act
        let preset = collect_new_preset(&prompter, &existing).unwrap();

        // Assert
        assert_eq!(preset.key.as_str(), "analytics");
        assert_eq!(preset.description, "Data analytics");
        assert!((preset.default_rate - 750.0).abs() < f64::EPSILON);
        prompter.assert_exhausted();
    }

    #[test]
    fn rejects_duplicate_key_and_reprompts() {
        // Arrange
        let existing = vec![Preset {
            key: PresetKey::try_new("dev").unwrap(),
            description: "Development".into(),
            default_rate: 800.0,
            currency: None,
            tax_rate: None,
        }];
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("dev".into()),
            MockResponse::Text("backend".into()),
            MockResponse::Text("Backend work".into()),
            MockResponse::F64(900.0),
        ]);

        // Act
        let preset = collect_new_preset(&prompter, &existing).unwrap();

        // Assert
        assert_eq!(preset.key.as_str(), "backend");
        let messages = prompter.messages.borrow();
        assert!(
            messages
                .iter()
                .any(|m| m.contains("Key \"dev\" already exists")),
            "Expected duplicate-key message, got: {messages:?}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn rejects_duplicate_key_twice_then_accepts() {
        // Arrange
        let existing = vec![
            Preset {
                key: PresetKey::try_new("dev").unwrap(),
                description: "Development".into(),
                default_rate: 800.0,
                currency: None,
                tax_rate: None,
            },
            Preset {
                key: PresetKey::try_new("qa").unwrap(),
                description: "QA".into(),
                default_rate: 600.0,
                currency: None,
                tax_rate: None,
            },
        ];
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("dev".into()),
            MockResponse::Text("qa".into()),
            MockResponse::Text("ops".into()),
            MockResponse::Text("Operations".into()),
            MockResponse::F64(500.0),
        ]);

        // Act
        let preset = collect_new_preset(&prompter, &existing).unwrap();

        // Assert
        assert_eq!(preset.key.as_str(), "ops");
        let messages = prompter.messages.borrow();
        let rejection_count = messages
            .iter()
            .filter(|m| m.contains("already exists"))
            .count();
        assert_eq!(
            rejection_count, 2,
            "Expected 2 rejection messages, got {rejection_count}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn uppercase_key_rejected_by_validation_then_lowercase_accepted() {
        // Arrange — what used to be a "case-sensitive duplicate" test is now
        // a validation test: `PresetKey` only accepts lowercase ASCII, so
        // entering "Dev" is rejected and the user retries with "ops".
        let existing = vec![Preset {
            key: PresetKey::try_new("dev").unwrap(),
            description: "Development".into(),
            default_rate: 800.0,
            currency: None,
            tax_rate: None,
        }];
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("Dev".into()),  // rejected: uppercase
            MockResponse::Text("ops".into()),  // accepted
            MockResponse::Text("Ops services".into()),
            MockResponse::F64(800.0),
        ]);

        // Act
        let preset = collect_new_preset(&prompter, &existing).unwrap();

        // Assert
        assert_eq!(preset.key.as_str(), "ops");
        let messages = prompter.messages.borrow();
        assert!(
            messages.iter().any(|m| m.contains("invalid characters")),
            "Expected validation error message, got: {messages:?}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn with_no_existing_presets_accepts_any_key() {
        // Arrange
        let existing: Vec<Preset> = vec![];
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("first".into()),
            MockResponse::Text("First preset".into()),
            MockResponse::F64(100.0),
        ]);

        // Act
        let preset = collect_new_preset(&prompter, &existing).unwrap();

        // Assert
        assert_eq!(preset.key.as_str(), "first");
        assert_eq!(preset.description, "First preset");
        assert!((preset.default_rate - 100.0).abs() < f64::EPSILON);
        prompter.assert_exhausted();
    }
}
