use std::fmt;

use super::error::ConfigError;
use super::types::*;
use crate::domain::{HexColor, RecipientKey};
use crate::locale::Locale;

const DEFAULT_ACCENT_COLOR: &str = "#2c3e50";

fn default_accent_color() -> HexColor {
    HexColor::try_new(DEFAULT_ACCENT_COLOR)
        .expect("DEFAULT_ACCENT_COLOR is a valid hex color literal")
}

/// Branding with validated values, ready for PDF generation.
///
/// As of the `HexColor` migration, `accent_color` is a parsed [`HexColor`]
/// (not a raw string). Invalid colors are now rejected at config-deserialize
/// time, so this struct is effectively a passthrough that fills in defaults
/// for missing fields.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedBranding {
    /// Raw logo path from config (resolved to absolute path later in pdf module).
    pub logo: Option<String>,
    /// Validated hex color (`#rrggbb`, lowercase).
    pub accent_color: HexColor,
    /// Font family name override, or None for default.
    pub font: Option<String>,
    /// Custom footer text, or None for default.
    pub footer_text: Option<String>,
}

impl Default for ValidatedBranding {
    fn default() -> Self {
        Self {
            logo: None,
            accent_color: default_accent_color(),
            font: None,
            footer_text: None,
        }
    }
}

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
    pub default_recipient_key: RecipientKey,
    /// Guaranteed non-empty.
    pub payment: Vec<PaymentMethod>,
    /// Guaranteed non-empty.
    pub presets: Vec<Preset>,
    pub defaults: Defaults,
    pub branding: ValidatedBranding,
    pub template: TemplateKey,
    pub locale: Locale,
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
    /// Returns `Err(ConfigError)` for hard errors like duplicate keys or invalid
    /// default recipient references.
    pub fn validate(self) -> Result<ValidationOutcome, ConfigError> {
        let mut missing = Vec::new();

        let sender = self.sender;
        let payment = self.payment;
        let presets = self.presets;
        let defaults = self.defaults;
        let branding = self.branding;

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
                    let key = match r.key.clone() {
                        Some(k) => k,
                        None => match RecipientKey::from_name(&r.name) {
                            Ok(k) => k,
                            Err(e) => {
                                return Err(ConfigError::InvalidDefaultRecipient(e.to_string()));
                            }
                        },
                    };
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
            // Every recipient must have a key.
            for r in list {
                if r.key.is_none() {
                    return Err(ConfigError::InvalidDefaultRecipient(
                        "recipient has missing key".into(),
                    ));
                }
            }
            // Check for duplicate keys
            let mut seen = std::collections::HashSet::new();
            for r in list {
                let k = r.key.as_ref().unwrap();
                if !seen.insert(k.clone()) {
                    return Err(ConfigError::DuplicateRecipientKey(k.as_str().to_string()));
                }
            }
            // Validate default_recipient references a valid key
            match &default_key {
                Some(dk) => {
                    if !list.iter().any(|r| r.key.as_ref() == Some(dk)) {
                        return Err(ConfigError::InvalidDefaultRecipient(dk.as_str().to_string()));
                    }
                }
                None => {
                    return Err(ConfigError::MissingDefaultRecipient);
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
                .find(|r| r.key.as_ref() == Some(&dk))
                .cloned()
                .unwrap();

            let resolved_defaults = defaults.unwrap_or_default();
            let template = resolved_defaults.template;
            let locale = resolved_defaults.locale;
            Ok(ValidationOutcome::Complete(ValidatedConfig {
                sender: sender.unwrap(),
                recipient,
                recipients: recipients_vec,
                default_recipient_key: dk,
                payment: payment.unwrap(),
                presets: presets.unwrap(),
                defaults: resolved_defaults,
                branding: match branding {
                    Some(b) => ValidatedBranding {
                        logo: b.logo,
                        accent_color: b.accent_color.unwrap_or_else(default_accent_color),
                        font: b.font,
                        footer_text: b.footer_text,
                    },
                    None => ValidatedBranding::default(),
                },
                template,
                locale,
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
                    branding,
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
                assert_eq!(v.default_recipient_key.as_str(), "bob-corp");
                assert_eq!(v.recipient.key, Some(RecipientKey::try_new("bob-corp").unwrap()));
                assert_eq!(v.payment.len(), 1);
                assert_eq!(v.presets.len(), 1);
                assert_eq!(v.defaults.currency, crate::domain::Currency::Eur);
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
                assert_eq!(v.defaults.currency, crate::domain::Currency::Eur);
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
            branding: None,
        };

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.recipients.len(), 1);
                assert_eq!(v.default_recipient_key.as_str(), "bob-corp");
                assert_eq!(v.recipient.name, "Bob Corp");
                assert_eq!(v.recipient.key, Some(RecipientKey::try_new("bob-corp").unwrap()));
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
                    key: Some(RecipientKey::try_new("acme").unwrap()),
                    name: "Acme Corp".into(),
                    address: vec!["123 St".into()],
                    company_id: None,
                    vat_number: None,
                },
                Recipient {
                    key: Some(RecipientKey::try_new("globex").unwrap()),
                    name: "Globex Inc".into(),
                    address: vec!["456 Ave".into()],
                    company_id: None,
                    vat_number: None,
                },
            ]),
            default_recipient: Some(RecipientKey::try_new("globex").unwrap()),
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
            branding: None,
        };

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.recipients.len(), 2);
                assert_eq!(v.default_recipient_key.as_str(), "globex");
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
            branding: None,
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
            branding: None,
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
            default_recipient: Some(RecipientKey::try_new("nonexistent").unwrap()),
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
            branding: None,
        };

        // Act
        let result = config.validate();

        // Assert
        assert!(matches!(result, Err(ConfigError::InvalidDefaultRecipient(_))));
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
            branding: None,
        };

        // Act
        let result = config.validate();

        // Assert
        assert!(matches!(result, Err(ConfigError::MissingDefaultRecipient)));
    }

    #[test]
    fn test_validate_duplicate_recipient_keys_returns_error() {
        // Arrange
        let config = Config {
            sender: Some(make_sender()),
            recipient: None,
            recipients: Some(vec![
                Recipient {
                    key: Some(RecipientKey::try_new("acme").unwrap()),
                    name: "Acme Corp".into(),
                    address: vec!["123 St".into()],
                    company_id: None,
                    vat_number: None,
                },
                Recipient {
                    key: Some(RecipientKey::try_new("acme").unwrap()),
                    name: "Acme LLC".into(),
                    address: vec!["456 Ave".into()],
                    company_id: None,
                    vat_number: None,
                },
            ]),
            default_recipient: Some(RecipientKey::try_new("acme").unwrap()),
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
            branding: None,
        };

        // Act
        let result = config.validate();

        // Assert
        assert!(matches!(result, Err(ConfigError::DuplicateRecipientKey(_))));
    }

    #[test]
    fn test_empty_recipient_key_rejected_at_deserialize() {
        // Arrange — empty keys are no longer constructible via RecipientKey,
        // so the failure path is at YAML parse time, not validate().
        let yaml = "key: \"\"\nname: Acme Corp\naddress:\n  - 123 St\n";

        // Act
        let result: Result<Recipient, _> = serde_yaml::from_str(yaml);

        // Assert
        assert!(result.is_err());
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
            default_recipient: Some(RecipientKey::try_new("bob-corp").unwrap()),
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
            branding: None,
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
                    key: Some(RecipientKey::try_new("acme").unwrap()),
                    name: "Acme Corp".into(),
                    address: vec!["123 St".into()],
                    company_id: None,
                    vat_number: None,
                },
                Recipient {
                    key: Some(RecipientKey::try_new("globex").unwrap()),
                    name: "Globex Inc".into(),
                    address: vec!["456 Ave".into()],
                    company_id: None,
                    vat_number: None,
                },
            ]),
            default_recipient: Some(RecipientKey::try_new("globex").unwrap()),
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
            branding: None,
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
            branding: None,
        };

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.recipients.len(), 1, "v1 config should normalize to single-element recipients list");
                assert_eq!(v.recipient.name, "Bob Corp");
                assert!(!v.default_recipient_key.as_str().is_empty(), "default key should be auto-derived");
            }
            ValidationOutcome::Incomplete { .. } => panic!("Expected Complete for v1 config"),
        }
    }

    // ── Story 12.1 Cycle 6: ValidatedConfig.template ──

    #[test]
    fn test_validated_config_includes_template_from_defaults() {
        // Arrange
        let config = make_complete_config();

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.template, TemplateKey::Leda);
            }
            ValidationOutcome::Incomplete { .. } => panic!("Expected Complete"),
        }
    }

    #[test]
    fn test_validated_config_template_custom_value() {
        // Arrange
        let mut config = make_complete_config();
        config.defaults = Some(Defaults {
            template: TemplateKey::Callisto,
            ..Defaults::default()
        });

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.template, TemplateKey::Callisto);
            }
            ValidationOutcome::Incomplete { .. } => panic!("Expected Complete"),
        }
    }

    #[test]
    fn test_validated_config_missing_defaults_gets_leda_template() {
        // Arrange
        let mut config = make_complete_config();
        config.defaults = None;

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.template, TemplateKey::Leda);
            }
            ValidationOutcome::Incomplete { .. } => panic!("Expected Complete"),
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
            key: Some(RecipientKey::try_new("bob-corp").unwrap()),
            name: "Bob Corp".into(),
            address: vec!["456 Ave".into()],
            company_id: None,
            vat_number: None,
        }
    }

    fn make_payment() -> Vec<PaymentMethod> {
        vec![PaymentMethod {
            label: "SEPA".into(),
            iban: crate::domain::Iban::try_new("DE89370400440532013000").unwrap(),
            bic_swift: "BIC".into(),
        }]
    }

    fn make_presets() -> Vec<Preset> {
        vec![Preset {
            key: crate::domain::PresetKey::try_new("dev").unwrap(),
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
            default_recipient: Some(RecipientKey::try_new("bob-corp").unwrap()),
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
            branding: None,
        }
    }

    // ── Sprint 10: ValidatedBranding integration tests ──

    #[test]
    fn test_validate_no_branding_uses_defaults() {
        // Arrange
        let config = make_complete_config(); // branding: None

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.branding.accent_color.as_str(), "#2c3e50");
                assert!(v.branding.font.is_none());
                assert!(v.branding.footer_text.is_none());
                assert!(v.branding.logo.is_none());
            }
            _ => panic!("Expected Complete"),
        }
    }

    #[test]
    fn test_validate_branding_accent_color_passthrough() {
        // Arrange
        let mut config = make_complete_config();
        config.branding = Some(crate::config::types::Branding {
            accent_color: Some(HexColor::try_new("#ff0000").unwrap()),
            ..Default::default()
        });

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.branding.accent_color.as_str(), "#ff0000");
            }
            _ => panic!("Expected Complete"),
        }
    }

    #[test]
    fn test_validate_branding_missing_accent_uses_default() {
        // Arrange — Branding present but accent_color None still falls back.
        let mut config = make_complete_config();
        config.branding = Some(crate::config::types::Branding {
            accent_color: None,
            ..Default::default()
        });

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.branding.accent_color.as_str(), "#2c3e50");
            }
            _ => panic!("Expected Complete"),
        }
    }

    #[test]
    fn test_validate_branding_font_passthrough() {
        // Arrange
        let mut config = make_complete_config();
        config.branding = Some(crate::config::types::Branding {
            font: Some("Fira Code".into()),
            ..Default::default()
        });

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.branding.font, Some("Fira Code".into()));
            }
            _ => panic!("Expected Complete"),
        }
    }

    #[test]
    fn test_validate_branding_footer_text_passthrough() {
        // Arrange
        let mut config = make_complete_config();
        config.branding = Some(crate::config::types::Branding {
            footer_text: Some("Thanks!".into()),
            ..Default::default()
        });

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.branding.footer_text, Some("Thanks!".into()));
            }
            _ => panic!("Expected Complete"),
        }
    }

    #[test]
    fn test_validate_branding_logo_passthrough() {
        // Arrange
        let mut config = make_complete_config();
        config.branding = Some(crate::config::types::Branding {
            logo: Some("logo.png".into()),
            ..Default::default()
        });

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.branding.logo, Some("logo.png".into()));
            }
            _ => panic!("Expected Complete"),
        }
    }
}
