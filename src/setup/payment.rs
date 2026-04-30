use std::path::Path;

use crate::config::types::{Config, PaymentMethod};
use crate::config::writer::save_config;
use crate::error::AppError;
use super::prompter::Prompter;

/// Collect payment methods interactively and persist them to disk.
pub fn collect_payment(
    prompter: &dyn Prompter,
    config: &mut Config,
    config_path: &Path,
) -> Result<(), AppError> {
    prompter.message("\n--- Payment Methods ---\n");

    let count = loop {
        let n = prompter.u32_with_default("How many payment methods?", 2)?;
        if n >= 1 {
            break n;
        }
        prompter.message("At least one payment method is required.");
    };

    let mut methods = Vec::with_capacity(count as usize);
    for i in 1..=count {
        prompter.message(&format!("\nPayment method #{i}:"));
        let label = prompter.required_text("Label:")?;
        let iban = prompter.required_text("IBAN:")?;
        let bic_swift = prompter.required_text("BIC/SWIFT:")?;
        methods.push(PaymentMethod {
            label,
            iban,
            bic_swift,
        });
    }

    config.payment = Some(methods);
    save_config(config_path, config)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::loader::load_config;
    use crate::setup::mock_prompter::{MockPrompter, MockResponse};
    use crate::setup::test_helpers::*;

    #[test]
    fn test_collect_payment_single_method() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(1),
            MockResponse::Text("SEPA Transfer".into()),
            MockResponse::Text("DE89370400440532013000".into()),
            MockResponse::Text("COBADEFFXXX".into()),
        ]);

        // Act
        collect_payment(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let payment = config.payment.as_ref().unwrap();
        assert_eq!(payment.len(), 1);
        assert_eq!(payment[0].label, "SEPA Transfer");
        assert_eq!(payment[0].iban, "DE89370400440532013000");
        assert_eq!(payment[0].bic_swift, "COBADEFFXXX");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_payment_two_methods() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(2),
            MockResponse::Text("SEPA".into()),
            MockResponse::Text("DE00000000000000000001".into()),
            MockResponse::Text("BIC1".into()),
            MockResponse::Text("Wire".into()),
            MockResponse::Text("GB00000000000000000002".into()),
            MockResponse::Text("BIC2".into()),
        ]);

        // Act
        collect_payment(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let payment = config.payment.unwrap();
        assert_eq!(payment.len(), 2);
        assert_eq!(payment[0].label, "SEPA");
        assert_eq!(payment[1].label, "Wire");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_payment_three_methods() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(3),
            MockResponse::Text("A".into()), MockResponse::Text("IBAN1".into()), MockResponse::Text("BIC1".into()),
            MockResponse::Text("B".into()), MockResponse::Text("IBAN2".into()), MockResponse::Text("BIC2".into()),
            MockResponse::Text("C".into()), MockResponse::Text("IBAN3".into()), MockResponse::Text("BIC3".into()),
        ]);

        // Act
        collect_payment(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let payment = config.payment.unwrap();
        assert_eq!(payment.len(), 3);
        assert_eq!(payment[2].label, "C");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_payment_persists_to_disk() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(1),
            MockResponse::Text("SEPA".into()),
            MockResponse::Text("DE00".into()),
            MockResponse::Text("BIC".into()),
        ]);

        // Act
        collect_payment(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        let payment = loaded.payment.unwrap();
        assert_eq!(payment.len(), 1);
        assert_eq!(payment[0].label, "SEPA");
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_payment_displays_method_numbers() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(2),
            MockResponse::Text("A".into()), MockResponse::Text("DE".into()), MockResponse::Text("B".into()),
            MockResponse::Text("C".into()), MockResponse::Text("GB".into()), MockResponse::Text("D".into()),
        ]);

        // Act
        collect_payment(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let messages = prompter.messages.borrow();
        assert!(messages.iter().any(|m| m.contains("#1")), "Expected '#1' in messages: {messages:?}");
        assert!(messages.iter().any(|m| m.contains("#2")), "Expected '#2' in messages: {messages:?}");
        prompter.assert_exhausted();
    }
}
