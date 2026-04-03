use std::path::Path;

use crate::error::AppError;

use super::types::Config;

/// The config file name looked up in a directory.
pub const CONFIG_FILENAME: &str = "invoice_config.yaml";

/// Result of attempting to load a config file.
#[derive(Debug)]
pub enum LoadResult {
    /// Config file was found and parsed.
    Loaded(Config),
    /// No config file exists in the directory.
    NotFound,
}

/// Attempt to load and parse the config file from `dir`.
///
/// Returns `Ok(LoadResult::NotFound)` if the file does not exist,
/// `Ok(LoadResult::Loaded(config))` on success, or an error on IO/parse failure.
pub fn load_config(dir: &Path) -> Result<LoadResult, AppError> {
    let path = dir.join(CONFIG_FILENAME);
    if !path.exists() {
        return Ok(LoadResult::NotFound);
    }
    let contents = std::fs::read_to_string(&path)?;
    if contents.trim().is_empty() {
        return Ok(LoadResult::Loaded(Config::default()));
    }
    let config: Config = serde_yaml::from_str(&contents)?;
    Ok(LoadResult::Loaded(config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    // ── Cycle 1 ──

    #[test]
    fn test_load_config_file_not_found_returns_not_found() {
        let dir = TempDir::new().unwrap();
        let result = load_config(dir.path());
        assert!(result.is_ok());
        assert!(matches!(result.unwrap(), LoadResult::NotFound));
    }

    // ── Cycle 2 ──

    #[test]
    fn test_load_config_valid_complete_returns_loaded() {
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
        std::fs::write(dir.path().join("invoice_config.yaml"), yaml).unwrap();
        let result = load_config(dir.path()).unwrap();
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
                assert_eq!(presets[0].key, "dev");
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
        let result = load_from_yaml("").unwrap();
        match result {
            LoadResult::Loaded(config) => {
                assert_eq!(config, Config::default());
            }
            LoadResult::NotFound => panic!("Expected Loaded"),
        }
    }

    // ── Cycle 5: malformed YAML → ConfigParse ──

    #[test]
    fn test_load_config_malformed_yaml_returns_config_parse_error() {
        let result = load_from_yaml("sender:\n  name: [invalid yaml\n  broken: {{}");
        assert!(matches!(result, Err(AppError::ConfigParse(_))));
    }

    // ── Cycle 6: IO error (unix only) ──

    #[test]
    #[cfg(unix)]
    fn test_load_config_io_error_returns_config_io_error() {
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("invoice_config.yaml");
        std::fs::write(&path, "sender:\n  name: test").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o000)).unwrap();
        let result = load_config(dir.path());
        assert!(matches!(result, Err(AppError::ConfigIo(_))));
    }

    // ── Cycle 7: bic alias ──

    #[test]
    fn test_load_config_bic_alias() {
        let result = load_from_yaml(
            r#"
payment:
  - label: "Transfer"
    iban: "DE00000000000000000000"
    bic: "COBADEFFXXX"
"#,
        )
        .unwrap();
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
        let result = load_from_yaml("defaults: {}").unwrap();
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
        match result {
            LoadResult::Loaded(config) => {
                assert!(config.sender.is_some());
                assert_eq!(config.sender.unwrap().name, "Alice");
            }
            LoadResult::NotFound => panic!("Expected Loaded"),
        }
    }

    // ── Helper ──

    fn load_from_yaml(yaml: &str) -> Result<LoadResult, AppError> {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("invoice_config.yaml"), yaml).unwrap();
        load_config(dir.path())
    }
}
