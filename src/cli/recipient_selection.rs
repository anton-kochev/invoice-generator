use crate::config::validator::ValidatedRecipient;
use crate::error::AppError;
use crate::setup::prompter::Prompter;
use crate::setup::prompts::prompt_u32_in_range;

/// Select a recipient for the invoice.
///
/// If only one recipient exists, auto-selects it.
/// If multiple exist, shows a numbered list and prompts for selection.
pub fn select_recipient(
    prompter: &dyn Prompter,
    recipients: &[ValidatedRecipient],
    default_key: &str,
) -> Result<ValidatedRecipient, AppError> {
    if recipients.len() == 1 {
        prompter.message(&format!("Using recipient: {}", recipients[0].name));
        return Ok(recipients[0].clone());
    }

    prompter.message("\nSelect a recipient:\n");

    let default_index = recipients
        .iter()
        .position(|r| r.key.as_str() == default_key)
        .map(|i| i + 1)
        .unwrap_or(1) as u32;

    for (i, r) in recipients.iter().enumerate() {
        let marker = if r.key.as_str() == default_key {
            " (default)"
        } else {
            ""
        };
        let addr = r.address.first().map(|a| a.as_str()).unwrap_or("");
        prompter.message(&format!(
            "  [{}] {} \u{2014} {}, {}{}",
            i + 1,
            r.key.as_str(),
            r.name,
            addr,
            marker,
        ));
    }

    let max = recipients.len() as u32;
    let choice = prompt_u32_in_range(
        prompter,
        "Select recipient number:",
        1..=max,
        default_index,
    )?;

    Ok(recipients[choice as usize - 1].clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::mock_prompter::{MockPrompter, MockResponse};
    use crate::setup::test_helpers::*;

    #[test]
    fn test_select_recipient_single_returns_without_prompt() {
        // Arrange
        let recipients = vec![synthetic_validated_acme()];
        let prompter = MockPrompter::new(vec![]);

        // Act
        let result = select_recipient(&prompter, &recipients, "acme").unwrap();

        // Assert
        assert_eq!(result.name, "Acme Corp");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_select_recipient_single_displays_auto_select_message() {
        // Arrange
        let recipients = vec![synthetic_validated_acme()];
        let prompter = MockPrompter::new(vec![]);

        // Act
        select_recipient(&prompter, &recipients, "acme").unwrap();

        // Assert
        let messages = prompter.messages.borrow();
        let all = messages.join("\n");
        assert!(
            all.contains("Using recipient: Acme Corp"),
            "Expected auto-select message, got: {all}"
        );
    }

    #[test]
    fn test_select_recipient_multiple_displays_numbered_list() {
        // Arrange
        let recipients = vec![synthetic_validated_acme(), synthetic_validated_globex()];
        let prompter = MockPrompter::new(vec![MockResponse::U32(1)]);

        // Act
        select_recipient(&prompter, &recipients, "acme").unwrap();

        // Assert
        let messages = prompter.messages.borrow();
        let all = messages.join("\n");
        assert!(all.contains("[1]"), "Expected [1] in messages, got: {all}");
        assert!(all.contains("[2]"), "Expected [2] in messages, got: {all}");
        assert!(
            all.contains("Acme Corp"),
            "Expected 'Acme Corp', got: {all}"
        );
        assert!(
            all.contains("Globex Inc"),
            "Expected 'Globex Inc', got: {all}"
        );
    }

    #[test]
    fn test_select_recipient_marks_default_with_indicator() {
        // Arrange
        let recipients = vec![synthetic_validated_acme(), synthetic_validated_globex()];
        let prompter = MockPrompter::new(vec![MockResponse::U32(1)]);

        // Act
        select_recipient(&prompter, &recipients, "acme").unwrap();

        // Assert
        let messages = prompter.messages.borrow();
        let all = messages.join("\n");
        assert!(
            all.contains("(default)"),
            "Expected '(default)' marker, got: {all}"
        );
    }

    #[test]
    fn test_select_recipient_choice_one_returns_first() {
        // Arrange
        let recipients = vec![synthetic_validated_acme(), synthetic_validated_globex()];
        let prompter = MockPrompter::new(vec![MockResponse::U32(1)]);

        // Act
        let result = select_recipient(&prompter, &recipients, "acme").unwrap();

        // Assert
        assert_eq!(result.name, "Acme Corp");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_select_recipient_choice_two_returns_second() {
        // Arrange
        let recipients = vec![synthetic_validated_acme(), synthetic_validated_globex()];
        let prompter = MockPrompter::new(vec![MockResponse::U32(2)]);

        // Act
        let result = select_recipient(&prompter, &recipients, "acme").unwrap();

        // Assert
        assert_eq!(result.name, "Globex Inc");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_select_recipient_invalid_number_reprompts() {
        // Arrange
        let recipients = vec![synthetic_validated_acme(), synthetic_validated_globex()];
        let prompter = MockPrompter::new(vec![MockResponse::U32(0), MockResponse::U32(1)]);

        // Act
        let result = select_recipient(&prompter, &recipients, "acme").unwrap();

        // Assert
        assert_eq!(result.name, "Acme Corp");
        let messages = prompter.messages.borrow();
        let all = messages.join("\n");
        assert!(
            all.contains("Please enter a number between 1 and 2"),
            "Expected range error, got: {all}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn test_select_recipient_too_high_reprompts() {
        // Arrange
        let recipients = vec![synthetic_validated_acme(), synthetic_validated_globex()];
        let prompter = MockPrompter::new(vec![MockResponse::U32(99), MockResponse::U32(2)]);

        // Act
        let result = select_recipient(&prompter, &recipients, "acme").unwrap();

        // Assert
        assert_eq!(result.name, "Globex Inc");
        prompter.assert_exhausted();
    }

    // ── Story 11.1: v1 backwards compatibility verification ──

    #[test]
    fn test_v1_config_single_recipient_auto_selects_without_prompt() {
        // Arrange — simulate v1 config that was normalized: single recipient with derived key
        let recipient = ValidatedRecipient {
            key: crate::domain::RecipientKey::try_new("bob-corp").unwrap(),
            name: "Bob Corp".into(),
            address: vec!["456 Ave".into()],
            company_id: None,
            vat_number: None,
        };
        let recipients = vec![recipient];
        let prompter = MockPrompter::new(vec![]); // no prompts expected

        // Act
        let result = select_recipient(&prompter, &recipients, "bob-corp").unwrap();

        // Assert
        assert_eq!(result.name, "Bob Corp");
        prompter.assert_exhausted(); // confirms no interactive prompt was needed
    }
}
