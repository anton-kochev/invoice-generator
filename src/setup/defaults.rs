use std::path::Path;
use std::str::FromStr;

use crate::config::types::{Config, Defaults, TemplateKey};
use crate::config::writer::save_config;
use crate::error::AppError;
use crate::locale::Locale;
use super::prompter::Prompter;
use super::prompts::prompt_until_valid;

/// Collect default invoice values interactively and persist them to disk.
pub fn collect_defaults(
    prompter: &dyn Prompter,
    config: &mut Config,
    config_path: &Path,
) -> Result<(), AppError> {
    prompter.message("\n--- Defaults ---\n");

    let currency = prompter.text_with_default("Currency:", "EUR")?;
    let invoice_date_day = prompter.u32_with_default("Invoice date (day of month):", 9)?;
    let payment_terms_days = prompter.u32_with_default("Payment terms (days):", 30)?;

    let template_input = prompt_until_valid(
        prompter,
        |p| p.text_with_default("Template:", "leda"),
        |input: &String| {
            TemplateKey::from_str(input).map(|_| ()).map_err(|_| {
                let list: Vec<String> = TemplateKey::ALL
                    .iter()
                    .map(|t| format!("{} ({})", t, t.description()))
                    .collect();
                format!("Invalid template. Available: {}", list.join(", "))
            })
        },
    )?;
    let template = TemplateKey::from_str(&template_input).expect("validated above");

    let locale_input = prompt_until_valid(
        prompter,
        |p| p.text_with_default("Locale for PDF formatting:", "en-US"),
        |input: &String| {
            Locale::from_str(input).map(|_| ()).map_err(|_| {
                let list: Vec<String> = Locale::ALL.iter().map(|l| l.to_string()).collect();
                format!("Unsupported locale. Available: {}", list.join(", "))
            })
        },
    )?;
    let locale = Locale::from_str(&locale_input).expect("validated above");

    config.defaults = Some(Defaults {
        currency,
        invoice_date_day,
        payment_terms_days,
        template,
        locale,
    });

    save_config(config_path, config)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::loader::load_config;
    use crate::locale::Locale;
    use crate::setup::mock_prompter::{MockPrompter, MockResponse};
    use crate::setup::test_helpers::*;

    #[test]
    fn test_collect_defaults_accepts_defaults() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("".into()),  // accept default EUR
            MockResponse::U32(9),
            MockResponse::U32(30),
            MockResponse::Text("leda".into()),
            MockResponse::Text("en-US".into()),
        ]);

        // Act
        collect_defaults(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let defaults = config.defaults.as_ref().unwrap();
        assert_eq!(defaults.currency, "EUR");
        assert_eq!(defaults.invoice_date_day, 9);
        assert_eq!(defaults.payment_terms_days, 30);
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_defaults_custom_values() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("USD".into()),
            MockResponse::U32(15),
            MockResponse::U32(14),
            MockResponse::Text("leda".into()),
            MockResponse::Text("en-US".into()),
        ]);

        // Act
        collect_defaults(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let defaults = config.defaults.unwrap();
        assert_eq!(defaults.currency, "USD");
        assert_eq!(defaults.invoice_date_day, 15);
        assert_eq!(defaults.payment_terms_days, 14);
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_defaults_persists_to_disk() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("CHF".into()),
            MockResponse::U32(1),
            MockResponse::U32(60),
            MockResponse::Text("leda".into()),
            MockResponse::Text("en-US".into()),
        ]);

        // Act
        collect_defaults(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        let defaults = loaded.defaults.unwrap();
        assert_eq!(defaults.currency, "CHF");
        assert_eq!(defaults.invoice_date_day, 1);
        assert_eq!(defaults.payment_terms_days, 60);
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_defaults_accepts_default_template() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("EUR".into()),
            MockResponse::U32(9),
            MockResponse::U32(30),
            MockResponse::Text("leda".into()),
            MockResponse::Text("en-US".into()),
        ]);
        // Act
        collect_defaults(&prompter, &mut config, &cfg_path(&dir)).unwrap();
        // Assert
        let defaults = config.defaults.as_ref().unwrap();
        assert_eq!(defaults.template, TemplateKey::Leda);
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_defaults_custom_template_saved() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("EUR".into()),
            MockResponse::U32(9),
            MockResponse::U32(30),
            MockResponse::Text("callisto".into()),
            MockResponse::Text("en-US".into()),
        ]);
        // Act
        collect_defaults(&prompter, &mut config, &cfg_path(&dir)).unwrap();
        // Assert
        assert_eq!(config.defaults.as_ref().unwrap().template, TemplateKey::Callisto);
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_defaults_template_persisted_to_disk() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("EUR".into()),
            MockResponse::U32(9),
            MockResponse::U32(30),
            MockResponse::Text("thebe".into()),
            MockResponse::Text("en-US".into()),
        ]);
        // Act
        collect_defaults(&prompter, &mut config, &cfg_path(&dir)).unwrap();
        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        assert_eq!(loaded.defaults.unwrap().template, TemplateKey::Thebe);
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_defaults_invalid_template_reprompts() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("EUR".into()),
            MockResponse::U32(9),
            MockResponse::U32(30),
            MockResponse::Text("bogus".into()),
            MockResponse::Text("leda".into()),
            MockResponse::Text("en-US".into()),
        ]);
        // Act
        collect_defaults(&prompter, &mut config, &cfg_path(&dir)).unwrap();
        // Assert
        assert_eq!(config.defaults.as_ref().unwrap().template, TemplateKey::Leda);
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_defaults_invalid_template_shows_available_list() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("EUR".into()),
            MockResponse::U32(9),
            MockResponse::U32(30),
            MockResponse::Text("xyz".into()),
            MockResponse::Text("leda".into()),
            MockResponse::Text("en-US".into()),
        ]);
        // Act
        collect_defaults(&prompter, &mut config, &cfg_path(&dir)).unwrap();
        // Assert
        let messages = prompter.messages.borrow();
        assert!(
            messages.iter().any(|m| m.contains("callisto") && m.contains("leda") && m.contains("thebe")),
            "Expected available templates in messages, got: {messages:?}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_defaults_accepts_default_locale() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("EUR".into()),
            MockResponse::U32(9),
            MockResponse::U32(30),
            MockResponse::Text("leda".into()),
            MockResponse::Text("en-US".into()),
        ]);

        // Act
        collect_defaults(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let defaults = config.defaults.as_ref().unwrap();
        assert_eq!(defaults.locale, Locale::EnUs);
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_defaults_custom_locale() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("EUR".into()),
            MockResponse::U32(9),
            MockResponse::U32(30),
            MockResponse::Text("leda".into()),
            MockResponse::Text("de-DE".into()),
        ]);

        // Act
        collect_defaults(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let defaults = config.defaults.as_ref().unwrap();
        assert_eq!(defaults.locale, Locale::DeDe);
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_defaults_invalid_locale_reprompts() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("EUR".into()),
            MockResponse::U32(9),
            MockResponse::U32(30),
            MockResponse::Text("leda".into()),
            MockResponse::Text("xx-YY".into()),  // invalid — triggers re-prompt
            MockResponse::Text("en-US".into()),   // valid on retry
        ]);

        // Act
        collect_defaults(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let defaults = config.defaults.as_ref().unwrap();
        assert_eq!(defaults.locale, Locale::EnUs);
        let messages = prompter.messages.borrow();
        assert!(
            messages.iter().any(|m| m.contains("Unsupported locale")),
            "Expected 'Unsupported locale' message, got: {messages:?}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_defaults_locale_persisted_to_disk() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("EUR".into()),
            MockResponse::U32(9),
            MockResponse::U32(30),
            MockResponse::Text("leda".into()),
            MockResponse::Text("fr-FR".into()),
        ]);

        // Act
        collect_defaults(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        let defaults = loaded.defaults.unwrap();
        assert_eq!(defaults.locale, Locale::FrFr);
        prompter.assert_exhausted();
    }
}
