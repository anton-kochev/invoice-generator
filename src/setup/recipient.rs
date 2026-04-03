use std::path::Path;

use crate::config::types::{Config, Recipient};
use crate::config::writer::save_config;
use crate::error::AppError;
use super::prompter::Prompter;

/// Collect recipient information interactively and persist it to disk.
pub fn collect_recipient(
    prompter: &dyn Prompter,
    config: &mut Config,
    dir: &Path,
) -> Result<(), AppError> {
    prompter.message("\n--- Recipient Information ---\n");

    let name = prompter.required_text("Company name:")?;
    let address = prompter.multi_line("Address")?;
    let company_id = prompter.optional_text("Company ID (blank to skip):")?;
    let vat_number = prompter.optional_text("VAT number (blank to skip):")?;

    config.recipient = Some(Recipient {
        name,
        address,
        company_id,
        vat_number,
    });

    save_config(dir, config)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::loader::load_config;
    use crate::setup::mock_prompter::{MockPrompter, MockResponse};
    use crate::setup::test_helpers::*;

    #[test]
    fn test_collect_recipient_happy_path_all_fields() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("Acme Corp".into()),
            MockResponse::Lines(vec!["1 Industry Ave".into()]),
            MockResponse::OptionalText(Some("AC-12345".into())),
            MockResponse::OptionalText(Some("CZ9999".into())),
        ]);

        // Act
        collect_recipient(&prompter, &mut config, dir.path()).unwrap();

        // Assert
        let r = config.recipient.as_ref().unwrap();
        assert_eq!(r.name, "Acme Corp");
        assert_eq!(r.company_id, Some("AC-12345".into()));
        assert_eq!(r.vat_number, Some("CZ9999".into()));
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_recipient_optional_fields_skipped() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("Acme Corp".into()),
            MockResponse::Lines(vec!["Street".into()]),
            MockResponse::OptionalText(None),
            MockResponse::OptionalText(None),
        ]);

        // Act
        collect_recipient(&prompter, &mut config, dir.path()).unwrap();

        // Assert
        let r = config.recipient.unwrap();
        assert!(r.company_id.is_none());
        assert!(r.vat_number.is_none());
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_recipient_company_id_only() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("Acme".into()),
            MockResponse::Lines(vec!["Street".into()]),
            MockResponse::OptionalText(Some("ID-999".into())),
            MockResponse::OptionalText(None),
        ]);

        // Act
        collect_recipient(&prompter, &mut config, dir.path()).unwrap();

        // Assert
        let r = config.recipient.unwrap();
        assert_eq!(r.company_id, Some("ID-999".into()));
        assert!(r.vat_number.is_none());
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_recipient_persists_to_disk() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("Acme Corp".into()),
            MockResponse::Lines(vec!["Street".into()]),
            MockResponse::OptionalText(None),
            MockResponse::OptionalText(None),
        ]);

        // Act
        collect_recipient(&prompter, &mut config, dir.path()).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(dir.path()));
        assert_eq!(loaded.recipient.unwrap().name, "Acme Corp");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_recipient_displays_section_header() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("Acme".into()),
            MockResponse::Lines(vec!["Street".into()]),
            MockResponse::OptionalText(None),
            MockResponse::OptionalText(None),
        ]);

        // Act
        collect_recipient(&prompter, &mut config, dir.path()).unwrap();

        // Assert
        let messages = prompter.messages.borrow();
        assert!(
            messages.iter().any(|m| m.contains("Recipient")),
            "Expected 'Recipient' in messages, got: {messages:?}"
        );
        prompter.assert_exhausted();
    }
}
