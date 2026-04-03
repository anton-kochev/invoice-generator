use std::cell::RefCell;
use std::collections::VecDeque;

use crate::error::AppError;
use super::prompter::Prompter;

/// A canned response for [`MockPrompter`].
#[derive(Debug)]
pub enum MockResponse {
    Text(String),
    OptionalText(Option<String>),
    Lines(Vec<String>),
    Confirm(bool),
    F64(f64),
    U32(u32),
}

/// Queue-based mock of [`Prompter`] for unit tests.
pub struct MockPrompter {
    responses: RefCell<VecDeque<MockResponse>>,
    /// Captured messages sent via [`Prompter::message`].
    pub messages: RefCell<Vec<String>>,
}

impl MockPrompter {
    pub fn new(responses: Vec<MockResponse>) -> Self {
        Self {
            responses: RefCell::new(VecDeque::from(responses)),
            messages: RefCell::new(Vec::new()),
        }
    }

    /// Panic if any responses remain unconsumed.
    pub fn assert_exhausted(&self) {
        let remaining = self.responses.borrow().len();
        assert!(
            remaining == 0,
            "MockPrompter has {remaining} unconsumed responses"
        );
    }

    fn pop(&self, expected: &str) -> MockResponse {
        self.responses
            .borrow_mut()
            .pop_front()
            .unwrap_or_else(|| panic!("MockPrompter exhausted; expected {expected}"))
    }
}

impl Prompter for MockPrompter {
    fn message(&self, text: &str) {
        self.messages.borrow_mut().push(text.to_string());
    }

    fn required_text(&self, _prompt: &str) -> Result<String, AppError> {
        match self.pop("Text") {
            MockResponse::Text(s) => Ok(s),
            other => panic!("MockPrompter: expected Text, got {other:?}"),
        }
    }

    fn optional_text(&self, _prompt: &str) -> Result<Option<String>, AppError> {
        match self.pop("OptionalText") {
            MockResponse::OptionalText(s) => Ok(s),
            other => panic!("MockPrompter: expected OptionalText, got {other:?}"),
        }
    }

    fn multi_line(&self, _prompt: &str) -> Result<Vec<String>, AppError> {
        match self.pop("Lines") {
            MockResponse::Lines(v) => Ok(v),
            other => panic!("MockPrompter: expected Lines, got {other:?}"),
        }
    }

    fn text_with_default(&self, _prompt: &str, default: &str) -> Result<String, AppError> {
        match self.pop("Text") {
            MockResponse::Text(s) if s.is_empty() => Ok(default.to_string()),
            MockResponse::Text(s) => Ok(s),
            other => panic!("MockPrompter: expected Text, got {other:?}"),
        }
    }

    fn u32_with_default(&self, _prompt: &str, _default: u32) -> Result<u32, AppError> {
        match self.pop("U32") {
            MockResponse::U32(v) => Ok(v),
            other => panic!("MockPrompter: expected U32, got {other:?}"),
        }
    }

    fn positive_f64(&self, _prompt: &str) -> Result<f64, AppError> {
        match self.pop("F64") {
            MockResponse::F64(v) => Ok(v),
            other => panic!("MockPrompter: expected F64, got {other:?}"),
        }
    }

    fn confirm(&self, _prompt: &str, _default: bool) -> Result<bool, AppError> {
        match self.pop("Confirm") {
            MockResponse::Confirm(b) => Ok(b),
            other => panic!("MockPrompter: expected Confirm, got {other:?}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_prompter_pops_text_response() {
        // Arrange
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("hello".into()),
        ]);

        // Act
        let result = prompter.required_text("Name:").unwrap();

        // Assert
        assert_eq!(result, "hello");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_mock_prompter_pops_confirm_response() {
        // Arrange
        let prompter = MockPrompter::new(vec![
            MockResponse::Confirm(true),
        ]);

        // Act
        let result = prompter.confirm("Continue?", false).unwrap();

        // Assert
        assert!(result);
        prompter.assert_exhausted();
    }

    #[test]
    fn test_mock_prompter_captures_messages() {
        // Arrange
        let prompter = MockPrompter::new(vec![]);

        // Act
        prompter.message("Hello");
        prompter.message("World");

        // Assert
        let messages = prompter.messages.borrow();
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0], "Hello");
        assert_eq!(messages[1], "World");
    }

    #[test]
    fn test_mock_prompter_assert_exhausted_passes_when_empty() {
        // Arrange
        let prompter = MockPrompter::new(vec![]);

        // Act & Assert
        prompter.assert_exhausted(); // should not panic
    }

    #[test]
    #[should_panic(expected = "unconsumed responses")]
    fn test_mock_prompter_assert_exhausted_panics_when_remaining() {
        // Arrange
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("unused".into()),
        ]);

        // Act & Assert
        prompter.assert_exhausted(); // should panic
    }
}
