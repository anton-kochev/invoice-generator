use crate::error::AppError;
use inquire::ui::{Color, RenderConfig, StyleSheet, Styled};

/// Abstracts interactive user prompts for testability.
///
/// Each method returns `Ok(value)` on success, or `Err(AppError::SetupCancelled)`
/// if the user presses Escape/Ctrl-C.
pub trait Prompter {
    /// Display a message to the user (header, summary, etc.)
    fn message(&self, text: &str);

    /// Prompt for a required non-empty string. Re-prompts until non-blank.
    fn required_text(&self, prompt: &str) -> Result<String, AppError>;

    /// Prompt for an optional string. Returns None if blank.
    fn optional_text(&self, prompt: &str) -> Result<Option<String>, AppError>;

    /// Collect lines until a blank line. At least one line required.
    fn multi_line(&self, prompt: &str) -> Result<Vec<String>, AppError>;

    /// Prompt for text with a default value. Enter accepts default.
    fn text_with_default(&self, prompt: &str, default: &str) -> Result<String, AppError>;

    /// Prompt for a u32 with a default value.
    fn u32_with_default(&self, prompt: &str, default: u32) -> Result<u32, AppError>;

    /// Prompt for a positive f64. Re-prompts on non-positive or parse error.
    fn positive_f64(&self, prompt: &str) -> Result<f64, AppError>;

    /// Prompt for a positive f64 with a default value. Enter accepts default.
    fn positive_f64_with_default(&self, prompt: &str, default: f64) -> Result<f64, AppError>;

    /// Yes/No confirmation. `default` determines what Enter alone means.
    fn confirm(&self, prompt: &str, default: bool) -> Result<bool, AppError>;
}

/// Production prompter backed by the `inquire` crate.
pub struct InquirePrompter;

impl InquirePrompter {
    /// Creates a new `InquirePrompter` and sets a global render config
    /// with a `❯` chevron prompt prefix in light cyan.
    pub fn new() -> Self {
        let config = RenderConfig::default().with_prompt_prefix(
            Styled::new("❯").with_style_sheet(StyleSheet::new().with_fg(Color::LightCyan)),
        );
        inquire::set_global_render_config(config);
        Self
    }
}

impl Prompter for InquirePrompter {
    fn message(&self, text: &str) {
        println!("{text}");
    }

    fn required_text(&self, prompt: &str) -> Result<String, AppError> {
        inquire::Text::new(prompt)
            .with_validator(|input: &str| {
                if input.trim().is_empty() {
                    Ok(inquire::validator::Validation::Invalid(
                        "This field is required.".into(),
                    ))
                } else {
                    Ok(inquire::validator::Validation::Valid)
                }
            })
            .prompt()
            .map(|s| s.trim().to_string())
            .map_err(|_| AppError::SetupCancelled)
    }

    fn optional_text(&self, prompt: &str) -> Result<Option<String>, AppError> {
        let input = inquire::Text::new(prompt)
            .prompt()
            .map_err(|_| AppError::SetupCancelled)?;
        let trimmed = input.trim();
        if trimmed.is_empty() {
            Ok(None)
        } else {
            Ok(Some(trimmed.to_string()))
        }
    }

    fn multi_line(&self, prompt: &str) -> Result<Vec<String>, AppError> {
        let mut lines = Vec::new();
        loop {
            let label = format!("{prompt} line {}:", lines.len() + 1);
            let mut text = inquire::Text::new(&label);

            if lines.is_empty() {
                text = text.with_help_message("At least one line required. Blank line to finish.");
            } else {
                text = text.with_help_message("Blank line to finish.");
            }

            let line = text.prompt().map_err(|_| AppError::SetupCancelled)?;

            if line.trim().is_empty() {
                if lines.is_empty() {
                    continue;
                }
                break;
            }
            lines.push(line.trim().to_string());
        }
        Ok(lines)
    }

    fn text_with_default(&self, prompt: &str, default: &str) -> Result<String, AppError> {
        inquire::Text::new(prompt)
            .with_default(default)
            .prompt()
            .map(|s| s.trim().to_string())
            .map_err(|_| AppError::SetupCancelled)
    }

    fn u32_with_default(&self, prompt: &str, default: u32) -> Result<u32, AppError> {
        inquire::CustomType::<u32>::new(prompt)
            .with_default(default)
            .with_error_message("Please enter a valid number.")
            .prompt()
            .map_err(|_| AppError::SetupCancelled)
    }

    fn positive_f64(&self, prompt: &str) -> Result<f64, AppError> {
        inquire::CustomType::<f64>::new(prompt)
            .with_error_message("Please enter a valid number.")
            .with_validator(|val: &f64| {
                if *val > 0.0 {
                    Ok(inquire::validator::Validation::Valid)
                } else {
                    Ok(inquire::validator::Validation::Invalid(
                        "Rate must be greater than 0.".into(),
                    ))
                }
            })
            .prompt()
            .map_err(|_| AppError::SetupCancelled)
    }

    fn positive_f64_with_default(&self, prompt: &str, default: f64) -> Result<f64, AppError> {
        inquire::CustomType::<f64>::new(prompt)
            .with_default(default)
            .with_error_message("Please enter a valid number.")
            .with_validator(|val: &f64| {
                if *val > 0.0 {
                    Ok(inquire::validator::Validation::Valid)
                } else {
                    Ok(inquire::validator::Validation::Invalid(
                        "Value must be greater than 0.".into(),
                    ))
                }
            })
            .prompt()
            .map_err(|_| AppError::SetupCancelled)
    }

    fn confirm(&self, prompt: &str, default: bool) -> Result<bool, AppError> {
        inquire::Confirm::new(prompt)
            .with_default(default)
            .prompt()
            .map_err(|_| AppError::SetupCancelled)
    }
}
