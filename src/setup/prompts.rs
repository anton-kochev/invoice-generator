//! Reusable re-prompt helpers shared by setup, CLI, and invoice flows.
//!
//! Both helpers wrap the common pattern of "prompt the user, validate the
//! response, and re-prompt until valid". Centralizing this avoids drift
//! between hand-rolled loops scattered across the codebase.

use std::ops::RangeInclusive;

use crate::error::AppError;
use crate::setup::prompter::Prompter;

/// Prompt for a `u32` until the user enters a value inside `range`.
///
/// On out-of-range input, displays `Please enter a number between {start} and
/// {end}.` and re-prompts. The prompt and default are forwarded to
/// [`Prompter::u32_with_default`].
pub fn prompt_u32_in_range(
    prompter: &dyn Prompter,
    prompt: &str,
    range: RangeInclusive<u32>,
    default: u32,
) -> Result<u32, AppError> {
    loop {
        let v = prompter.u32_with_default(prompt, default)?;
        if range.contains(&v) {
            return Ok(v);
        }
        prompter.message(&format!(
            "Please enter a number between {} and {}.",
            range.start(),
            range.end()
        ));
    }
}

/// Prompt via `prompt_fn`, validate via `validator`, and re-prompt until
/// `validator` returns `Ok(())`. The validator's `Err` payload is shown to the
/// user verbatim before re-prompting.
///
/// `prompt_fn` is invoked with the same prompter, allowing it to call any
/// method (`required_text`, `text_with_default`, `positive_f64`, etc.) and
/// transform the result however the caller likes.
pub fn prompt_until_valid<T>(
    prompter: &dyn Prompter,
    mut prompt_fn: impl FnMut(&dyn Prompter) -> Result<T, AppError>,
    validator: impl Fn(&T) -> Result<(), String>,
) -> Result<T, AppError> {
    loop {
        let value = prompt_fn(prompter)?;
        match validator(&value) {
            Ok(()) => return Ok(value),
            Err(msg) => prompter.message(&msg),
        }
    }
}

/// Prompt via `prompt_fn`, then run `parser` on the raw input. Re-prompt until
/// `parser` returns `Ok(parsed_value)`. The parser's error message is shown to
/// the user verbatim before re-prompting.
///
/// Differs from [`prompt_until_valid`] in that the parser *transforms* the raw
/// input (e.g. `String` → `TemplateKey`) instead of merely validating it. This
/// removes the double-parse + `.expect("validated above")` smell at call sites
/// that need a typed value out of a string prompt.
pub fn prompt_parsed<T, U>(
    prompter: &dyn Prompter,
    mut prompt_fn: impl FnMut(&dyn Prompter) -> Result<T, AppError>,
    parser: impl Fn(T) -> Result<U, String>,
) -> Result<U, AppError> {
    loop {
        let raw = prompt_fn(prompter)?;
        match parser(raw) {
            Ok(parsed) => return Ok(parsed),
            Err(msg) => prompter.message(&msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::mock_prompter::{MockPrompter, MockResponse};

    // ── prompt_u32_in_range ──

    #[test]
    fn test_prompt_u32_in_range_accepts_value_in_range() {
        // Arrange
        let prompter = MockPrompter::new(vec![MockResponse::U32(5)]);

        // Act
        let result = prompt_u32_in_range(&prompter, "Pick:", 1..=12, 1).unwrap();

        // Assert
        assert_eq!(result, 5);
        prompter.assert_exhausted();
    }

    #[test]
    fn test_prompt_u32_in_range_rejects_below_and_reprompts() {
        // Arrange
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(0), // below range
            MockResponse::U32(7), // valid
        ]);

        // Act
        let result = prompt_u32_in_range(&prompter, "Pick:", 1..=12, 1).unwrap();

        // Assert
        assert_eq!(result, 7);
        let messages = prompter.messages.borrow();
        assert!(
            messages.iter().any(|m| m.contains("Please enter a number between 1 and 12")),
            "Expected range message, got: {messages:?}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn test_prompt_u32_in_range_rejects_above_and_reprompts() {
        // Arrange
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(99), // above range
            MockResponse::U32(12), // valid (boundary)
        ]);

        // Act
        let result = prompt_u32_in_range(&prompter, "Pick:", 1..=12, 1).unwrap();

        // Assert
        assert_eq!(result, 12);
        prompter.assert_exhausted();
    }

    #[test]
    fn test_prompt_u32_in_range_accepts_inclusive_boundaries() {
        // Arrange
        let prompter = MockPrompter::new(vec![MockResponse::U32(2000)]);

        // Act
        let result = prompt_u32_in_range(&prompter, "Year:", 2000..=2099, 2026).unwrap();

        // Assert
        assert_eq!(result, 2000);
        prompter.assert_exhausted();
    }

    #[test]
    fn test_prompt_u32_in_range_multiple_invalid_then_valid() {
        // Arrange
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(0),
            MockResponse::U32(13),
            MockResponse::U32(6),
        ]);

        // Act
        let result = prompt_u32_in_range(&prompter, "Month:", 1..=12, 1).unwrap();

        // Assert
        assert_eq!(result, 6);
        let messages = prompter.messages.borrow();
        let count = messages.iter().filter(|m| m.contains("Please enter a number between 1 and 12")).count();
        assert_eq!(count, 2, "Expected 2 error messages, got {count}");
        prompter.assert_exhausted();
    }

    // ── prompt_until_valid ──

    #[test]
    fn test_prompt_until_valid_returns_first_valid_value() {
        // Arrange
        let prompter = MockPrompter::new(vec![MockResponse::Text("hello".into())]);

        // Act
        let result = prompt_until_valid(
            &prompter,
            |p| p.required_text("Name:"),
            |_s: &String| Ok(()),
        )
        .unwrap();

        // Assert
        assert_eq!(result, "hello");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_prompt_until_valid_reprompts_on_validator_error() {
        // Arrange
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("bad".into()),
            MockResponse::Text("good".into()),
        ]);

        // Act
        let result = prompt_until_valid(
            &prompter,
            |p| p.required_text("Pick:"),
            |s: &String| {
                if s == "bad" {
                    Err("Try a different word".to_string())
                } else {
                    Ok(())
                }
            },
        )
        .unwrap();

        // Assert
        assert_eq!(result, "good");
        let messages = prompter.messages.borrow();
        assert!(
            messages.iter().any(|m| m == "Try a different word"),
            "Expected validator error message verbatim, got: {messages:?}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn test_prompt_until_valid_passes_validator_message_verbatim() {
        // Arrange
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("dev".into()),
            MockResponse::Text("ops".into()),
        ]);
        let existing = ["dev"];

        // Act
        let result = prompt_until_valid(
            &prompter,
            |p| p.required_text("Key:"),
            |s: &String| {
                if existing.iter().any(|e| e == s) {
                    Err(format!("Key \"{s}\" already exists. Choose another:"))
                } else {
                    Ok(())
                }
            },
        )
        .unwrap();

        // Assert
        assert_eq!(result, "ops");
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
    fn test_prompt_until_valid_multiple_invalid_then_valid() {
        // Arrange
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("a".into()),
            MockResponse::Text("b".into()),
            MockResponse::Text("c".into()),
        ]);

        // Act
        let result = prompt_until_valid(
            &prompter,
            |p| p.required_text("Pick:"),
            |s: &String| {
                if s == "c" { Ok(()) } else { Err(format!("not c: {s}")) }
            },
        )
        .unwrap();

        // Assert
        assert_eq!(result, "c");
        let messages = prompter.messages.borrow();
        assert_eq!(
            messages.iter().filter(|m| m.starts_with("not c")).count(),
            2,
            "Expected 2 rejection messages, got: {messages:?}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn test_prompt_until_valid_propagates_prompt_error() {
        // Arrange — the closure can return AppError directly; we verify the
        // helper propagates it without entering an infinite loop.
        let prompter = MockPrompter::new(vec![]);

        // Act
        let result: Result<String, AppError> = prompt_until_valid(
            &prompter,
            |_p| Err(AppError::SetupCancelled),
            |_s: &String| Ok(()),
        );

        // Assert
        assert!(matches!(result, Err(AppError::SetupCancelled)));
    }

    // ── prompt_parsed ──

    #[test]
    fn test_prompt_parsed_returns_parsed_value_on_first_try() {
        // Arrange
        let prompter = MockPrompter::new(vec![MockResponse::Text("42".into())]);

        // Act
        let result: u32 = prompt_parsed(
            &prompter,
            |p| p.required_text("Pick a number:"),
            |s: String| s.parse::<u32>().map_err(|_| "not a number".to_string()),
        )
        .unwrap();

        // Assert
        assert_eq!(result, 42);
        prompter.assert_exhausted();
    }

    #[test]
    fn test_prompt_parsed_reprompts_on_parser_error_with_verbatim_message() {
        // Arrange
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("nope".into()),
            MockResponse::Text("7".into()),
        ]);

        // Act
        let result: u32 = prompt_parsed(
            &prompter,
            |p| p.required_text("Pick a number:"),
            |s: String| {
                s.parse::<u32>()
                    .map_err(|_| format!("'{s}' is not a valid number"))
            },
        )
        .unwrap();

        // Assert
        assert_eq!(result, 7);
        let messages = prompter.messages.borrow();
        assert!(
            messages.iter().any(|m| m == "'nope' is not a valid number"),
            "Expected verbatim parser error, got: {messages:?}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn test_prompt_parsed_propagates_prompt_error() {
        // Arrange — prompt_fn returns an AppError; the helper must surface it
        // rather than loop forever.
        let prompter = MockPrompter::new(vec![]);

        // Act
        let result: Result<u32, AppError> = prompt_parsed(
            &prompter,
            |_p| Err(AppError::SetupCancelled),
            |s: String| s.parse::<u32>().map_err(|_| "bad".to_string()),
        );

        // Assert
        assert!(matches!(result, Err(AppError::SetupCancelled)));
    }

    #[test]
    fn test_prompt_parsed_transforms_input_type_to_output_type() {
        // Arrange — prompt_fn returns String; parser yields a custom enum.
        #[derive(Debug, PartialEq)]
        enum Color {
            Red,
            Blue,
        }

        let prompter = MockPrompter::new(vec![
            MockResponse::Text("green".into()), // unknown
            MockResponse::Text("blue".into()),  // valid
        ]);

        // Act
        let result: Color = prompt_parsed(
            &prompter,
            |p| p.required_text("Color:"),
            |s: String| match s.as_str() {
                "red" => Ok(Color::Red),
                "blue" => Ok(Color::Blue),
                _ => Err(format!("unknown color: {s}")),
            },
        )
        .unwrap();

        // Assert
        assert_eq!(result, Color::Blue);
        let messages = prompter.messages.borrow();
        assert!(
            messages.iter().any(|m| m == "unknown color: green"),
            "Expected verbatim parser error, got: {messages:?}"
        );
        prompter.assert_exhausted();
    }
}
