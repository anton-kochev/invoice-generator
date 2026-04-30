use crate::config::types::Preset;
use crate::error::AppError;
use crate::setup::prompter::Prompter;
use crate::setup::prompts::prompt_until_valid;

/// Interactively collect a new preset from the user.
///
/// Prompts for a unique key (checked against `existing_presets`),
/// a description, and a default daily rate. The duplicate-key
/// check is case-sensitive. Returns the assembled [`Preset`]
/// without any filesystem side effects.
pub fn collect_new_preset(
    prompter: &dyn Prompter,
    existing_presets: &[Preset],
) -> Result<Preset, AppError> {
    let key = prompt_until_valid(
        prompter,
        |p| p.required_text("Short key (e.g. 'dev'):"),
        |candidate: &String| {
            if existing_presets.iter().any(|p| &p.key == candidate) {
                Err(format!("Key \"{candidate}\" already exists. Choose another:"))
            } else {
                Ok(())
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
            key: "dev".into(),
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
        assert_eq!(preset.key, "analytics");
        assert_eq!(preset.description, "Data analytics");
        assert!((preset.default_rate - 750.0).abs() < f64::EPSILON);
        prompter.assert_exhausted();
    }

    #[test]
    fn rejects_duplicate_key_and_reprompts() {
        // Arrange
        let existing = vec![Preset {
            key: "dev".into(),
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
        assert_eq!(preset.key, "backend");
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
                key: "dev".into(),
                description: "Development".into(),
                default_rate: 800.0,
                currency: None,
                tax_rate: None,
            },
            Preset {
                key: "qa".into(),
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
        assert_eq!(preset.key, "ops");
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
    fn duplicate_check_is_case_sensitive() {
        // Arrange
        let existing = vec![Preset {
            key: "dev".into(),
            description: "Development".into(),
            default_rate: 800.0,
            currency: None,
            tax_rate: None,
        }];
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("Dev".into()),
            MockResponse::Text("Dev services".into()),
            MockResponse::F64(800.0),
        ]);

        // Act
        let preset = collect_new_preset(&prompter, &existing).unwrap();

        // Assert
        assert_eq!(preset.key, "Dev");
        let messages = prompter.messages.borrow();
        let rejection_count = messages
            .iter()
            .filter(|m| m.contains("already exists"))
            .count();
        assert_eq!(
            rejection_count, 0,
            "Expected 0 rejection messages, got {rejection_count}"
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
        assert_eq!(preset.key, "first");
        assert_eq!(preset.description, "First preset");
        assert!((preset.default_rate - 100.0).abs() < f64::EPSILON);
        prompter.assert_exhausted();
    }
}
