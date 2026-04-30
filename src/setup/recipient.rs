use std::path::Path;

use crate::config::types::{Config, Recipient};
use crate::config::writer::save_config;
use crate::error::AppError;
use super::prompter::Prompter;

/// Collect recipient information interactively and persist it to disk.
///
/// Writes v2 format: pushes the recipient into `config.recipients` and sets
/// `config.default_recipient` to the chosen key.
pub fn collect_recipient(
    prompter: &dyn Prompter,
    config: &mut Config,
    config_path: &Path,
) -> Result<(), AppError> {
    prompter.message("\n--- Recipient Information ---\n");

    let key = prompter.required_text("Recipient key (short identifier):")?;
    let name = prompter.required_text("Company name:")?;
    let address = prompter.multi_line("Address")?;
    let company_id = prompter.optional_text("Company ID (blank to skip):")?;
    let vat_number = prompter.optional_text("VAT number (blank to skip):")?;

    let recipient = Recipient {
        key: Some(key.clone()),
        name,
        address,
        company_id,
        vat_number,
    };

    let mut recipients = config.recipients.take().unwrap_or_default();
    recipients.push(recipient);
    config.recipients = Some(recipients);
    config.default_recipient = Some(key);
    config.recipient = None;

    save_config(config_path, config)?;

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
            MockResponse::Text("acme".into()),
            MockResponse::Text("Acme Corp".into()),
            MockResponse::Lines(vec!["1 Industry Ave".into()]),
            MockResponse::OptionalText(Some("AC-12345".into())),
            MockResponse::OptionalText(Some("CZ9999".into())),
        ]);

        // Act
        collect_recipient(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let r = &config.recipients.as_ref().unwrap()[0];
        assert_eq!(r.name, "Acme Corp");
        assert_eq!(r.key, Some("acme".into()));
        assert_eq!(r.company_id, Some("AC-12345".into()));
        assert_eq!(r.vat_number, Some("CZ9999".into()));
        assert_eq!(config.default_recipient, Some("acme".into()));
        assert!(config.recipient.is_none());
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_recipient_optional_fields_skipped() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("acme".into()),
            MockResponse::Text("Acme Corp".into()),
            MockResponse::Lines(vec!["Street".into()]),
            MockResponse::OptionalText(None),
            MockResponse::OptionalText(None),
        ]);

        // Act
        collect_recipient(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let r = &config.recipients.as_ref().unwrap()[0];
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
            MockResponse::Text("acme".into()),
            MockResponse::Text("Acme".into()),
            MockResponse::Lines(vec!["Street".into()]),
            MockResponse::OptionalText(Some("ID-999".into())),
            MockResponse::OptionalText(None),
        ]);

        // Act
        collect_recipient(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let r = &config.recipients.as_ref().unwrap()[0];
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
            MockResponse::Text("acme".into()),
            MockResponse::Text("Acme Corp".into()),
            MockResponse::Lines(vec!["Street".into()]),
            MockResponse::OptionalText(None),
            MockResponse::OptionalText(None),
        ]);

        // Act
        collect_recipient(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        assert_eq!(loaded.recipients.as_ref().unwrap()[0].name, "Acme Corp");
        assert_eq!(loaded.default_recipient, Some("acme".into()));
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_recipient_displays_section_header() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("acme".into()),
            MockResponse::Text("Acme".into()),
            MockResponse::Lines(vec!["Street".into()]),
            MockResponse::OptionalText(None),
            MockResponse::OptionalText(None),
        ]);

        // Act
        collect_recipient(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let messages = prompter.messages.borrow();
        assert!(
            messages.iter().any(|m| m.contains("Recipient")),
            "Expected 'Recipient' in messages, got: {messages:?}"
        );
        prompter.assert_exhausted();
    }

    // ── Story 7.1 Phase 7: v2 format test ──

    #[test]
    fn test_collect_recipient_creates_v2_format() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("acme".into()),
            MockResponse::Text("Acme Corp".into()),
            MockResponse::Lines(vec!["1 Industry Ave".into()]),
            MockResponse::OptionalText(None),
            MockResponse::OptionalText(None),
        ]);

        // Act
        collect_recipient(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        assert!(config.recipient.is_none(), "v1 recipient should be None");
        let recipients = config.recipients.as_ref().unwrap();
        assert_eq!(recipients.len(), 1);
        assert_eq!(recipients[0].key, Some("acme".into()));
        assert_eq!(recipients[0].name, "Acme Corp");
        assert_eq!(config.default_recipient, Some("acme".into()));
        prompter.assert_exhausted();
    }
}
