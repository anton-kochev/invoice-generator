//! Interactive sender-selection helper for the invoice flow.
//!
//! Mirrors [`crate::cli::recipient_selection`] for senders: auto-selects when
//! there is only one sender, otherwise displays a numbered list (with the
//! default sender marked) and prompts the user to pick a number.

use crate::config::validator::ValidatedSender;
use crate::error::AppError;
use crate::setup::prompter::Prompter;
use crate::setup::prompts::prompt_u32_in_range;

/// Select a sender for the invoice.
///
/// If only one sender exists, auto-selects it.
/// If multiple exist, shows a numbered list and prompts for selection.
pub fn select_sender(
    prompter: &dyn Prompter,
    senders: &[ValidatedSender],
    default_key: &str,
) -> Result<ValidatedSender, AppError> {
    if senders.len() == 1 {
        prompter.message(&format!("Using sender: {}", senders[0].name()));
        return Ok(senders[0].clone());
    }

    prompter.message("\nSelect a sender:\n");

    let default_index = senders
        .iter()
        .position(|s| s.key().as_str() == default_key)
        .map(|i| i + 1)
        .unwrap_or(1) as u32;

    for (i, s) in senders.iter().enumerate() {
        let marker = if s.key().as_str() == default_key {
            " (default)"
        } else {
            ""
        };
        let addr = s.address().first().map(|a| a.as_str()).unwrap_or("");
        prompter.message(&format!(
            "  [{}] {} \u{2014} {}, {}{}",
            i + 1,
            s.key().as_str(),
            s.name(),
            addr,
            marker,
        ));
    }

    let max = senders.len() as u32;
    let choice = prompt_u32_in_range(prompter, "Select sender number:", 1..=max, default_index)?;

    Ok(senders[choice as usize - 1].clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::mock_prompter::{MockPrompter, MockResponse};
    use crate::setup::test_helpers::*;

    #[test]
    fn test_select_sender_single_returns_without_prompt() {
        // Arrange
        let senders = vec![synthetic_validated_alice()];
        let prompter = MockPrompter::new(vec![]);

        // Act
        let result = select_sender(&prompter, &senders, "alice").unwrap();

        // Assert
        assert_eq!(result.name(), "Alice Smith");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_select_sender_single_displays_auto_select_message() {
        // Arrange
        let senders = vec![synthetic_validated_alice()];
        let prompter = MockPrompter::new(vec![]);

        // Act
        select_sender(&prompter, &senders, "alice").unwrap();

        // Assert
        let messages = prompter.messages.borrow();
        let all = messages.join("\n");
        assert!(
            all.contains("Using sender: Alice Smith"),
            "Expected auto-select message, got: {all}"
        );
    }

    #[test]
    fn test_select_sender_multiple_displays_numbered_list() {
        // Arrange
        let senders = vec![synthetic_validated_alice(), synthetic_validated_bob()];
        let prompter = MockPrompter::new(vec![MockResponse::U32(1)]);

        // Act
        select_sender(&prompter, &senders, "alice").unwrap();

        // Assert
        let messages = prompter.messages.borrow();
        let all = messages.join("\n");
        assert!(all.contains("[1]"), "Expected [1] in messages, got: {all}");
        assert!(all.contains("[2]"), "Expected [2] in messages, got: {all}");
        assert!(
            all.contains("Alice Smith"),
            "Expected 'Alice Smith', got: {all}"
        );
        assert!(
            all.contains("Bob Jones"),
            "Expected 'Bob Jones', got: {all}"
        );
    }

    #[test]
    fn test_select_sender_marks_default_with_indicator() {
        // Arrange
        let senders = vec![synthetic_validated_alice(), synthetic_validated_bob()];
        let prompter = MockPrompter::new(vec![MockResponse::U32(1)]);

        // Act
        select_sender(&prompter, &senders, "alice").unwrap();

        // Assert
        let messages = prompter.messages.borrow();
        let all = messages.join("\n");
        assert!(
            all.contains("(default)"),
            "Expected '(default)' marker, got: {all}"
        );
    }

    #[test]
    fn test_select_sender_choice_one_returns_first() {
        // Arrange
        let senders = vec![synthetic_validated_alice(), synthetic_validated_bob()];
        let prompter = MockPrompter::new(vec![MockResponse::U32(1)]);

        // Act
        let result = select_sender(&prompter, &senders, "alice").unwrap();

        // Assert
        assert_eq!(result.name(), "Alice Smith");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_select_sender_choice_two_returns_second() {
        // Arrange
        let senders = vec![synthetic_validated_alice(), synthetic_validated_bob()];
        let prompter = MockPrompter::new(vec![MockResponse::U32(2)]);

        // Act
        let result = select_sender(&prompter, &senders, "alice").unwrap();

        // Assert
        assert_eq!(result.name(), "Bob Jones");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_select_sender_invalid_number_reprompts() {
        // Arrange
        let senders = vec![synthetic_validated_alice(), synthetic_validated_bob()];
        let prompter = MockPrompter::new(vec![MockResponse::U32(0), MockResponse::U32(1)]);

        // Act
        let result = select_sender(&prompter, &senders, "alice").unwrap();

        // Assert
        assert_eq!(result.name(), "Alice Smith");
        let messages = prompter.messages.borrow();
        let all = messages.join("\n");
        assert!(
            all.contains("Please enter a number between 1 and 2"),
            "Expected range error, got: {all}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn test_select_sender_too_high_reprompts() {
        // Arrange
        let senders = vec![synthetic_validated_alice(), synthetic_validated_bob()];
        let prompter = MockPrompter::new(vec![MockResponse::U32(99), MockResponse::U32(2)]);

        // Act
        let result = select_sender(&prompter, &senders, "alice").unwrap();

        // Assert
        assert_eq!(result.name(), "Bob Jones");
        prompter.assert_exhausted();
    }
}
