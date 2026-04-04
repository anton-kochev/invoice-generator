use std::fmt;

use super::types::*;
use crate::error::AppError;

/// Identifies a top-level config section for validation reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSection {
    Sender,
    Recipient,
    Payment,
    Presets,
}

impl fmt::Display for ConfigSection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sender => write!(f, "sender"),
            Self::Recipient => write!(f, "recipient"),
            Self::Payment => write!(f, "payment"),
            Self::Presets => write!(f, "presets"),
        }
    }
}

/// A fully validated configuration with all required sections present.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedConfig {
    pub sender: Sender,
    pub recipient: Recipient,
    /// All available recipients (guaranteed non-empty).
    pub recipients: Vec<Recipient>,
    /// Key of the default recipient profile.
    pub default_recipient_key: String,
    /// Guaranteed non-empty.
    pub payment: Vec<PaymentMethod>,
    /// Guaranteed non-empty.
    pub presets: Vec<Preset>,
    pub defaults: Defaults,
}

impl ValidatedConfig {
    /// Returns the default recipient (the one matching `default_recipient_key`).
    #[allow(dead_code)] // needed by Story 7.2 (recipient selection)
    pub fn active_recipient(&self) -> &Recipient {
        &self.recipient
    }
}

/// Result of validating a [`Config`].
#[derive(Debug)]
pub enum ValidationOutcome {
    /// All required sections are present.
    Complete(ValidatedConfig),
    /// One or more required sections are missing.
    Incomplete {
        #[allow(dead_code)] // needed by setup wizard (Story 2.1) to resume from partial config
        config: Config,
        missing: Vec<ConfigSection>,
    },
}

impl Config {
    /// Validate that all required sections are present.
    ///
    /// Returns `Ok(ValidationOutcome::Complete)` with a [`ValidatedConfig`] when
    /// all sections are filled in, or `Ok(ValidationOutcome::Incomplete)` listing
    /// which sections are missing.
    ///
    /// Returns `Err(AppError)` for hard errors like duplicate keys or invalid
    /// default recipient references.
    pub fn validate(self) -> Result<ValidationOutcome, AppError> {
        let mut missing = Vec::new();

        let sender = self.sender;
        let payment = self.payment;
        let presets = self.presets;
        let defaults = self.defaults;

        if sender.is_none() {
            missing.push(ConfigSection::Sender);
        }

        // Normalize recipients: v2 (recipients list) takes precedence over v1 (single recipient).
        let (recipients, default_key) =
            match (self.recipient, self.recipients, self.default_recipient) {
                (_, Some(list), dk) if !list.is_empty() => (Some(list), dk),
                (_, Some(_), _) => {
                    // Empty list — treat as missing
                    missing.push(ConfigSection::Recipient);
                    (None, None)
                }
                (Some(mut r), None, _) => {
                    let key = r
                        .key
                        .clone()
                        .unwrap_or_else(|| derive_recipient_key(&r.name));
                    r.key = Some(key.clone());
                    (Some(vec![r]), Some(key))
                }
                (None, None, _) => {
                    missing.push(ConfigSection::Recipient);
                    (None, None)
                }
            };

        // Validate recipient keys if recipients present.
        if let Some(ref list) = recipients {
            // Check for empty keys
            for r in list {
                if r.key.as_ref().map_or(true, |k| k.is_empty()) {
                    return Err(AppError::InvalidDefaultRecipient(
                        "recipient has empty or missing key".into(),
                    ));
                }
            }
            // Check for duplicate keys
            let mut seen = std::collections::HashSet::new();
            for r in list {
                let k = r.key.as_ref().unwrap();
                if !seen.insert(k.clone()) {
                    return Err(AppError::DuplicateRecipientKey(k.clone()));
                }
            }
            // Validate default_recipient references a valid key
            match &default_key {
                Some(dk) => {
                    if !list.iter().any(|r| r.key.as_deref() == Some(dk.as_str())) {
                        return Err(AppError::InvalidDefaultRecipient(dk.clone()));
                    }
                }
                None => {
                    return Err(AppError::InvalidDefaultRecipient(
                        "default_recipient is required when recipients are defined".into(),
                    ));
                }
            }
        }

        match &payment {
            Some(v) if !v.is_empty() => {}
            _ => missing.push(ConfigSection::Payment),
        }
        match &presets {
            Some(v) if !v.is_empty() => {}
            _ => missing.push(ConfigSection::Presets),
        }

        if missing.is_empty() {
            let recipients_vec = recipients.unwrap();
            let dk = default_key.unwrap();
            let recipient = recipients_vec
                .iter()
                .find(|r| r.key.as_deref() == Some(&dk))
                .cloned()
                .unwrap();

            Ok(ValidationOutcome::Complete(ValidatedConfig {
                sender: sender.unwrap(),
                recipient,
                recipients: recipients_vec,
                default_recipient_key: dk,
                payment: payment.unwrap(),
                presets: presets.unwrap(),
                defaults: defaults.unwrap_or_default(),
            }))
        } else {
            Ok(ValidationOutcome::Incomplete {
                config: Config {
                    sender,
                    recipient: None, // already consumed by normalization
                    recipients,
                    default_recipient: default_key,
                    payment,
                    presets,
                    defaults,
                },
                missing,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_section_display() {
        assert_eq!(ConfigSection::Sender.to_string(), "sender");
        assert_eq!(ConfigSection::Recipient.to_string(), "recipient");
        assert_eq!(ConfigSection::Payment.to_string(), "payment");
        assert_eq!(ConfigSection::Presets.to_string(), "presets");
    }

    // ── Cycle 2 ──

    #[test]
    fn test_validate_empty_config_returns_all_missing() {
        // Act
        let result = Config::default().validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Incomplete { missing, .. } => {
                assert_eq!(
                    missing,
                    vec![
                        ConfigSection::Sender,
                        ConfigSection::Recipient,
                        ConfigSection::Payment,
                        ConfigSection::Presets,
                    ]
                );
            }
            ValidationOutcome::Complete(_) => panic!("Expected Incomplete"),
        }
    }

    // ── Cycle 3 ──

    #[test]
    fn test_validate_complete_config_returns_validated() {
        // Act
        let result = make_complete_config().validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.sender.name, "Alice");
                assert_eq!(v.recipient.name, "Bob Corp");
                assert_eq!(v.recipients.len(), 1);
                assert_eq!(v.default_recipient_key, "bob-corp");
                assert_eq!(v.recipient.key, Some("bob-corp".into()));
                assert_eq!(v.payment.len(), 1);
                assert_eq!(v.presets.len(), 1);
                assert_eq!(v.defaults.currency, "EUR");
            }
            ValidationOutcome::Incomplete { .. } => panic!("Expected Complete"),
        }
    }

    // ── Cycle 4 ──

    #[test]
    fn test_validate_missing_defaults_filled_with_default() {
        // Arrange
        let mut config = make_complete_config();
        config.defaults = None;

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.defaults.currency, "EUR");
                assert_eq!(v.defaults.invoice_date_day, 9);
                assert_eq!(v.defaults.payment_terms_days, 30);
            }
            ValidationOutcome::Incomplete { .. } => panic!("Expected Complete"),
        }
    }

    // ── Cycle 5 ──

    #[test]
    fn test_validate_sender_only_returns_three_missing() {
        // Arrange
        let config = Config {
            sender: Some(make_sender()),
            ..Config::default()
        };

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Incomplete { missing, .. } => {
                assert_eq!(
                    missing,
                    vec![
                        ConfigSection::Recipient,
                        ConfigSection::Payment,
                        ConfigSection::Presets,
                    ]
                );
            }
            ValidationOutcome::Complete(_) => panic!("Expected Incomplete"),
        }
    }

    // ── Cycle 6 ──

    #[test]
    fn test_validate_sender_and_recipient_returns_two_missing() {
        // Arrange
        let config = Config {
            sender: Some(make_sender()),
            recipient: Some(make_recipient()),
            ..Config::default()
        };

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Incomplete { missing, .. } => {
                assert_eq!(missing, vec![ConfigSection::Payment, ConfigSection::Presets]);
            }
            ValidationOutcome::Complete(_) => panic!("Expected Incomplete"),
        }
    }

    // ── Cycle 7 ──

    #[test]
    fn test_validate_empty_payment_vec_treated_as_missing() {
        // Arrange
        let mut config = make_complete_config();
        config.payment = Some(vec![]);

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Incomplete { missing, .. } => {
                assert_eq!(missing, vec![ConfigSection::Payment]);
            }
            ValidationOutcome::Complete(_) => panic!("Expected Incomplete"),
        }
    }

    // ── Cycle 8 ──

    #[test]
    fn test_validate_empty_presets_vec_treated_as_missing() {
        // Arrange
        let mut config = make_complete_config();
        config.presets = Some(vec![]);

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Incomplete { missing, .. } => {
                assert_eq!(missing, vec![ConfigSection::Presets]);
            }
            ValidationOutcome::Complete(_) => panic!("Expected Incomplete"),
        }
    }

    // ── Story 7.1 Phase 4: Recipient validation ──

    #[test]
    fn test_validate_v1_config_normalizes_to_recipients_list() {
        // Arrange — v1 style with single recipient
        let config = Config {
            sender: Some(make_sender()),
            recipient: Some(make_recipient()),
            recipients: None,
            default_recipient: None,
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
        };

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.recipients.len(), 1);
                assert_eq!(v.default_recipient_key, "bob-corp");
                assert_eq!(v.recipient.name, "Bob Corp");
                assert_eq!(v.recipient.key, Some("bob-corp".into()));
            }
            ValidationOutcome::Incomplete { .. } => panic!("Expected Complete"),
        }
    }

    #[test]
    fn test_validate_v2_config_with_multiple_recipients() {
        // Arrange
        let config = Config {
            sender: Some(make_sender()),
            recipient: None,
            recipients: Some(vec![
                Recipient {
                    key: Some("acme".into()),
                    name: "Acme Corp".into(),
                    address: vec!["123 St".into()],
                    company_id: None,
                    vat_number: None,
                },
                Recipient {
                    key: Some("globex".into()),
                    name: "Globex Inc".into(),
                    address: vec!["456 Ave".into()],
                    company_id: None,
                    vat_number: None,
                },
            ]),
            default_recipient: Some("globex".into()),
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
        };

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.recipients.len(), 2);
                assert_eq!(v.default_recipient_key, "globex");
                assert_eq!(v.recipient.name, "Globex Inc");
            }
            ValidationOutcome::Incomplete { .. } => panic!("Expected Complete"),
        }
    }

    #[test]
    fn test_validate_missing_recipients_returns_incomplete() {
        // Arrange
        let config = Config {
            sender: Some(make_sender()),
            recipient: None,
            recipients: None,
            default_recipient: None,
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
        };

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Incomplete { missing, .. } => {
                assert!(missing.contains(&ConfigSection::Recipient));
            }
            ValidationOutcome::Complete(_) => panic!("Expected Incomplete"),
        }
    }

    #[test]
    fn test_validate_empty_recipients_vec_treated_as_missing_section() {
        // Arrange
        let config = Config {
            sender: Some(make_sender()),
            recipient: None,
            recipients: Some(vec![]),
            default_recipient: None,
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
        };

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Incomplete { missing, .. } => {
                assert!(missing.contains(&ConfigSection::Recipient));
            }
            ValidationOutcome::Complete(_) => panic!("Expected Incomplete"),
        }
    }

    #[test]
    fn test_validate_invalid_default_recipient_key_returns_error() {
        // Arrange
        let config = Config {
            sender: Some(make_sender()),
            recipient: None,
            recipients: Some(vec![make_recipient_with_key()]),
            default_recipient: Some("nonexistent".into()),
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
        };

        // Act
        let result = config.validate();

        // Assert
        assert!(matches!(result, Err(AppError::InvalidDefaultRecipient(_))));
    }

    #[test]
    fn test_validate_missing_default_recipient_with_recipients_returns_error() {
        // Arrange
        let config = Config {
            sender: Some(make_sender()),
            recipient: None,
            recipients: Some(vec![make_recipient_with_key()]),
            default_recipient: None,
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
        };

        // Act
        let result = config.validate();

        // Assert
        assert!(matches!(result, Err(AppError::InvalidDefaultRecipient(_))));
    }

    #[test]
    fn test_validate_duplicate_recipient_keys_returns_error() {
        // Arrange
        let config = Config {
            sender: Some(make_sender()),
            recipient: None,
            recipients: Some(vec![
                Recipient {
                    key: Some("acme".into()),
                    name: "Acme Corp".into(),
                    address: vec!["123 St".into()],
                    company_id: None,
                    vat_number: None,
                },
                Recipient {
                    key: Some("acme".into()),
                    name: "Acme LLC".into(),
                    address: vec!["456 Ave".into()],
                    company_id: None,
                    vat_number: None,
                },
            ]),
            default_recipient: Some("acme".into()),
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
        };

        // Act
        let result = config.validate();

        // Assert
        assert!(matches!(result, Err(AppError::DuplicateRecipientKey(_))));
    }

    #[test]
    fn test_validate_empty_recipient_key_returns_error() {
        // Arrange
        let config = Config {
            sender: Some(make_sender()),
            recipient: None,
            recipients: Some(vec![Recipient {
                key: Some("".into()),
                name: "Acme Corp".into(),
                address: vec!["123 St".into()],
                company_id: None,
                vat_number: None,
            }]),
            default_recipient: Some("".into()),
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
        };

        // Act
        let result = config.validate();

        // Assert
        assert!(matches!(result, Err(AppError::InvalidDefaultRecipient(_))));
    }

    #[test]
    fn test_validate_v1_and_v2_both_present_v2_wins() {
        // Arrange — pathological: both recipient and recipients set
        let config = Config {
            sender: Some(make_sender()),
            recipient: Some(Recipient {
                key: None,
                name: "Old Corp".into(),
                address: vec!["Old St".into()],
                company_id: None,
                vat_number: None,
            }),
            recipients: Some(vec![make_recipient_with_key()]),
            default_recipient: Some("bob-corp".into()),
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
        };

        // Act
        let result = config.validate().unwrap();

        // Assert — v2 recipients wins, not the v1 recipient
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.recipient.name, "Bob Corp");
                assert_eq!(v.recipients.len(), 1);
            }
            ValidationOutcome::Incomplete { .. } => panic!("Expected Complete"),
        }
    }

    // ── Story 7.1 Phase 5: active_recipient() ──

    #[test]
    fn test_active_recipient_returns_default() {
        // Arrange
        let config = Config {
            sender: Some(make_sender()),
            recipient: None,
            recipients: Some(vec![
                Recipient {
                    key: Some("acme".into()),
                    name: "Acme Corp".into(),
                    address: vec!["123 St".into()],
                    company_id: None,
                    vat_number: None,
                },
                Recipient {
                    key: Some("globex".into()),
                    name: "Globex Inc".into(),
                    address: vec!["456 Ave".into()],
                    company_id: None,
                    vat_number: None,
                },
            ]),
            default_recipient: Some("globex".into()),
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
        };

        // Act
        let validated = match config.validate().unwrap() {
            ValidationOutcome::Complete(v) => v,
            _ => panic!("Expected Complete"),
        };

        // Assert
        assert_eq!(validated.active_recipient().name, "Globex Inc");
    }

    #[test]
    fn test_active_recipient_single_recipient() {
        // Arrange
        let config = make_complete_config();

        // Act
        let validated = match config.validate().unwrap() {
            ValidationOutcome::Complete(v) => v,
            _ => panic!("Expected Complete"),
        };

        // Assert
        assert_eq!(validated.active_recipient().name, "Bob Corp");
    }

    // ── Story 11.1: v1 backwards compatibility verification ──

    #[test]
    fn test_v1_config_round_trips_through_validation_with_single_recipient() {
        // Arrange — pure v1 config with no recipients list or default_recipient
        let config = Config {
            sender: Some(make_sender()),
            recipient: Some(make_recipient()),
            recipients: None,
            default_recipient: None,
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
        };

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.recipients.len(), 1, "v1 config should normalize to single-element recipients list");
                assert_eq!(v.recipient.name, "Bob Corp");
                assert!(!v.default_recipient_key.is_empty(), "default key should be auto-derived");
            }
            ValidationOutcome::Incomplete { .. } => panic!("Expected Complete for v1 config"),
        }
    }

    // ── Helpers ──

    fn make_sender() -> Sender {
        Sender {
            name: "Alice".into(),
            address: vec!["123 St".into()],
            email: "a@b.com".into(),
        }
    }

    fn make_recipient() -> Recipient {
        Recipient {
            key: None,
            name: "Bob Corp".into(),
            address: vec!["456 Ave".into()],
            company_id: None,
            vat_number: None,
        }
    }

    fn make_recipient_with_key() -> Recipient {
        Recipient {
            key: Some("bob-corp".into()),
            name: "Bob Corp".into(),
            address: vec!["456 Ave".into()],
            company_id: None,
            vat_number: None,
        }
    }

    fn make_payment() -> Vec<PaymentMethod> {
        vec![PaymentMethod {
            label: "SEPA".into(),
            iban: "DE00".into(),
            bic_swift: "BIC".into(),
        }]
    }

    fn make_presets() -> Vec<Preset> {
        vec![Preset {
            key: "dev".into(),
            description: "Dev".into(),
            default_rate: 100.0,
            currency: None,
            tax_rate: None,
        }]
    }

    fn make_complete_config() -> Config {
        Config {
            sender: Some(make_sender()),
            recipient: None,
            recipients: Some(vec![make_recipient_with_key()]),
            default_recipient: Some("bob-corp".into()),
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
        }
    }
}
