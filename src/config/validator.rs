use std::fmt;

use super::types::*;

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
    /// Guaranteed non-empty.
    pub payment: Vec<PaymentMethod>,
    /// Guaranteed non-empty.
    pub presets: Vec<Preset>,
    pub defaults: Defaults,
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
    /// Returns [`ValidationOutcome::Complete`] with a [`ValidatedConfig`] when
    /// all sections are filled in, or [`ValidationOutcome::Incomplete`] listing
    /// which sections are missing.
    pub fn validate(self) -> ValidationOutcome {
        let mut missing = Vec::new();

        if self.sender.is_none() {
            missing.push(ConfigSection::Sender);
        }
        if self.recipient.is_none() {
            missing.push(ConfigSection::Recipient);
        }
        match &self.payment {
            Some(v) if !v.is_empty() => {}
            _ => missing.push(ConfigSection::Payment),
        }
        match &self.presets {
            Some(v) if !v.is_empty() => {}
            _ => missing.push(ConfigSection::Presets),
        }

        if missing.is_empty() {
            ValidationOutcome::Complete(ValidatedConfig {
                sender: self.sender.unwrap(),
                recipient: self.recipient.unwrap(),
                payment: self.payment.unwrap(),
                presets: self.presets.unwrap(),
                defaults: self.defaults.unwrap_or_default(),
            })
        } else {
            ValidationOutcome::Incomplete {
                config: self,
                missing,
            }
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
        let result = Config::default().validate();

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
        let result = make_complete_config().validate();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.sender.name, "Alice");
                assert_eq!(v.recipient.name, "Bob Corp");
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
        let result = config.validate();

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
        let result = config.validate();

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
        let result = config.validate();

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
        let result = config.validate();

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
        let result = config.validate();

        // Assert
        match result {
            ValidationOutcome::Incomplete { missing, .. } => {
                assert_eq!(missing, vec![ConfigSection::Presets]);
            }
            ValidationOutcome::Complete(_) => panic!("Expected Incomplete"),
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
        }]
    }

    fn make_complete_config() -> Config {
        Config {
            sender: Some(make_sender()),
            recipient: Some(make_recipient()),
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
        }
    }
}
