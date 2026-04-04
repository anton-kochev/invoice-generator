use serde::{Deserialize, Serialize};

/// Top-level invoice generator configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Config {
    /// Sender / freelancer info.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender: Option<Sender>,
    /// Default recipient / client info.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipient: Option<Recipient>,
    /// Available payment methods.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment: Option<Vec<PaymentMethod>>,
    /// v2 multi-recipient list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipients: Option<Vec<Recipient>>,
    /// Key of the default recipient profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_recipient: Option<String>,
    /// Invoice presets (e.g. hourly-rate templates).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presets: Option<Vec<Preset>>,
    /// Default values for new invoices.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defaults: Option<Defaults>,
}

/// Information about the invoice sender.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Sender {
    pub name: String,
    pub address: Vec<String>,
    pub email: String,
}

/// Information about the invoice recipient.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Recipient {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    pub name: String,
    pub address: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub company_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "vat")]
    pub vat_number: Option<String>,
}

/// A payment method shown on the invoice.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaymentMethod {
    pub label: String,
    pub iban: String,
    #[serde(alias = "bic")]
    pub bic_swift: String,
}

/// An invoice preset / template.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Preset {
    pub key: String,
    pub description: String,
    pub default_rate: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_rate: Option<f64>,
}

fn default_currency() -> String {
    "EUR".to_string()
}

const fn default_invoice_date_day() -> u32 {
    9
}

const fn default_payment_terms_days() -> u32 {
    30
}

/// Default values applied to new invoices.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Defaults {
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default = "default_invoice_date_day")]
    pub invoice_date_day: u32,
    #[serde(default = "default_payment_terms_days")]
    pub payment_terms_days: u32,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            currency: default_currency(),
            invoice_date_day: default_invoice_date_day(),
            payment_terms_days: default_payment_terms_days(),
        }
    }
}

/// Derive a slug key from a recipient name.
/// Lowercases, replaces non-alphanumeric characters with hyphens, collapses runs.
pub fn derive_recipient_key(name: &str) -> String {
    name.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_key_simple_two_words() {
        // Arrange
        let name = "Acme Corp";

        // Act
        let key = derive_recipient_key(name);

        // Assert
        assert_eq!(key, "acme-corp");
    }

    #[test]
    fn derive_key_single_word() {
        // Arrange
        let name = "Bob";

        // Act
        let key = derive_recipient_key(name);

        // Assert
        assert_eq!(key, "bob");
    }

    #[test]
    fn derive_key_punctuation_stripped() {
        // Arrange
        let name = "Foo & Bar, Inc.";

        // Act
        let key = derive_recipient_key(name);

        // Assert
        assert_eq!(key, "foo-bar-inc");
    }

    #[test]
    fn derive_key_whitespace_only_returns_empty() {
        // Arrange
        let name = "   ";

        // Act
        let key = derive_recipient_key(name);

        // Assert
        assert_eq!(key, "");
    }

    #[test]
    fn derive_key_empty_string_returns_empty() {
        // Arrange
        let name = "";

        // Act
        let key = derive_recipient_key(name);

        // Assert
        assert_eq!(key, "");
    }

    #[test]
    fn derive_key_preserves_existing_hyphens() {
        // Arrange
        let name = "Müller-Schmidt GmbH";

        // Act
        let key = derive_recipient_key(name);

        // Assert
        assert_eq!(key, "müller-schmidt-gmbh");
    }

    #[test]
    fn v1_yaml_deserializes_with_key_none() {
        // Arrange
        let yaml = "name: Acme Corp\naddress:\n  - 123 Main St\n";

        // Act
        let r: Recipient = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert!(r.key.is_none());
        assert_eq!(r.name, "Acme Corp");
    }

    #[test]
    fn v2_yaml_with_key_deserializes() {
        // Arrange
        let yaml = "key: acme\nname: Acme Corp\naddress:\n  - 123 Main St\n";

        // Act
        let r: Recipient = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert_eq!(r.key, Some("acme".into()));
    }

    #[test]
    fn config_recipients_none_omitted_from_yaml() {
        // Arrange
        let config = Config {
            recipients: None,
            default_recipient: None,
            ..Config::default()
        };

        // Act
        let yaml = serde_yaml::to_string(&config).unwrap();

        // Assert
        assert!(!yaml.contains("recipients"), "None recipients should be omitted from YAML");
        assert!(!yaml.contains("default_recipient"), "None default_recipient should be omitted");
    }

    #[test]
    fn v2_config_with_recipients_round_trips() {
        // Arrange
        let config = Config {
            recipients: Some(vec![
                Recipient {
                    key: Some("acme".into()),
                    name: "Acme Corp".into(),
                    address: vec!["123 Main St".into()],
                    company_id: None,
                    vat_number: None,
                },
            ]),
            default_recipient: Some("acme".into()),
            ..Config::default()
        };

        // Act
        let yaml = serde_yaml::to_string(&config).unwrap();
        let loaded: Config = serde_yaml::from_str(&yaml).unwrap();

        // Assert
        assert_eq!(loaded.recipients.as_ref().unwrap().len(), 1);
        assert_eq!(loaded.default_recipient, Some("acme".into()));
    }

    #[test]
    fn test_preset_without_currency_deserializes_as_none() {
        // Arrange
        let yaml = "key: dev\ndescription: Development\ndefault_rate: 800.0\n";

        // Act
        let preset: Preset = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert!(preset.currency.is_none());
    }

    #[test]
    fn test_preset_with_currency_deserializes() {
        // Arrange
        let yaml = "key: dev\ndescription: Development\ndefault_rate: 800.0\ncurrency: USD\n";

        // Act
        let preset: Preset = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert_eq!(preset.currency, Some("USD".into()));
    }

    #[test]
    fn test_preset_currency_none_omitted_from_yaml() {
        // Arrange
        let preset = Preset {
            key: "dev".into(),
            description: "Development".into(),
            default_rate: 800.0,
            currency: None,
            tax_rate: None,
        };

        // Act
        let yaml = serde_yaml::to_string(&preset).unwrap();

        // Assert
        assert!(!yaml.contains("currency"), "None currency should be omitted from YAML");
    }

    #[test]
    fn test_preset_with_currency_round_trips() {
        // Arrange
        let preset = Preset {
            key: "dev".into(),
            description: "Development".into(),
            default_rate: 800.0,
            currency: Some("CZK".into()),
            tax_rate: None,
        };

        // Act
        let yaml = serde_yaml::to_string(&preset).unwrap();
        let loaded: Preset = serde_yaml::from_str(&yaml).unwrap();

        // Assert
        assert_eq!(loaded.currency, Some("CZK".into()));
    }

    #[test]
    fn test_preset_without_tax_rate_deserializes_as_none() {
        // Arrange
        let yaml = "key: dev\ndescription: Development\ndefault_rate: 800.0\n";

        // Act
        let preset: Preset = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert!(preset.tax_rate.is_none());
    }

    #[test]
    fn test_preset_with_tax_rate_deserializes() {
        // Arrange
        let yaml = "key: dev\ndescription: Development\ndefault_rate: 800.0\ntax_rate: 21.0\n";

        // Act
        let preset: Preset = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert_eq!(preset.tax_rate, Some(21.0));
    }

    #[test]
    fn test_preset_tax_rate_none_omitted_from_yaml() {
        // Arrange
        let preset = Preset {
            key: "dev".into(),
            description: "Development".into(),
            default_rate: 800.0,
            currency: None,
            tax_rate: None,
        };

        // Act
        let yaml = serde_yaml::to_string(&preset).unwrap();

        // Assert
        assert!(!yaml.contains("tax_rate"), "None tax_rate should be omitted from YAML");
    }

    #[test]
    fn test_preset_with_tax_rate_round_trips() {
        // Arrange
        let preset = Preset {
            key: "dev".into(),
            description: "Development".into(),
            default_rate: 800.0,
            currency: None,
            tax_rate: Some(21.0),
        };

        // Act
        let yaml = serde_yaml::to_string(&preset).unwrap();
        let loaded: Preset = serde_yaml::from_str(&yaml).unwrap();

        // Assert
        assert_eq!(loaded.tax_rate, Some(21.0));
    }

    #[test]
    fn test_preset_with_zero_tax_rate_serializes() {
        // Arrange
        let preset = Preset {
            key: "dev".into(),
            description: "Development".into(),
            default_rate: 800.0,
            currency: None,
            tax_rate: Some(0.0),
        };

        // Act
        let yaml = serde_yaml::to_string(&preset).unwrap();

        // Assert
        assert!(yaml.contains("tax_rate: 0.0"), "Zero tax_rate should be serialized, got: {yaml}");
    }
}
