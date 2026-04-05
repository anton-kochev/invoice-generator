use std::path::Path;
use std::str::FromStr;

use crate::config::types::{Config, Defaults, TemplateKey};
use crate::config::writer::save_config;
use crate::error::AppError;
use super::prompter::Prompter;

/// Collect default invoice values interactively and persist them to disk.
pub fn collect_defaults(
    prompter: &dyn Prompter,
    config: &mut Config,
    dir: &Path,
) -> Result<(), AppError> {
    prompter.message("\n--- Defaults ---\n");

    let currency = prompter.text_with_default("Currency:", "EUR")?;
    let invoice_date_day = prompter.u32_with_default("Invoice date (day of month):", 9)?;
    let payment_terms_days = prompter.u32_with_default("Payment terms (days):", 30)?;

    let template = loop {
        let input = prompter.text_with_default("Template:", "leda")?;
        match TemplateKey::from_str(&input) {
            Ok(t) => break t,
            Err(_) => {
                let list: Vec<String> = TemplateKey::ALL.iter()
                    .map(|t| format!("{} ({})", t, t.description()))
                    .collect();
                prompter.message(&format!("Invalid template. Available: {}", list.join(", ")));
            }
        }
    };

    config.defaults = Some(Defaults {
        currency,
        invoice_date_day,
        payment_terms_days,
        template,
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
    fn test_collect_defaults_accepts_defaults() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::Text("".into()),  // accept default EUR
            MockResponse::U32(9),
            MockResponse::U32(30),
            MockResponse::Text("leda".into()),
        ]);

        // Act
        collect_defaults(&prompter, &mut config, dir.path()).unwrap();

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
        ]);

        // Act
        collect_defaults(&prompter, &mut config, dir.path()).unwrap();

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
        ]);

        // Act
        collect_defaults(&prompter, &mut config, dir.path()).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(dir.path()));
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
        ]);
        // Act
        collect_defaults(&prompter, &mut config, dir.path()).unwrap();
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
        ]);
        // Act
        collect_defaults(&prompter, &mut config, dir.path()).unwrap();
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
        ]);
        // Act
        collect_defaults(&prompter, &mut config, dir.path()).unwrap();
        // Assert
        let loaded = unwrap_loaded(load_config(dir.path()));
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
        ]);
        // Act
        collect_defaults(&prompter, &mut config, dir.path()).unwrap();
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
        ]);
        // Act
        collect_defaults(&prompter, &mut config, dir.path()).unwrap();
        // Assert
        let messages = prompter.messages.borrow();
        assert!(
            messages.iter().any(|m| m.contains("callisto") && m.contains("leda") && m.contains("thebe")),
            "Expected available templates in messages, got: {messages:?}"
        );
        prompter.assert_exhausted();
    }
}
