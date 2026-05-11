use std::path::Path;

use super::prompter::Prompter;
use crate::config::ConfigError;
use crate::config::types::{Config, Sender};
use crate::config::writer::save_config;
use crate::domain::SenderKey;
use crate::error::AppError;

/// Collect sender information interactively and persist it to disk.
///
/// Pushes the new sender onto `config.senders` (initializing the vec if
/// needed) and sets `config.default_sender` when no default is configured
/// yet. The legacy `config.sender` field is intentionally left untouched —
/// callers loading old v1 configs see the legacy data migrated to v2 by the
/// validator / writer-side `ensure_senders_v2`.
pub fn collect_sender(
    prompter: &dyn Prompter,
    config: &mut Config,
    config_path: &Path,
) -> Result<(), AppError> {
    prompter.message("\n--- Sender Information ---\n");

    let name = prompter.required_text("Full name:")?;
    let address = prompter.multi_line("Address")?;
    let email = prompter.required_text("Email:")?;

    let key = SenderKey::from_name(&name)
        .map_err(|e| AppError::from(ConfigError::InvalidDefaultSender(e.to_string())))?;

    let sender = Sender {
        key: Some(key.clone()),
        name,
        address,
        email,
    };

    let mut senders = config.senders.take().unwrap_or_default();
    senders.push(sender);
    config.senders = Some(senders);

    if config.default_sender.is_none() {
        config.default_sender = Some(key);
    }

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
    fn test_collect_sender_happy_path_saves_to_config_and_disk() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("Alice Smith".into()),
            MockResponse::Lines(vec!["42 Elm St".into()]),
            MockResponse::Text("alice@example.com".into()),
        ]);

        // Act
        collect_sender(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert — in-memory state lands on v2 senders, not legacy field.
        let senders = config.senders.as_ref().unwrap();
        assert_eq!(senders.len(), 1);
        let sender = &senders[0];
        assert_eq!(sender.name, "Alice Smith");
        assert_eq!(sender.address, vec!["42 Elm St"]);
        assert_eq!(sender.email, "alice@example.com");
        assert_eq!(
            sender.key.as_ref().map(|k| k.as_str()),
            Some("alice-smith")
        );
        assert_eq!(
            config.default_sender.as_ref().map(|k| k.as_str()),
            Some("alice-smith")
        );

        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        let loaded_senders = loaded.senders.as_ref().unwrap();
        assert_eq!(loaded_senders[0].name, "Alice Smith");
        assert_eq!(
            loaded.default_sender.as_ref().map(|k| k.as_str()),
            Some("alice-smith")
        );

        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_sender_multiline_address_preserved() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("Bob Jones".into()),
            MockResponse::Lines(vec![
                "123 Main St".into(),
                "Suite 400".into(),
                "NYC, NY 10001".into(),
            ]),
            MockResponse::Text("bob@example.com".into()),
        ]);

        // Act
        collect_sender(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let senders = config.senders.unwrap();
        let sender = &senders[0];
        assert_eq!(sender.address.len(), 3);
        assert_eq!(sender.address[1], "Suite 400");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_sender_single_line_address() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("Carol".into()),
            MockResponse::Lines(vec!["1 Short St".into()]),
            MockResponse::Text("carol@example.com".into()),
        ]);

        // Act
        collect_sender(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let senders = config.senders.unwrap();
        let sender = &senders[0];
        assert_eq!(sender.address.len(), 1);
        assert_eq!(sender.address[0], "1 Short St");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_sender_displays_section_header() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("Alice".into()),
            MockResponse::Lines(vec!["Street".into()]),
            MockResponse::Text("a@b.com".into()),
        ]);

        // Act
        collect_sender(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let messages = prompter.messages.borrow();
        assert!(
            messages.iter().any(|m| m.contains("Sender")),
            "Expected a message containing 'Sender', got: {messages:?}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_sender_preserves_existing_recipient() {
        // Arrange
        let mut config = Config {
            recipient: Some(synthetic_recipient()),
            ..Config::default()
        };
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("Alice".into()),
            MockResponse::Lines(vec!["Street".into()]),
            MockResponse::Text("a@b.com".into()),
        ]);

        // Act
        collect_sender(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        assert!(config.senders.is_some());
        assert!(config.recipient.is_some());
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        assert_eq!(loaded.recipient.unwrap().name, "Bob Corp");
        prompter.assert_exhausted();
    }
}
