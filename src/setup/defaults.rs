use std::path::Path;

use crate::config::types::{Config, Defaults};
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

    config.defaults = Some(Defaults {
        currency,
        invoice_date_day,
        payment_terms_days,
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
}
