use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::domain::HexColor;
use crate::error::AppError;
use crate::locale::Locale;

/// Available invoice template styles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TemplateKey {
    Callisto,
    #[default]
    Leda,
    Thebe,
    Amalthea,
    Metis,
}

impl TemplateKey {
    /// All available template keys.
    pub const ALL: [TemplateKey; 5] = [
        TemplateKey::Callisto,
        TemplateKey::Leda,
        TemplateKey::Thebe,
        TemplateKey::Amalthea,
        TemplateKey::Metis,
    ];

    /// Short description of the template style.
    pub fn description(&self) -> &'static str {
        match self {
            Self::Callisto => "Bold & structured",
            Self::Leda => "Clean & minimal",
            Self::Thebe => "Compact & dense",
            Self::Amalthea => "High-contrast & vivid",
            Self::Metis => "Bare-bones & printable",
        }
    }
}

impl fmt::Display for TemplateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Callisto => "callisto",
            Self::Leda => "leda",
            Self::Thebe => "thebe",
            Self::Amalthea => "amalthea",
            Self::Metis => "metis",
        };
        write!(f, "{s}")
    }
}

impl FromStr for TemplateKey {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "callisto" => Ok(Self::Callisto),
            "leda" => Ok(Self::Leda),
            "thebe" => Ok(Self::Thebe),
            "amalthea" => Ok(Self::Amalthea),
            "metis" => Ok(Self::Metis),
            _ => Err(AppError::InvalidTemplateKey {
                key: s.to_string(),
                available: Self::ALL.iter().map(|k| k.to_string()).collect(),
            }),
        }
    }
}

/// Branding options for invoice appearance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Branding {
    /// Path to the logo image file.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub logo: Option<String>,
    /// Accent color (e.g. hex code like "#ff0000"). Validated at deserialize-time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accent_color: Option<HexColor>,
    /// Font family name.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font: Option<String>,
    /// Custom footer text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub footer_text: Option<String>,
}

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
    /// Branding / appearance options.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branding: Option<Branding>,
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
    #[serde(default)]
    pub template: TemplateKey,
    #[serde(default, deserialize_with = "crate::locale::deserialize_locale_lenient")]
    pub locale: Locale,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            currency: default_currency(),
            invoice_date_day: default_invoice_date_day(),
            payment_terms_days: default_payment_terms_days(),
            template: TemplateKey::default(),
            locale: Locale::default(),
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

    #[test]
    fn test_config_without_branding_deserializes_as_none() {
        // Arrange — YAML with no branding section (like existing v1 configs)
        let yaml = "sender:\n  name: Alice\n  address:\n    - 123 St\n  email: a@b.com\n";

        // Act
        let config: Config = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert!(config.branding.is_none());
    }

    #[test]
    fn test_branding_none_omitted_from_yaml() {
        // Arrange
        let config = Config { branding: None, ..Config::default() };

        // Act
        let yaml = serde_yaml::to_string(&config).unwrap();

        // Assert
        assert!(!yaml.contains("branding"), "None branding should be omitted from YAML");
    }

    #[test]
    fn test_branding_all_fields_round_trips() {
        // Arrange
        let branding = Branding {
            logo: Some("logo.png".into()),
            accent_color: Some(HexColor::try_new("#ff0000").unwrap()),
            font: Some("Fira Code".into()),
            footer_text: Some("Custom footer".into()),
        };
        let config = Config { branding: Some(branding), ..Config::default() };

        // Act
        let yaml = serde_yaml::to_string(&config).unwrap();
        let loaded: Config = serde_yaml::from_str(&yaml).unwrap();

        // Assert
        let b = loaded.branding.unwrap();
        assert_eq!(b.logo, Some("logo.png".into()));
        assert_eq!(b.accent_color.as_ref().map(|c| c.as_str()), Some("#ff0000"));
        assert_eq!(b.font, Some("Fira Code".into()));
        assert_eq!(b.footer_text, Some("Custom footer".into()));
    }

    #[test]
    fn test_branding_partial_fields_round_trips() {
        // Arrange — only accent_color set (full 6-digit form; #RGB short form
        // is rejected by HexColor)
        let branding = Branding {
            accent_color: Some(HexColor::try_new("#aabbcc").unwrap()),
            ..Branding::default()
        };
        let config = Config { branding: Some(branding), ..Config::default() };

        // Act
        let yaml = serde_yaml::to_string(&config).unwrap();
        let loaded: Config = serde_yaml::from_str(&yaml).unwrap();

        // Assert
        let b = loaded.branding.unwrap();
        assert_eq!(b.accent_color.as_ref().map(|c| c.as_str()), Some("#aabbcc"));
        assert!(b.logo.is_none());
        assert!(b.font.is_none());
        assert!(b.footer_text.is_none());
    }

    #[test]
    fn test_branding_short_form_hex_rejected_at_deserialize() {
        // Arrange — `#abc` short form is no longer accepted; loading must fail.
        let yaml = "branding:\n  accent_color: \"#abc\"\n";

        // Act
        let result: Result<Config, _> = serde_yaml::from_str(yaml);

        // Assert
        assert!(result.is_err(), "Expected deserialize failure for #abc short form");
    }

    #[test]
    fn test_branding_invalid_hex_rejected_at_deserialize() {
        // Arrange — non-hex chars
        let yaml = "branding:\n  accent_color: \"red\"\n";

        // Act
        let result: Result<Config, _> = serde_yaml::from_str(yaml);

        // Assert
        assert!(result.is_err(), "Expected deserialize failure for non-hex value");
    }

    #[test]
    fn test_branding_empty_struct_round_trips() {
        // Arrange — all fields None
        let config = Config { branding: Some(Branding::default()), ..Config::default() };

        // Act
        let yaml = serde_yaml::to_string(&config).unwrap();
        let loaded: Config = serde_yaml::from_str(&yaml).unwrap();

        // Assert
        assert!(loaded.branding.is_some());
    }

    // ── Story 12.1 Cycle 1: TemplateKey basics ──

    #[test]
    fn test_template_key_default_is_leda() {
        // Arrange & Act
        let key = TemplateKey::default();

        // Assert
        assert_eq!(key, TemplateKey::Leda);
    }

    #[test]
    fn test_template_key_all_has_five_variants() {
        // Arrange & Act & Assert
        assert_eq!(TemplateKey::ALL.len(), 5);
    }

    #[test]
    fn test_template_key_all_contains_all_variants() {
        // Arrange
        let all = TemplateKey::ALL;

        // Act & Assert
        assert!(all.contains(&TemplateKey::Callisto));
        assert!(all.contains(&TemplateKey::Leda));
        assert!(all.contains(&TemplateKey::Thebe));
        assert!(all.contains(&TemplateKey::Amalthea));
        assert!(all.contains(&TemplateKey::Metis));
    }

    #[test]
    fn test_template_key_display_leda() {
        // Arrange
        let key = TemplateKey::Leda;

        // Act
        let display = format!("{key}");

        // Assert
        assert_eq!(display, "leda");
    }

    #[test]
    fn test_template_key_display_all_lowercase() {
        // Arrange & Act & Assert
        for key in TemplateKey::ALL {
            let display = format!("{key}");
            assert_eq!(display, display.to_lowercase(), "Display for {key:?} should be lowercase");
        }
    }

    #[test]
    fn test_template_key_description_leda() {
        // Arrange
        let key = TemplateKey::Leda;

        // Act
        let desc = key.description();

        // Assert
        assert_eq!(desc, "Clean & minimal");
    }

    #[test]
    fn test_template_key_description_all_unique() {
        // Arrange
        let descriptions: Vec<&str> = TemplateKey::ALL.iter().map(|k| k.description()).collect();

        // Act & Assert
        let mut seen = std::collections::HashSet::new();
        for d in &descriptions {
            assert!(seen.insert(d), "Duplicate description: {d}");
        }
    }

    // ── Story 12.1 Cycle 2: FromStr ──

    #[test]
    fn test_template_key_from_str_leda() {
        // Arrange & Act
        let key: TemplateKey = "leda".parse().unwrap();

        // Assert
        assert_eq!(key, TemplateKey::Leda);
    }

    #[test]
    fn test_template_key_from_str_all_valid_keys() {
        // Arrange
        let names = ["callisto", "leda", "thebe", "amalthea", "metis"];

        // Act & Assert
        for name in names {
            let result: Result<TemplateKey, _> = name.parse();
            assert!(result.is_ok(), "Should parse '{name}' as a valid TemplateKey");
        }
    }

    #[test]
    fn test_template_key_from_str_invalid_returns_error() {
        // Arrange & Act
        let result: Result<TemplateKey, _> = "europa".parse();

        // Assert
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("europa"), "Error should contain the invalid key");
        assert!(msg.contains("leda"), "Error should list available keys");
    }

    #[test]
    fn test_template_key_from_str_empty_returns_error() {
        // Arrange & Act
        let result: Result<TemplateKey, _> = "".parse();

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_template_key_from_str_case_insensitive() {
        // Arrange & Act & Assert
        assert_eq!("CALLISTO".parse::<TemplateKey>().unwrap(), TemplateKey::Callisto);
        assert_eq!("Leda".parse::<TemplateKey>().unwrap(), TemplateKey::Leda);
        assert_eq!("THEBE".parse::<TemplateKey>().unwrap(), TemplateKey::Thebe);
        assert_eq!("AmAlThEa".parse::<TemplateKey>().unwrap(), TemplateKey::Amalthea);
        assert_eq!("Metis".parse::<TemplateKey>().unwrap(), TemplateKey::Metis);
    }

    // ── Story 12.1 Cycle 3: serde ──

    #[test]
    fn test_template_key_serializes_as_lowercase_string() {
        // Arrange
        let key = TemplateKey::Callisto;

        // Act
        let yaml = serde_yaml::to_string(&key).unwrap();

        // Assert
        assert_eq!(yaml.trim(), "callisto");
    }

    #[test]
    fn test_template_key_all_keys_round_trip_through_yaml() {
        // Arrange & Act & Assert
        for key in TemplateKey::ALL {
            let yaml = serde_yaml::to_string(&key).unwrap();
            let loaded: TemplateKey = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(loaded, key, "Round trip failed for {key:?}");
        }
    }

    #[test]
    fn test_template_key_deserializes_from_lowercase_string() {
        // Arrange
        let yaml = "amalthea";

        // Act
        let key: TemplateKey = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert_eq!(key, TemplateKey::Amalthea);
    }

    #[test]
    fn test_template_key_invalid_yaml_returns_error() {
        // Arrange
        let yaml = "ganymede";

        // Act
        let result: Result<TemplateKey, _> = serde_yaml::from_str(yaml);

        // Assert
        assert!(result.is_err());
    }

    // ── Story 12.1 Cycle 4: Defaults.template ──

    #[test]
    fn test_defaults_default_includes_template_leda() {
        // Arrange & Act
        let defaults = Defaults::default();

        // Assert
        assert_eq!(defaults.template, TemplateKey::Leda);
    }

    #[test]
    fn test_defaults_without_template_field_deserializes_as_leda() {
        // Arrange — existing config YAML with no template field (backwards compat)
        let yaml = "currency: USD\ninvoice_date_day: 5\npayment_terms_days: 14\n";

        // Act
        let defaults: Defaults = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert_eq!(defaults.template, TemplateKey::Leda);
    }

    #[test]
    fn test_defaults_with_template_field_deserializes() {
        // Arrange
        let yaml = "currency: EUR\ninvoice_date_day: 9\npayment_terms_days: 30\ntemplate: callisto\n";

        // Act
        let defaults: Defaults = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert_eq!(defaults.template, TemplateKey::Callisto);
    }

    #[test]
    fn test_defaults_template_round_trips() {
        // Arrange
        let defaults = Defaults {
            template: TemplateKey::Metis,
            ..Defaults::default()
        };

        // Act
        let yaml = serde_yaml::to_string(&defaults).unwrap();
        let loaded: Defaults = serde_yaml::from_str(&yaml).unwrap();

        // Assert
        assert_eq!(loaded.template, TemplateKey::Metis);
    }

    #[test]
    fn test_branding_optional_fields_omitted_when_none() {
        // Arrange
        let branding = Branding { logo: Some("logo.png".into()), ..Branding::default() };

        // Act
        let yaml = serde_yaml::to_string(&branding).unwrap();

        // Assert
        assert!(yaml.contains("logo"));
        assert!(!yaml.contains("accent_color"));
        assert!(!yaml.contains("font"));
        assert!(!yaml.contains("footer_text"));
    }
}
