use std::path::Path;

use super::prompter::Prompter;
use super::prompts::{prompt_parsed, prompt_until_valid};
use crate::config::types::{Config, PaymentMethod};
use crate::config::writer::save_config;
use crate::domain::{Iban, PaymentMethodKey};
use crate::error::AppError;

/// Collect payment methods interactively and persist them to disk.
pub fn collect_payment(
    prompter: &dyn Prompter,
    config: &mut Config,
    config_path: &Path,
) -> Result<(), AppError> {
    prompter.message("\n--- Payment Methods ---\n");

    let count = prompt_until_valid(
        prompter,
        |p| p.u32_with_default("How many payment methods?", 2),
        |n: &u32| {
            if *n >= 1 {
                Ok(())
            } else {
                Err("At least one payment method is required.".into())
            }
        },
    )?;

    let mut methods = Vec::with_capacity(count as usize);
    for i in 1..=count {
        prompter.message(&format!("\nPayment method #{i}:"));
        let key = prompt_parsed(
            prompter,
            |p| p.required_text("Key (slug, e.g. mono-eur-sepa):"),
            |raw: String| PaymentMethodKey::try_new(raw).map_err(|e| e.to_string()),
        )?;
        let label =
            prompter.optional_text("Display label (shown on invoice — leave empty to skip):")?;
        let iban = prompt_parsed(
            prompter,
            |p| p.required_text("IBAN:"),
            |raw: String| Iban::try_new(&raw).map_err(|e| e.to_string()),
        )?;
        let bic_swift = prompter.required_text("BIC/SWIFT:")?;
        methods.push(PaymentMethod {
            key: Some(key),
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

    /// Synthetic but mod-97-valid IBANs for tests.
    /// (Validation is real now, so dummy values like "DE00" are rejected.)
    const VALID_DE_IBAN: &str = "DE89370400440532013000";
    const VALID_GB_IBAN: &str = "GB82WEST12345698765432";
    const VALID_UA_IBAN: &str = "UA213996220000026007233566001";

    #[test]
    fn test_collect_payment_single_method() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(1),
            MockResponse::Text("sepa-transfer".into()),
            MockResponse::OptionalText(Some("SEPA Transfer".into())),
            MockResponse::Text(VALID_DE_IBAN.into()),
            MockResponse::Text("COBADEFFXXX".into()),
        ]);

        // Act
        collect_payment(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let payment = config.payment.as_ref().unwrap();
        assert_eq!(payment.len(), 1);
        assert_eq!(
            payment[0].key.as_ref().map(|k| k.as_str()),
            Some("sepa-transfer")
        );
        assert_eq!(payment[0].label.as_deref(), Some("SEPA Transfer"));
        assert_eq!(payment[0].iban.as_str(), VALID_DE_IBAN);
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
            MockResponse::Text("sepa".into()),
            MockResponse::OptionalText(Some("SEPA".into())),
            MockResponse::Text(VALID_DE_IBAN.into()),
            MockResponse::Text("BIC1".into()),
            MockResponse::Text("wire".into()),
            MockResponse::OptionalText(Some("Wire".into())),
            MockResponse::Text(VALID_GB_IBAN.into()),
            MockResponse::Text("BIC2".into()),
        ]);

        // Act
        collect_payment(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let payment = config.payment.unwrap();
        assert_eq!(payment.len(), 2);
        assert_eq!(payment[0].label.as_deref(), Some("SEPA"));
        assert_eq!(payment[1].label.as_deref(), Some("Wire"));
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_payment_three_methods() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(3),
            MockResponse::Text("a".into()),
            MockResponse::OptionalText(Some("A".into())),
            MockResponse::Text(VALID_DE_IBAN.into()),
            MockResponse::Text("BIC1".into()),
            MockResponse::Text("b".into()),
            MockResponse::OptionalText(Some("B".into())),
            MockResponse::Text(VALID_GB_IBAN.into()),
            MockResponse::Text("BIC2".into()),
            MockResponse::Text("c".into()),
            MockResponse::OptionalText(Some("C".into())),
            MockResponse::Text(VALID_UA_IBAN.into()),
            MockResponse::Text("BIC3".into()),
        ]);

        // Act
        collect_payment(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let payment = config.payment.unwrap();
        assert_eq!(payment.len(), 3);
        assert_eq!(payment[2].label.as_deref(), Some("C"));
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_payment_persists_to_disk() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(1),
            MockResponse::Text("sepa".into()),
            MockResponse::OptionalText(Some("SEPA".into())),
            MockResponse::Text(VALID_DE_IBAN.into()),
            MockResponse::Text("BIC".into()),
        ]);

        // Act
        collect_payment(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        let payment = loaded.payment.unwrap();
        assert_eq!(payment.len(), 1);
        assert_eq!(payment[0].label.as_deref(), Some("SEPA"));
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_payment_displays_method_numbers() {
        // Arrange
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(2),
            MockResponse::Text("a".into()),
            MockResponse::OptionalText(Some("A".into())),
            MockResponse::Text(VALID_DE_IBAN.into()),
            MockResponse::Text("B".into()),
            MockResponse::Text("c".into()),
            MockResponse::OptionalText(Some("C".into())),
            MockResponse::Text(VALID_GB_IBAN.into()),
            MockResponse::Text("D".into()),
        ]);

        // Act
        collect_payment(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let messages = prompter.messages.borrow();
        assert!(
            messages.iter().any(|m| m.contains("#1")),
            "Expected '#1' in messages: {messages:?}"
        );
        assert!(
            messages.iter().any(|m| m.contains("#2")),
            "Expected '#2' in messages: {messages:?}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_payment_invalid_iban_reprompts() {
        // Arrange — first IBAN has bad checksum, second is valid.
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(1),
            MockResponse::Text("sepa".into()),
            MockResponse::OptionalText(Some("SEPA".into())),
            MockResponse::Text("GB00WEST12345698765432".into()), // bad checksum
            MockResponse::Text(VALID_DE_IBAN.into()),
            MockResponse::Text("BIC".into()),
        ]);

        // Act
        collect_payment(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let payment = config.payment.unwrap();
        assert_eq!(payment[0].iban.as_str(), VALID_DE_IBAN);
        let messages = prompter.messages.borrow();
        assert!(
            messages.iter().any(|m| m.contains("checksum")),
            "Expected re-prompt with checksum error, got: {messages:?}"
        );
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_payment_blank_label_records_none() {
        // Arrange — user accepts the prompt for label with a blank line.
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(1),
            MockResponse::Text("sepa".into()),
            MockResponse::OptionalText(None),
            MockResponse::Text(VALID_DE_IBAN.into()),
            MockResponse::Text("BIC".into()),
        ]);

        // Act
        collect_payment(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let payment = config.payment.unwrap();
        assert!(payment[0].label.is_none());
        assert_eq!(payment[0].key.as_ref().map(|k| k.as_str()), Some("sepa"));
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_payment_with_label_records_some() {
        // Arrange — symmetric: user provides a non-empty label.
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(1),
            MockResponse::Text("sepa".into()),
            MockResponse::OptionalText(Some("SEPA EUR".into())),
            MockResponse::Text(VALID_DE_IBAN.into()),
            MockResponse::Text("BIC".into()),
        ]);

        // Act
        collect_payment(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let payment = config.payment.unwrap();
        assert_eq!(payment[0].label.as_deref(), Some("SEPA EUR"));
        prompter.assert_exhausted();
    }

    #[test]
    fn test_collect_payment_invalid_key_reprompts() {
        // Arrange — first key is uppercase (rejected), second is valid.
        let dir = setup_dir(None);
        let mut config = empty_config();
        let prompter = MockPrompter::new(vec![
            MockResponse::U32(1),
            MockResponse::Text("SEPA".into()), // rejected: uppercase
            MockResponse::Text("sepa".into()), // accepted
            MockResponse::OptionalText(None),
            MockResponse::Text(VALID_DE_IBAN.into()),
            MockResponse::Text("BIC".into()),
        ]);

        // Act
        collect_payment(&prompter, &mut config, &cfg_path(&dir)).unwrap();

        // Assert
        let payment = config.payment.unwrap();
        assert_eq!(payment[0].key.as_ref().map(|k| k.as_str()), Some("sepa"));
        let messages = prompter.messages.borrow();
        assert!(
            messages.iter().any(|m| m.contains("invalid characters")),
            "Expected re-prompt with invalid-character error, got: {messages:?}"
        );
        prompter.assert_exhausted();
    }
}
