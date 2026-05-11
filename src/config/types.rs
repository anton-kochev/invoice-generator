use serde::{Deserialize, Serialize};

use crate::domain::{
    Currency, HexColor, Iban, PaymentMethodKey, PresetKey, RecipientKey, SenderKey,
};
use crate::locale::Locale;

/// Default template name baked into [`Defaults`] when the user has not picked
/// one explicitly. Matches the previous `TemplateKey::Leda` default.
const DEFAULT_TEMPLATE_NAME: &str = "leda";

fn default_template() -> String {
    DEFAULT_TEMPLATE_NAME.to_string()
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
    pub default_recipient: Option<RecipientKey>,
    /// v2 multi-sender list.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub senders: Option<Vec<Sender>>,
    /// Key of the default sender profile.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_sender: Option<SenderKey>,
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
    /// Validated slug. Optional in raw config (v1 configs may lack it),
    /// auto-derived during validation when missing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<SenderKey>,
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub address: Vec<String>,
    pub email: String,
}

/// Information about the invoice recipient.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Recipient {
    /// Validated slug. Optional in raw config (v1 configs may lack it),
    /// auto-derived during validation when missing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<RecipientKey>,
    pub name: String,
    pub address: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub company_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "vat")]
    pub vat_number: Option<String>,
}

/// A payment method shown on the invoice.
///
/// Both `key` and `label` are `Option` at the raw-deserialize level so that v1
/// configs (label-only) and v2 configs (key + optional label) both parse
/// successfully. The validator enforces the "at least one of key/label" rule
/// and auto-derives `key` from `label` when missing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaymentMethod {
    /// Validated slug. Optional in raw config (v1 configs lack it),
    /// auto-derived from `label` during validation when missing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<PaymentMethodKey>,
    /// Display label rendered on the invoice. Optional — when absent, the
    /// payment block on the PDF shows only IBAN/BIC with no header.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Validated IBAN (mod-97 checksum verified at deserialize-time).
    pub iban: Iban,
    #[serde(alias = "bic")]
    pub bic_swift: String,
}

/// An invoice preset / template.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Preset {
    /// Validated slug.
    pub key: PresetKey,
    pub description: String,
    pub default_rate: f64,
    /// Per-preset currency override (validated to be USD/EUR/UAH at deserialize-time).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub currency: Option<Currency>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tax_rate: Option<f64>,
}

fn default_currency() -> Currency {
    Currency::Eur
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
    /// Default currency for line items (validated to USD/EUR/UAH at deserialize-time).
    #[serde(default = "default_currency")]
    pub currency: Currency,
    #[serde(default = "default_invoice_date_day")]
    pub invoice_date_day: u32,
    #[serde(default = "default_payment_terms_days")]
    pub payment_terms_days: u32,
    /// Slug of the template to use. Validated against the local templates
    /// directory by [`crate::config::validator`], not at deserialize time.
    #[serde(default = "default_template")]
    pub template: String,
    #[serde(
        default,
        deserialize_with = "crate::locale::deserialize_locale_lenient"
    )]
    pub locale: Locale,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            currency: default_currency(),
            invoice_date_day: default_invoice_date_day(),
            payment_terms_days: default_payment_terms_days(),
            template: default_template(),
            locale: Locale::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(r.key, Some(RecipientKey::try_new("acme").unwrap()));
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
        assert!(
            !yaml.contains("recipients"),
            "None recipients should be omitted from YAML"
        );
        assert!(
            !yaml.contains("default_recipient"),
            "None default_recipient should be omitted"
        );
    }

    #[test]
    fn v2_config_with_recipients_round_trips() {
        // Arrange
        let config = Config {
            recipients: Some(vec![Recipient {
                key: Some(RecipientKey::try_new("acme").unwrap()),
                name: "Acme Corp".into(),
                address: vec!["123 Main St".into()],
                company_id: None,
                vat_number: None,
            }]),
            default_recipient: Some(RecipientKey::try_new("acme").unwrap()),
            ..Config::default()
        };

        // Act
        let yaml = serde_yaml::to_string(&config).unwrap();
        let loaded: Config = serde_yaml::from_str(&yaml).unwrap();

        // Assert
        assert_eq!(loaded.recipients.as_ref().unwrap().len(), 1);
        assert_eq!(
            loaded.default_recipient,
            Some(RecipientKey::try_new("acme").unwrap())
        );
    }

    // ── Sender: v1/v2 key field ──

    #[test]
    fn v1_sender_yaml_deserializes_with_key_none() {
        // Arrange — legacy v1 sender YAML with no `key:` field.
        let yaml = "name: Alice\naddress:\n  - 42 Elm Street\nemail: alice@example.com\n";

        // Act
        let s: Sender = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert!(s.key.is_none());
        assert_eq!(s.name, "Alice");
        assert_eq!(s.address, vec!["42 Elm Street"]);
        assert_eq!(s.email, "alice@example.com");
    }

    #[test]
    fn v2_sender_yaml_with_key_deserializes() {
        // Arrange
        let yaml =
            "key: alice\nname: Alice\naddress:\n  - 42 Elm Street\nemail: alice@example.com\n";

        // Act
        let s: Sender = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert_eq!(s.key, Some(SenderKey::try_new("alice").unwrap()));
        assert_eq!(s.name, "Alice");
    }

    #[test]
    fn config_senders_none_omitted_from_yaml() {
        // Arrange
        let config = Config {
            senders: None,
            default_sender: None,
            ..Config::default()
        };

        // Act
        let yaml = serde_yaml::to_string(&config).unwrap();

        // Assert
        assert!(
            !yaml.contains("senders"),
            "None senders should be omitted from YAML"
        );
        assert!(
            !yaml.contains("default_sender"),
            "None default_sender should be omitted"
        );
    }

    #[test]
    fn v2_config_with_senders_round_trips() {
        // Arrange
        let config = Config {
            senders: Some(vec![Sender {
                key: Some(SenderKey::try_new("alice").unwrap()),
                name: "Alice".into(),
                address: vec!["42 Elm Street".into()],
                email: "alice@example.com".into(),
            }]),
            default_sender: Some(SenderKey::try_new("alice").unwrap()),
            ..Config::default()
        };

        // Act
        let yaml = serde_yaml::to_string(&config).unwrap();
        let loaded: Config = serde_yaml::from_str(&yaml).unwrap();

        // Assert
        assert_eq!(loaded.senders.as_ref().unwrap().len(), 1);
        assert_eq!(
            loaded.default_sender,
            Some(SenderKey::try_new("alice").unwrap())
        );
        let s = &loaded.senders.as_ref().unwrap()[0];
        assert_eq!(s.key, Some(SenderKey::try_new("alice").unwrap()));
        assert_eq!(s.name, "Alice");
        assert_eq!(s.address, vec!["42 Elm Street"]);
        assert_eq!(s.email, "alice@example.com");
    }

    #[test]
    fn sender_key_none_omitted_from_yaml() {
        // Arrange — keyless v1-style Sender literal.
        let sender = Sender {
            key: None,
            name: "Alice".into(),
            address: vec!["42 Elm Street".into()],
            email: "alice@example.com".into(),
        };

        // Act
        let yaml = serde_yaml::to_string(&sender).unwrap();
        let value: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
        let map = value.as_mapping().expect("sender serializes as map");

        // Assert
        assert!(
            !map.contains_key(serde_yaml::Value::String("key".into())),
            "Expected `key` field to be absent, got YAML: {yaml}"
        );
    }

    #[test]
    fn test_sender_empty_address_omits_field_in_yaml() {
        // Arrange — sender with no address lines; synthetic identity data.
        let sender = Sender {
            key: None,
            name: "Alice".into(),
            address: vec![],
            email: "a@b.com".into(),
        };

        // Act
        let yaml = serde_yaml::to_string(&sender).unwrap();

        // Assert
        assert!(
            !yaml.contains("address:"),
            "Empty address should be omitted from YAML, got: {yaml}"
        );
    }

    #[test]
    fn test_sender_deserializes_when_address_field_absent() {
        // Arrange — YAML omits the `address:` field entirely (this is the shape
        // `skip_serializing_if = "Vec::is_empty"` will produce in practice).
        let yaml = "name: Alice\nemail: a@b.com\n";

        // Act
        let result: Result<Sender, _> = serde_yaml::from_str(yaml);

        // Assert — currently fails to parse (no `#[serde(default)]` on `address`).
        // After step 4, this passes with an empty vec.
        let loaded = result.unwrap();
        assert!(loaded.address.is_empty());
        assert_eq!(loaded.name, "Alice");
    }

    #[test]
    fn test_preset_without_currency_deserializes_as_none() {
        // Arrange
        let yaml = "key: dev\ndescription: Development\ndefault_rate: 800.0\n";

        // Act
        let preset: Preset = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert!(preset.currency.is_none());
        assert_eq!(preset.key.as_str(), "dev");
    }

    #[test]
    fn test_preset_with_currency_deserializes() {
        // Arrange
        let yaml = "key: dev\ndescription: Development\ndefault_rate: 800.0\ncurrency: USD\n";

        // Act
        let preset: Preset = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert_eq!(preset.currency, Some(Currency::Usd));
    }

    #[test]
    fn test_preset_currency_none_omitted_from_yaml() {
        // Arrange
        let preset = Preset {
            key: PresetKey::try_new("dev").unwrap(),
            description: "Development".into(),
            default_rate: 800.0,
            currency: None,
            tax_rate: None,
        };

        // Act
        let yaml = serde_yaml::to_string(&preset).unwrap();

        // Assert
        assert!(
            !yaml.contains("currency"),
            "None currency should be omitted from YAML"
        );
    }

    #[test]
    fn test_preset_with_currency_round_trips() {
        // Arrange — UAH replaces the old CZK fixture: only USD/EUR/UAH are
        // accepted by the closed `Currency` enum.
        let preset = Preset {
            key: PresetKey::try_new("dev").unwrap(),
            description: "Development".into(),
            default_rate: 800.0,
            currency: Some(Currency::Uah),
            tax_rate: None,
        };

        // Act
        let yaml = serde_yaml::to_string(&preset).unwrap();
        let loaded: Preset = serde_yaml::from_str(&yaml).unwrap();

        // Assert
        assert_eq!(loaded.currency, Some(Currency::Uah));
    }

    #[test]
    fn test_preset_with_unsupported_currency_rejected_at_deserialize() {
        // Arrange — CZK was previously accepted as a free-form string but is
        // now rejected by the closed `Currency` enum at parse time.
        let yaml = "key: dev\ndescription: Development\ndefault_rate: 800.0\ncurrency: CZK\n";

        // Act
        let result: Result<Preset, _> = serde_yaml::from_str(yaml);

        // Assert
        assert!(result.is_err(), "Expected deserialize failure for CZK");
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
            key: PresetKey::try_new("dev").unwrap(),
            description: "Development".into(),
            default_rate: 800.0,
            currency: None,
            tax_rate: None,
        };

        // Act
        let yaml = serde_yaml::to_string(&preset).unwrap();

        // Assert
        assert!(
            !yaml.contains("tax_rate"),
            "None tax_rate should be omitted from YAML"
        );
    }

    #[test]
    fn test_preset_with_tax_rate_round_trips() {
        // Arrange
        let preset = Preset {
            key: PresetKey::try_new("dev").unwrap(),
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
            key: PresetKey::try_new("dev").unwrap(),
            description: "Development".into(),
            default_rate: 800.0,
            currency: None,
            tax_rate: Some(0.0),
        };

        // Act
        let yaml = serde_yaml::to_string(&preset).unwrap();

        // Assert
        assert!(
            yaml.contains("tax_rate: 0.0"),
            "Zero tax_rate should be serialized, got: {yaml}"
        );
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
        let config = Config {
            branding: None,
            ..Config::default()
        };

        // Act
        let yaml = serde_yaml::to_string(&config).unwrap();

        // Assert
        assert!(
            !yaml.contains("branding"),
            "None branding should be omitted from YAML"
        );
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
        let config = Config {
            branding: Some(branding),
            ..Config::default()
        };

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
        let config = Config {
            branding: Some(branding),
            ..Config::default()
        };

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
        assert!(
            result.is_err(),
            "Expected deserialize failure for #abc short form"
        );
    }

    #[test]
    fn test_branding_invalid_hex_rejected_at_deserialize() {
        // Arrange — non-hex chars
        let yaml = "branding:\n  accent_color: \"red\"\n";

        // Act
        let result: Result<Config, _> = serde_yaml::from_str(yaml);

        // Assert
        assert!(
            result.is_err(),
            "Expected deserialize failure for non-hex value"
        );
    }

    #[test]
    fn test_branding_empty_struct_round_trips() {
        // Arrange — all fields None
        let config = Config {
            branding: Some(Branding::default()),
            ..Config::default()
        };

        // Act
        let yaml = serde_yaml::to_string(&config).unwrap();
        let loaded: Config = serde_yaml::from_str(&yaml).unwrap();

        // Assert
        assert!(loaded.branding.is_some());
    }

    // ── Defaults.template (string slug) ──

    #[test]
    fn test_defaults_default_includes_template_leda() {
        // Arrange & Act
        let defaults = Defaults::default();

        // Assert
        assert_eq!(defaults.template, "leda");
    }

    #[test]
    fn test_defaults_without_template_field_deserializes_as_leda() {
        // Arrange — existing config YAML with no template field (backwards compat)
        let yaml = "currency: USD\ninvoice_date_day: 5\npayment_terms_days: 14\n";

        // Act
        let defaults: Defaults = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert_eq!(defaults.template, "leda");
    }

    #[test]
    fn test_defaults_with_template_field_deserializes() {
        // Arrange
        let yaml =
            "currency: EUR\ninvoice_date_day: 9\npayment_terms_days: 30\ntemplate: callisto\n";

        // Act
        let defaults: Defaults = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert_eq!(defaults.template, "callisto");
    }

    #[test]
    fn test_defaults_with_arbitrary_template_string_deserializes() {
        // Arrange — registry no longer constrains the template name to a closed
        // enum: any non-empty slug round-trips as-is. Validation against the
        // local templates dir happens later in `crate::config::validator`.
        let yaml = "template: my-custom-template\n";

        // Act
        let defaults: Defaults = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert_eq!(defaults.template, "my-custom-template");
    }

    #[test]
    fn test_defaults_template_round_trips() {
        // Arrange
        let defaults = Defaults {
            template: "metis".into(),
            ..Defaults::default()
        };

        // Act
        let yaml = serde_yaml::to_string(&defaults).unwrap();
        let loaded: Defaults = serde_yaml::from_str(&yaml).unwrap();

        // Assert
        assert_eq!(loaded.template, "metis");
    }

    #[test]
    fn test_branding_optional_fields_omitted_when_none() {
        // Arrange
        let branding = Branding {
            logo: Some("logo.png".into()),
            ..Branding::default()
        };

        // Act
        let yaml = serde_yaml::to_string(&branding).unwrap();

        // Assert
        assert!(yaml.contains("logo"));
        assert!(!yaml.contains("accent_color"));
        assert!(!yaml.contains("font"));
        assert!(!yaml.contains("footer_text"));
    }

    // ── PaymentMethod: key/label split ──

    #[test]
    fn v1_payment_yaml_with_label_only_deserializes_with_key_none() {
        // Arrange — legacy v1 config: label only, no key.
        let yaml = "label: SEPA Transfer\niban: DE89370400440532013000\nbic_swift: COBADEFFXXX\n";

        // Act
        let p: PaymentMethod = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert!(p.key.is_none());
        assert_eq!(p.label.as_deref(), Some("SEPA Transfer"));
    }

    #[test]
    fn v2_payment_yaml_with_key_and_label_deserializes() {
        // Arrange
        let yaml = "key: mono-eur-sepa\nlabel: SEPA Transfer\niban: DE89370400440532013000\nbic_swift: COBADEFFXXX\n";

        // Act
        let p: PaymentMethod = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert_eq!(
            p.key,
            Some(PaymentMethodKey::try_new("mono-eur-sepa").unwrap())
        );
        assert_eq!(p.label.as_deref(), Some("SEPA Transfer"));
    }

    #[test]
    fn v2_payment_yaml_with_key_only_deserializes() {
        // Arrange — anton's migrated config shape: key only, no label.
        let yaml = "key: mono-eur-sepa\niban: DE89370400440532013000\nbic_swift: COBADEFFXXX\n";

        // Act
        let p: PaymentMethod = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert_eq!(
            p.key,
            Some(PaymentMethodKey::try_new("mono-eur-sepa").unwrap())
        );
        assert!(p.label.is_none());
    }

    #[test]
    fn payment_yaml_with_neither_key_nor_label_deserializes_at_type_level() {
        // Arrange — both key and label absent. The type-level deserialization
        // accepts this; the validator is responsible for rejecting it.
        let yaml = "iban: DE89370400440532013000\nbic_swift: COBADEFFXXX\n";

        // Act
        let p: PaymentMethod = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert!(p.key.is_none());
        assert!(p.label.is_none());
    }

    #[test]
    fn payment_method_label_none_omitted_from_yaml() {
        // Arrange
        let p = PaymentMethod {
            key: Some(PaymentMethodKey::try_new("mono-eur-sepa").unwrap()),
            label: None,
            iban: Iban::try_new("DE89370400440532013000").unwrap(),
            bic_swift: "COBADEFFXXX".into(),
        };

        // Act
        let yaml = serde_yaml::to_string(&p).unwrap();
        let value: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
        let map = value
            .as_mapping()
            .expect("payment method serializes as map");

        // Assert
        assert!(
            !map.contains_key(serde_yaml::Value::String("label".into())),
            "Expected `label` key to be absent, got YAML: {yaml}"
        );
    }

    #[test]
    fn payment_method_key_none_omitted_from_yaml() {
        // Arrange — legacy shape: label-only, no key.
        let p = PaymentMethod {
            key: None,
            label: Some("SEPA Transfer".into()),
            iban: Iban::try_new("DE89370400440532013000").unwrap(),
            bic_swift: "COBADEFFXXX".into(),
        };

        // Act
        let yaml = serde_yaml::to_string(&p).unwrap();
        let value: serde_yaml::Value = serde_yaml::from_str(&yaml).unwrap();
        let map = value
            .as_mapping()
            .expect("payment method serializes as map");

        // Assert
        assert!(
            !map.contains_key(serde_yaml::Value::String("key".into())),
            "Expected `key` key to be absent, got YAML: {yaml}"
        );
    }

    #[test]
    fn payment_method_bic_alias_still_works() {
        // Arrange — historical regression guard: `bic:` is accepted as alias
        // for `bic_swift:`.
        let yaml = "label: SEPA\niban: DE89370400440532013000\nbic: COBADEFFXXX\n";

        // Act
        let p: PaymentMethod = serde_yaml::from_str(yaml).unwrap();

        // Assert
        assert_eq!(p.bic_swift, "COBADEFFXXX");
    }

    #[test]
    fn payment_method_round_trip_with_key_and_label() {
        // Arrange
        let p = PaymentMethod {
            key: Some(PaymentMethodKey::try_new("mono-eur-sepa").unwrap()),
            label: Some("SEPA Transfer".into()),
            iban: Iban::try_new("DE89370400440532013000").unwrap(),
            bic_swift: "COBADEFFXXX".into(),
        };

        // Act
        let yaml = serde_yaml::to_string(&p).unwrap();
        let loaded: PaymentMethod = serde_yaml::from_str(&yaml).unwrap();

        // Assert
        assert_eq!(loaded, p);
    }
}
