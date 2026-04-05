use std::fmt;

use super::types::*;
use crate::error::AppError;

const DEFAULT_ACCENT_COLOR: &str = "#2c3e50";

/// Validate a hex color string. Returns normalized #RRGGBB or None for invalid input.
/// Accepts #RGB (expands to #RRGGBB) and #RRGGBB formats.
fn validate_accent_color(input: &str) -> Option<String> {
    let s = input.trim();
    if !s.starts_with('#') {
        return None;
    }
    let hex = &s[1..];
    match hex.len() {
        3 => {
            if hex.chars().all(|c| c.is_ascii_hexdigit()) {
                let expanded: String = hex.chars().flat_map(|c| [c, c]).collect();
                Some(format!("#{}", expanded.to_lowercase()))
            } else {
                None
            }
        }
        6 => {
            if hex.chars().all(|c| c.is_ascii_hexdigit()) {
                Some(format!("#{}", hex.to_lowercase()))
            } else {
                None
            }
        }
        _ => None,
    }
}

/// Branding with validated values, ready for PDF generation.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedBranding {
    /// Raw logo path from config (resolved to absolute path later in pdf module).
    pub logo: Option<String>,
    /// Validated hex color string (always 7-char #rrggbb).
    pub accent_color: String,
    /// Font family name override, or None for default.
    pub font: Option<String>,
    /// Custom footer text, or None for default.
    pub footer_text: Option<String>,
}

impl Default for ValidatedBranding {
    fn default() -> Self {
        Self {
            logo: None,
            accent_color: DEFAULT_ACCENT_COLOR.to_string(),
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
    pub default_recipient_key: String,
    /// Guaranteed non-empty.
    pub payment: Vec<PaymentMethod>,
    /// Guaranteed non-empty.
    pub presets: Vec<Preset>,
    pub defaults: Defaults,
    pub branding: ValidatedBranding,
    pub template: TemplateKey,
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
                    return Err(AppError::MissingDefaultRecipient);
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

            let resolved_defaults = defaults.unwrap_or_default();
            let template = resolved_defaults.template;
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
                        accent_color: match &b.accent_color {
                            Some(c) => validate_accent_color(c).unwrap_or_else(|| {
                                eprintln!(
                                    "Warning: invalid accent_color \"{c}\", using default {DEFAULT_ACCENT_COLOR}"
                                );
                                DEFAULT_ACCENT_COLOR.to_string()
                            }),
                            None => DEFAULT_ACCENT_COLOR.to_string(),
                        },
                        font: b.font,
                        footer_text: b.footer_text,
                    },
                    None => ValidatedBranding::default(),
                },
                template,
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
            branding: None,
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
            branding: None,
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
            default_recipient: Some("nonexistent".into()),
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
            branding: None,
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
            branding: None,
        };

        // Act
        let result = config.validate();

        // Assert
        assert!(matches!(result, Err(AppError::MissingDefaultRecipient)));
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
            branding: None,
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
            branding: None,
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
                assert!(!v.default_recipient_key.is_empty(), "default key should be auto-derived");
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
            branding: None,
        }
    }

    // ── Sprint 10: validate_accent_color pure function tests ──

    #[test]
    fn test_validate_accent_color_valid_six_digit() {
        // Arrange & Act & Assert
        assert_eq!(validate_accent_color("#2c3e50"), Some("#2c3e50".into()));
    }

    #[test]
    fn test_validate_accent_color_uppercase_normalized() {
        // Arrange & Act & Assert
        assert_eq!(validate_accent_color("#AABBCC"), Some("#aabbcc".into()));
    }

    #[test]
    fn test_validate_accent_color_mixed_case_normalized() {
        // Arrange & Act & Assert
        assert_eq!(validate_accent_color("#aAbBcC"), Some("#aabbcc".into()));
    }

    #[test]
    fn test_validate_accent_color_three_digit_expanded() {
        // Arrange & Act & Assert
        assert_eq!(validate_accent_color("#abc"), Some("#aabbcc".into()));
    }

    #[test]
    fn test_validate_accent_color_missing_hash_returns_none() {
        // Arrange & Act & Assert
        assert_eq!(validate_accent_color("2c3e50"), None);
    }

    #[test]
    fn test_validate_accent_color_invalid_chars_returns_none() {
        // Arrange & Act & Assert
        assert_eq!(validate_accent_color("#zzzzzz"), None);
    }

    #[test]
    fn test_validate_accent_color_wrong_length_returns_none() {
        // Arrange & Act & Assert
        assert_eq!(validate_accent_color("#12345"), None);
    }

    #[test]
    fn test_validate_accent_color_empty_returns_none() {
        // Arrange & Act & Assert
        assert_eq!(validate_accent_color(""), None);
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
                assert_eq!(v.branding.accent_color, "#2c3e50");
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
            accent_color: Some("#ff0000".into()),
            ..Default::default()
        });

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.branding.accent_color, "#ff0000");
            }
            _ => panic!("Expected Complete"),
        }
    }

    #[test]
    fn test_validate_branding_invalid_accent_falls_back_to_default() {
        // Arrange
        let mut config = make_complete_config();
        config.branding = Some(crate::config::types::Branding {
            accent_color: Some("red".into()),
            ..Default::default()
        });

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.branding.accent_color, "#2c3e50");
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
