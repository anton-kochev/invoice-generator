use std::path::Path;

use crate::config::types::Config;
use super::prompter::Prompter;

/// Display a summary of the completed setup configuration.
pub fn display_summary(prompter: &dyn Prompter, config: &Config, config_path: &Path) {
    prompter.message("\n===== Setup Complete =====\n");

    if let Some(sender) = &config.sender {
        prompter.message(&format!("Sender:         {}", sender.name));
    }
    if let Some(recipient) = &config.recipient {
        prompter.message(&format!("Client:         {}", recipient.name));
    }
    if let Some(presets) = &config.presets {
        prompter.message(&format!("Presets:        {} defined", presets.len()));
        for p in presets {
            prompter.message(&format!("  - {} ({:.2}/day)", p.key, p.default_rate));
        }
    }
    if let Some(defaults) = &config.defaults {
        prompter.message(&format!("Payment terms:  {} days", defaults.payment_terms_days));
        prompter.message(&format!("Currency:       {}", defaults.currency));
    }

    prompter.message(&format!(
        "\nYou can edit these anytime in {}.",
        config_path.display()
    ));
    prompter.message("Proceeding to invoice generation...");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::mock_prompter::MockPrompter;
    use crate::setup::test_helpers::*;

    fn fake_config_path() -> std::path::PathBuf {
        std::path::PathBuf::from("/tmp/invoice-generator/config.yaml")
    }

    #[test]
    fn test_display_summary_includes_sender_name() {
        // Arrange
        let config = complete_config();
        let prompter = MockPrompter::new(vec![]);

        // Act
        display_summary(&prompter, &config, &fake_config_path());

        // Assert
        let messages = prompter.messages.borrow();
        let output = messages.join("\n");
        assert!(output.contains("Alice Smith"), "Should include sender name, got: {output}");
    }

    #[test]
    fn test_display_summary_includes_recipient_name() {
        // Arrange
        let config = complete_config();
        let prompter = MockPrompter::new(vec![]);

        // Act
        display_summary(&prompter, &config, &fake_config_path());

        // Assert
        let messages = prompter.messages.borrow();
        let output = messages.join("\n");
        assert!(output.contains("Bob Corp"), "Should include recipient name, got: {output}");
    }

    #[test]
    fn test_display_summary_includes_preset_details() {
        // Arrange
        let config = complete_config();
        let prompter = MockPrompter::new(vec![]);

        // Act
        display_summary(&prompter, &config, &fake_config_path());

        // Assert
        let messages = prompter.messages.borrow();
        let output = messages.join("\n");
        assert!(output.contains("dev"), "Should include preset key");
        assert!(output.contains("100"), "Should include rate");
    }

    #[test]
    fn test_display_summary_includes_payment_terms() {
        // Arrange
        let config = complete_config();
        let prompter = MockPrompter::new(vec![]);

        // Act
        display_summary(&prompter, &config, &fake_config_path());

        // Assert
        let messages = prompter.messages.borrow();
        let output = messages.join("\n");
        assert!(output.contains("14"), "Should include payment terms days (14 from synthetic_defaults)");
    }

    #[test]
    fn test_display_summary_includes_edit_hint_and_proceed() {
        // Arrange
        let config = complete_config();
        let prompter = MockPrompter::new(vec![]);

        // Act
        display_summary(&prompter, &config, &fake_config_path());

        // Assert
        let messages = prompter.messages.borrow();
        let output = messages.join("\n");
        assert!(output.contains("config.yaml"), "Should mention config file");
        assert!(output.contains("roceeding"), "Should mention proceeding");
    }
}
