use std::path::Path;

use crate::config::types::{Config, Sender};
use crate::config::writer::save_config;
use crate::error::AppError;
use super::prompter::Prompter;

/// Collect sender information interactively and persist it to disk.
pub fn collect_sender(
    prompter: &dyn Prompter,
    config: &mut Config,
    dir: &Path,
) -> Result<(), AppError> {
    prompter.message("\n--- Sender Information ---\n");

    let name = prompter.required_text("Full name:")?;
    let address = prompter.multi_line("Address")?;
    let email = prompter.required_text("Email:")?;

    config.sender = Some(Sender {
        name,
        address,
        email,
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
        collect_sender(&prompter, &mut config, dir.path()).unwrap();

        // Assert
        let sender = config.sender.as_ref().unwrap();
        assert_eq!(sender.name, "Alice Smith");
        assert_eq!(sender.address, vec!["42 Elm St"]);
        assert_eq!(sender.email, "alice@example.com");

        let loaded = unwrap_loaded(load_config(dir.path()));
        assert_eq!(loaded.sender.unwrap().name, "Alice Smith");

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
        collect_sender(&prompter, &mut config, dir.path()).unwrap();

        // Assert
        let sender = config.sender.unwrap();
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
        collect_sender(&prompter, &mut config, dir.path()).unwrap();

        // Assert
        let sender = config.sender.unwrap();
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
        collect_sender(&prompter, &mut config, dir.path()).unwrap();

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
        collect_sender(&prompter, &mut config, dir.path()).unwrap();

        // Assert
        assert!(config.sender.is_some());
        assert!(config.recipient.is_some());
        let loaded = unwrap_loaded(load_config(dir.path()));
        assert_eq!(loaded.recipient.unwrap().name, "Bob Corp");
        prompter.assert_exhausted();
    }
}
