use std::path::Path;

use crate::error::AppError;

use super::types::Config;

/// The default config file name used when constructing the default XDG path.
///
/// Loader/writer functions take a full file path, so this constant is no
/// longer joined inside them — it's only used by `config::path` to build the
/// default location (`~/.config/invoice-generator/config.yaml`).
pub const CONFIG_FILENAME: &str = "config.yaml";

/// Result of attempting to load a config file.
#[derive(Debug)]
pub enum LoadResult {
    /// Config file was found and parsed.
    Loaded(Box<Config>),
    /// No config file exists in the directory.
    NotFound,
}

/// Check whether a YAML field key appears as an actual key at the start of a
/// line (after optional leading whitespace), not inside a comment or string value.
fn yaml_has_field(content: &str, field: &str) -> bool {
    content.lines().any(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with(field) && trimmed[field.len()..].starts_with(':')
    })
}

/// Returns hints about optional fields missing from the raw YAML content.
///
/// Uses a line-based check so that field names inside comments or string
/// values are not mistaken for actual keys.
pub fn missing_field_hints(yaml_content: &str) -> Vec<&'static str> {
    let mut hints = Vec::new();
    if !yaml_has_field(yaml_content, "template") {
        hints.push(
            "  template: leda        \u{2014} invoice template style (leda, callisto, thebe, amalthea, metis)",
        );
    }
    if !yaml_has_field(yaml_content, "locale") {
        hints.push(
            "  locale: en-US         \u{2014} date/number formatting (en-US, en-GB, de-DE, fr-FR, cs-CZ, uk-UA)",
        );
    }
    hints
}

/// Attempt to load and parse the config file at `path`.
///
/// Returns `Ok(LoadResult::NotFound)` if the file does not exist,
/// `Ok(LoadResult::Loaded(config))` on success, or an error on IO/parse failure.
///
/// This function does **not** print hints about missing optional fields.
/// Callers that want to display hints (e.g. the interactive flow) should read
/// the raw YAML and call [`missing_field_hints`] themselves.
pub fn load_config(path: &Path) -> Result<LoadResult, AppError> {
    if !path.exists() {
        return Ok(LoadResult::NotFound);
    }
    let contents = std::fs::read_to_string(path)?;
    if contents.trim().is_empty() {
        return Ok(LoadResult::Loaded(Box::default()));
    }
    let config: Config = serde_yaml::from_str(&contents)?;
    Ok(LoadResult::Loaded(Box::new(config)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ── Cycle 1 ──

    #[test]
    fn test_load_config_file_not_found_returns_not_found() {
        // Arrange
        let dir = TempDir::new().unwrap();

        // Act
        let result = load_config(&dir.path().join("config.yaml"));

        // Assert
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), LoadResult::NotFound));
    }

    // ── Cycle 2 ──

    #[test]
    fn test_load_config_valid_complete_returns_loaded() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let yaml = r#"
sender:
  name: "Alice"
  address:
    - "123 Main St"
  email: "alice@example.com"
recipient:
  name: "Bob Corp"
  address:
    - "456 Oak Ave"
  company_id: "12345"
  vat_number: "CZ12345"
payment:
  - label: "SEPA Transfer"
    iban: "DE89370400440532013000"
    bic_swift: "COBADEFFXXX"
presets:
  - key: "dev"
    description: "Development Services"
    default_rate: 100.0
defaults:
  currency: "USD"
  payment_terms_days: 14
  invoice_date_day: 5
"#;
        let path = dir.path().join("config.yaml");
        std::fs::write(&path, yaml).unwrap();

        // Act
        let result = load_config(&path).unwrap();

        // Assert
        match result {
            LoadResult::Loaded(config) => {
                let sender = config.sender.unwrap();
                assert_eq!(sender.name, "Alice");
                assert_eq!(sender.address, vec!["123 Main St"]);
                assert_eq!(sender.email, "alice@example.com");

                let recipient = config.recipient.unwrap();
                assert_eq!(recipient.name, "Bob Corp");
                assert_eq!(recipient.company_id, Some("12345".to_string()));
                assert_eq!(recipient.vat_number, Some("CZ12345".to_string()));

                let payment = config.payment.unwrap();
                assert_eq!(payment.len(), 1);
                assert_eq!(payment[0].label, "SEPA Transfer");
                assert_eq!(payment[0].bic_swift, "COBADEFFXXX");

                let presets = config.presets.unwrap();
                assert_eq!(presets[0].key.as_str(), "dev");
                assert_eq!(presets[0].default_rate, 100.0);

                let defaults = config.defaults.unwrap();
                assert_eq!(defaults.currency, "USD");
                assert_eq!(defaults.payment_terms_days, 14);
                assert_eq!(defaults.invoice_date_day, 5);
            }
            LoadResult::NotFound => panic!("Expected Loaded, got NotFound"),
        }
    }

    // ── Cycle 3: partial config (sender only) ──

    #[test]
    fn test_load_config_partial_sender_only() {
        // Act
        let result = load_from_yaml(
            r#"
sender:
  name: "Alice"
  address:
    - "123 Main St"
  email: "alice@example.com"
"#,
        )
        .unwrap();

        // Assert
        match result {
            LoadResult::Loaded(config) => {
                assert!(config.sender.is_some());
                assert_eq!(config.sender.unwrap().name, "Alice");
                assert!(config.recipient.is_none());
                assert!(config.payment.is_none());
                assert!(config.presets.is_none());
                assert!(config.defaults.is_none());
            }
            LoadResult::NotFound => panic!("Expected Loaded"),
        }
    }

    // ── Cycle 4: empty file → Loaded with all None ──

    #[test]
    fn test_load_config_empty_file_returns_loaded_default() {
        // Act
        let result = load_from_yaml("").unwrap();

        // Assert
        match result {
            LoadResult::Loaded(config) => {
                assert_eq!(*config, Config::default());
            }
            LoadResult::NotFound => panic!("Expected Loaded"),
        }
    }

    // ── Cycle 5: malformed YAML → ConfigParse ──

    #[test]
    fn test_load_config_malformed_yaml_returns_config_parse_error() {
        // Act
        let result = load_from_yaml("sender:\n  name: [invalid yaml\n  broken: {{}");

        // Assert
        assert!(matches!(result, Err(AppError::ConfigParse(_))));
    }

    // ── Cycle 6: IO error (unix only) ──

    #[test]
    #[cfg(unix)]
    fn test_load_config_io_error_returns_config_io_error() {
        // Arrange
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.yaml");
        std::fs::write(&path, "sender:\n  name: test").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();

        // Act
        let result = load_config(&path);

        // Assert
        assert!(matches!(result, Err(AppError::ConfigIo(_))));
    }

    // ── Cycle 7: bic alias ──

    #[test]
    fn test_load_config_bic_alias() {
        // Act
        let result = load_from_yaml(
            r#"
payment:
  - label: "Transfer"
    iban: "DE89370400440532013000"
    bic: "COBADEFFXXX"
"#,
        )
        .unwrap();

        // Assert
        match result {
            LoadResult::Loaded(config) => {
                let payment = config.payment.unwrap();
                assert_eq!(payment[0].bic_swift, "COBADEFFXXX");
            }
            LoadResult::NotFound => panic!("Expected Loaded"),
        }
    }

    // ── Cycle 8: vat alias ──

    #[test]
    fn test_load_config_vat_alias() {
        // Act
        let result = load_from_yaml(
            r#"
recipient:
  name: "Corp"
  address:
    - "Street 1"
  vat: "CZ99999"
"#,
        )
        .unwrap();

        // Assert
        match result {
            LoadResult::Loaded(config) => {
                let recipient = config.recipient.unwrap();
                assert_eq!(recipient.vat_number, Some("CZ99999".to_string()));
            }
            LoadResult::NotFound => panic!("Expected Loaded"),
        }
    }

    // ── Cycle 9: defaults with empty object ──

    #[test]
    fn test_load_config_defaults_empty_object_uses_serde_defaults() {
        // Act
        let result = load_from_yaml("defaults: {}").unwrap();

        // Assert
        match result {
            LoadResult::Loaded(config) => {
                let defaults = config.defaults.unwrap();
                assert_eq!(defaults.currency, "EUR");
                assert_eq!(defaults.invoice_date_day, 9);
                assert_eq!(defaults.payment_terms_days, 30);
            }
            LoadResult::NotFound => panic!("Expected Loaded"),
        }
    }

    // ── Cycle 10: unknown fields are ignored ──

    #[test]
    fn test_load_config_unknown_fields_are_ignored() {
        // Act
        let result = load_from_yaml(
            r#"
unknown_section: "hello"
sender:
  name: "Alice"
  address:
    - "Street"
  email: "a@b.com"
  phone: "+1234567890"
"#,
        )
        .unwrap();

        // Assert
        match result {
            LoadResult::Loaded(config) => {
                assert!(config.sender.is_some());
                assert_eq!(config.sender.unwrap().name, "Alice");
            }
            LoadResult::NotFound => panic!("Expected Loaded"),
        }
    }

    // ── Story 14.1: backwards compatibility + field hints ──

    #[test]
    fn test_v2_config_without_template_or_locale_loads_successfully() {
        // Arrange — complete v2 config with no template/locale
        let yaml = r#"
sender:
  name: "Synthetic Sender"
  address:
    - "10 Fake Lane"
  email: "sender@example.com"
recipient:
  name: "Synthetic Corp"
  address:
    - "20 Mock Blvd"
  company_id: "00000"
  vat_number: "CZ00000"
payment:
  - label: "Wire"
    iban: "DE89370400440532013000"
    bic_swift: "TESTDEFFXXX"
presets:
  - key: "dev"
    description: "Development"
    default_rate: 100.0
defaults:
  currency: "USD"
  payment_terms_days: 14
  invoice_date_day: 5
"#;

        // Act
        let result = load_from_yaml(yaml);

        // Assert
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), LoadResult::Loaded(_)));
    }

    #[test]
    fn test_v2_config_defaults_to_leda_template() {
        // Arrange — defaults section without template field
        let yaml = r#"
defaults:
  currency: "USD"
  payment_terms_days: 14
  invoice_date_day: 5
"#;

        // Act
        let result = load_from_yaml(yaml).unwrap();

        // Assert
        match result {
            LoadResult::Loaded(config) => {
                let defaults = config.defaults.unwrap();
                assert_eq!(defaults.template, crate::config::types::TemplateKey::Leda);
            }
            LoadResult::NotFound => panic!("Expected Loaded"),
        }
    }

    #[test]
    fn test_v2_config_defaults_to_en_us_locale() {
        // Arrange — defaults section without locale field
        let yaml = r#"
defaults:
  currency: "EUR"
  payment_terms_days: 30
  invoice_date_day: 9
"#;

        // Act
        let result = load_from_yaml(yaml).unwrap();

        // Assert
        match result {
            LoadResult::Loaded(config) => {
                let defaults = config.defaults.unwrap();
                assert_eq!(defaults.locale, crate::locale::Locale::EnUs);
            }
            LoadResult::NotFound => panic!("Expected Loaded"),
        }
    }

    #[test]
    fn test_config_with_template_and_locale_loads() {
        // Arrange — config with both new fields present
        let yaml = r#"
defaults:
  currency: "EUR"
  payment_terms_days: 30
  invoice_date_day: 9
  template: callisto
  locale: de-DE
"#;

        // Act
        let result = load_from_yaml(yaml).unwrap();

        // Assert
        match result {
            LoadResult::Loaded(config) => {
                let defaults = config.defaults.unwrap();
                assert_eq!(defaults.template, crate::config::types::TemplateKey::Callisto);
                assert_eq!(defaults.locale, crate::locale::Locale::DeDe);
            }
            LoadResult::NotFound => panic!("Expected Loaded"),
        }
    }

    #[test]
    fn test_missing_field_hints_missing_both() {
        // Arrange
        let yaml = "defaults:\n  currency: EUR\n";

        // Act
        let hints = missing_field_hints(yaml);

        // Assert
        assert_eq!(hints.len(), 2);
        assert!(hints[0].contains("template"));
        assert!(hints[1].contains("locale"));
    }

    #[test]
    fn test_missing_field_hints_missing_locale_only() {
        // Arrange
        let yaml = "defaults:\n  currency: EUR\n  template: leda\n";

        // Act
        let hints = missing_field_hints(yaml);

        // Assert
        assert_eq!(hints.len(), 1);
        assert!(hints[0].contains("locale"));
    }

    #[test]
    fn test_missing_field_hints_nothing_missing() {
        // Arrange
        let yaml = "defaults:\n  currency: EUR\n  template: leda\n  locale: en-US\n";

        // Act
        let hints = missing_field_hints(yaml);

        // Assert
        assert!(hints.is_empty());
    }

    // ── Fix 2: robust field detection ──

    #[test]
    fn test_missing_field_hints_ignores_comment_with_template() {
        // Arrange — template appears only in a comment, not as an actual key
        let yaml = "# template: leda\ndefaults:\n  currency: EUR\n  locale: en-US\n";

        // Act
        let hints = missing_field_hints(yaml);

        // Assert — template hint should still be returned
        assert_eq!(hints.len(), 1);
        assert!(hints[0].contains("template"));
    }

    #[test]
    fn test_missing_field_hints_ignores_string_value_containing_template() {
        // Arrange — "template:" appears inside a string value, not as a key
        let yaml =
            "defaults:\n  currency: EUR\n  locale: en-US\n  footer_text: \"Use template: leda\"\n";

        // Act
        let hints = missing_field_hints(yaml);

        // Assert — template hint should still be returned
        assert_eq!(hints.len(), 1);
        assert!(hints[0].contains("template"));
    }

    // ── Helper ──

    fn load_from_yaml(yaml: &str) -> Result<LoadResult, AppError> {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("config.yaml");
        std::fs::write(&path, yaml).unwrap();
        load_config(&path)
    }
}
