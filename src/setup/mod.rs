pub mod defaults;
pub mod payment;
pub mod presets;
pub mod prompter;
pub mod recipient;
pub mod sender;
pub mod summary;

#[cfg(test)]
pub mod mock_prompter;
#[cfg(test)]
pub mod test_helpers;

use std::path::Path;
use crate::config::types::Config;
use crate::config::validator::ConfigSection;
use crate::error::AppError;
use self::prompter::Prompter;

/// Run the first-time setup wizard, or resume from where a previous run left off.
pub fn run_setup(
    prompter: &dyn Prompter,
    config: &mut Config,
    missing: &[ConfigSection],
    dir: &Path,
) -> Result<(), AppError> {
    // Show appropriate header
    let is_fresh = missing.len() == 4
        && missing.contains(&ConfigSection::Sender)
        && missing.contains(&ConfigSection::Recipient)
        && missing.contains(&ConfigSection::Payment)
        && missing.contains(&ConfigSection::Presets);

    if is_fresh {
        prompter.message("INVOICE GENERATOR — First-time setup");
    } else {
        prompter.message("INVOICE GENERATOR — Resuming setup...");
        prompter.message(&format!("Continuing from: {}", missing[0]));
    }

    // Dispatch to collectors for missing sections
    for section in missing {
        match section {
            ConfigSection::Sender => sender::collect_sender(prompter, config, dir)?,
            ConfigSection::Recipient => recipient::collect_recipient(prompter, config, dir)?,
            ConfigSection::Payment => payment::collect_payment(prompter, config, dir)?,
            ConfigSection::Presets => presets::collect_presets(prompter, config, dir)?,
        }
    }

    // Defaults are not in ConfigSection (always have a fallback),
    // but we still prompt during setup if not already set.
    if config.defaults.is_none() {
        defaults::collect_defaults(prompter, config, dir)?;
    }

    // Display summary
    summary::display_summary(prompter, config);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::mock_prompter::{MockPrompter, MockResponse};
    use crate::setup::test_helpers::*;

    #[test]
    fn test_run_setup_fresh_start_displays_first_run_header() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let all_missing = vec![
            ConfigSection::Sender, ConfigSection::Recipient,
            ConfigSection::Payment, ConfigSection::Presets,
        ];
        let prompter = MockPrompter::new(full_setup_responses());

        // Act
        run_setup(&prompter, &mut config, &all_missing, dir.path()).unwrap();

        // Assert
        let messages = prompter.messages.borrow();
        assert!(
            messages.iter().any(|m| m.contains("First-time setup")),
            "Expected first-run header, got: {messages:?}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn test_run_setup_resume_displays_resuming_header() {
        // Arrange
        let mut config = config_with_sender();
        let dir = setup_dir(Some(&config));
        let missing = vec![
            ConfigSection::Recipient, ConfigSection::Payment, ConfigSection::Presets,
        ];
        let prompter = MockPrompter::new(resume_from_recipient_responses());

        // Act
        run_setup(&prompter, &mut config, &missing, dir.path()).unwrap();

        // Assert
        let messages = prompter.messages.borrow();
        assert!(
            !messages.iter().any(|m| m.contains("First-time")),
            "Should NOT show first-time header on resume"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn test_run_setup_resume_skips_completed_sections() {
        // Arrange
        let mut config = Config {
            sender: Some(synthetic_sender()),
            recipient: Some(synthetic_recipient()),
            ..Config::default()
        };
        let dir = setup_dir(Some(&config));
        let missing = vec![ConfigSection::Payment, ConfigSection::Presets];
        // Only payment + presets + defaults responses needed
        let prompter = MockPrompter::new(vec![
            // Payment (1 method)
            MockResponse::U32(1),
            MockResponse::Text("SEPA".into()),
            MockResponse::Text("DE00".into()),
            MockResponse::Text("BIC".into()),
            // Presets (1 preset)
            MockResponse::Text("dev".into()),
            MockResponse::Text("Dev".into()),
            MockResponse::F64(100.0),
            MockResponse::Confirm(false),
            // Defaults
            MockResponse::Text("EUR".into()),
            MockResponse::U32(9),
            MockResponse::U32(30),
            MockResponse::Text("leda".into()),  // template
            MockResponse::Text("en-US".into()), // locale
        ]);

        // Act
        run_setup(&prompter, &mut config, &missing, dir.path()).unwrap();

        // Assert
        assert_eq!(config.sender.as_ref().unwrap().name, "Alice Smith");
        assert_eq!(config.recipient.as_ref().unwrap().name, "Bob Corp");
        assert!(config.payment.is_some());
        assert!(config.presets.is_some());
        assert!(config.defaults.is_some());
        prompter.assert_exhausted();
    }

    #[test]
    fn test_run_setup_full_run_populates_all_sections() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let all_missing = vec![
            ConfigSection::Sender, ConfigSection::Recipient,
            ConfigSection::Payment, ConfigSection::Presets,
        ];
        let prompter = MockPrompter::new(full_setup_responses());

        // Act
        run_setup(&prompter, &mut config, &all_missing, dir.path()).unwrap();

        // Assert
        assert!(config.sender.is_some());
        assert!(config.recipients.is_some());
        assert!(config.payment.is_some());
        assert!(config.presets.is_some());
        assert!(config.defaults.is_some());
        prompter.assert_exhausted();
    }

    #[test]
    fn test_run_setup_always_collects_defaults() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let all_missing = vec![
            ConfigSection::Sender, ConfigSection::Recipient,
            ConfigSection::Payment, ConfigSection::Presets,
        ];
        let prompter = MockPrompter::new(full_setup_responses());

        // Act
        run_setup(&prompter, &mut config, &all_missing, dir.path()).unwrap();

        // Assert — defaults collected even though not in missing
        assert!(config.defaults.is_some());
        let defaults = config.defaults.unwrap();
        assert_eq!(defaults.currency, "EUR");
        prompter.assert_exhausted();
    }
}
