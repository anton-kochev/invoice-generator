use std::path::Path;

use crate::error::AppError;

use super::loader::CONFIG_FILENAME;
use super::types::{Config, Preset, Recipient};

/// Serialize `config` to YAML and write it to `dir/CONFIG_FILENAME`.
pub fn save_config(dir: &Path, config: &Config) -> Result<(), AppError> {
    let yaml = serde_yaml::to_string(config)?;
    let path = dir.join(CONFIG_FILENAME);
    std::fs::write(path, yaml)?;
    Ok(())
}

/// Remove a preset by key from the config file in `dir`.
///
/// Returns the removed preset on success.
/// Checks the last-preset guard BEFORE key lookup.
pub fn remove_preset(dir: &Path, key: &str) -> Result<Preset, AppError> {
    use super::loader::{load_config, LoadResult};

    let config = match load_config(dir)? {
        LoadResult::Loaded(config) => config,
        LoadResult::NotFound => {
            return Err(AppError::ConfigIo(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("config file not found in {}", dir.display()),
            )));
        }
    };

    let mut config = config;
    let mut presets = config.presets.unwrap_or_default();

    if presets.len() <= 1 {
        return Err(AppError::LastPreset);
    }

    let pos = presets
        .iter()
        .position(|p| p.key == key)
        .ok_or_else(|| AppError::PresetNotFound(key.to_string()))?;

    let removed = presets.remove(pos);
    config.presets = Some(presets);

    save_config(dir, &config)?;
    Ok(removed)
}

/// Append a preset to the config file in `dir`.
///
/// Loads the existing config, pushes the new preset, and saves it back.
/// Returns an error if no config file exists yet.
pub fn append_preset(dir: &Path, preset: Preset) -> Result<(), AppError> {
    use super::loader::{load_config, LoadResult};

    let config = match load_config(dir)? {
        LoadResult::Loaded(config) => config,
        LoadResult::NotFound => {
            return Err(AppError::ConfigIo(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("config file not found in {}", dir.display()),
            )));
        }
    };

    let mut config = config;
    let mut presets = config.presets.unwrap_or_default();
    presets.push(preset);
    config.presets = Some(presets);

    save_config(dir, &config)
}

/// Append a recipient to the config file in `dir`.
///
/// If `set_default` is true, also sets `default_recipient` to the new recipient's key.
pub fn append_recipient(dir: &Path, recipient: Recipient, set_default: bool) -> Result<(), AppError> {
    use super::loader::{load_config, LoadResult};

    let config = match load_config(dir)? {
        LoadResult::Loaded(config) => config,
        LoadResult::NotFound => {
            return Err(AppError::ConfigIo(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("config file not found in {}", dir.display()),
            )));
        }
    };

    let mut config = config;
    let mut recipients = config.recipients.unwrap_or_default();

    if set_default {
        config.default_recipient = recipient.key.clone();
    }

    recipients.push(recipient);
    config.recipients = Some(recipients);

    save_config(dir, &config)
}

/// Remove a recipient by key from the config file in `dir`.
///
/// Returns the removed recipient on success.
/// If the removed recipient was the default, clears `default_recipient` (caller handles reassignment).
pub fn remove_recipient(dir: &Path, key: &str) -> Result<Recipient, AppError> {
    use super::loader::{load_config, LoadResult};

    let config = match load_config(dir)? {
        LoadResult::Loaded(config) => config,
        LoadResult::NotFound => {
            return Err(AppError::ConfigIo(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("config file not found in {}", dir.display()),
            )));
        }
    };

    let mut config = config;
    let mut recipients = config.recipients.unwrap_or_default();

    if recipients.len() <= 1 {
        return Err(AppError::LastRecipient);
    }

    let pos = recipients
        .iter()
        .position(|r| r.key.as_deref() == Some(key))
        .ok_or_else(|| AppError::RecipientNotFound {
            key: key.to_string(),
            available: recipients.iter().filter_map(|r| r.key.clone()).collect(),
        })?;

    let removed = recipients.remove(pos);
    config.recipients = Some(recipients);

    // Clear default if it was the removed recipient
    if config.default_recipient.as_deref() == Some(key) {
        config.default_recipient = None;
    }

    save_config(dir, &config)?;
    Ok(removed)
}

/// Set the default recipient key in the config file.
///
/// Verifies the key exists in the recipients list before updating.
pub fn set_default_recipient(dir: &Path, key: &str) -> Result<(), AppError> {
    use super::loader::{load_config, LoadResult};

    let config = match load_config(dir)? {
        LoadResult::Loaded(config) => config,
        LoadResult::NotFound => {
            return Err(AppError::ConfigIo(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("config file not found in {}", dir.display()),
            )));
        }
    };

    let mut config = config;
    let recipients = config.recipients.as_deref().unwrap_or_default();

    if !recipients.iter().any(|r| r.key.as_deref() == Some(key)) {
        return Err(AppError::RecipientNotFound {
            key: key.to_string(),
            available: recipients.iter().filter_map(|r| r.key.clone()).collect(),
        });
    }

    config.default_recipient = Some(key.to_string());
    save_config(dir, &config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::loader::{load_config, LoadResult};
    use crate::config::types::*;
    use tempfile::TempDir;

    // ── Helpers ──

    fn synthetic_sender() -> Sender {
        Sender {
            name: "Alice Smith".to_string(),
            address: vec!["42 Elm Street".to_string(), "Springfield, IL 62704".to_string()],
            email: "alice@example.com".to_string(),
        }
    }

    fn synthetic_recipient() -> Recipient {
        Recipient {
            key: Some("bob-corp".to_string()),
            name: "Bob Corp".to_string(),
            address: vec!["99 Oak Lane".to_string(), "Shelbyville, IL 62565".to_string()],
            company_id: Some("BC-98765".to_string()),
            vat_number: Some("CZ12345678".to_string()),
        }
    }

    fn synthetic_payment() -> Vec<PaymentMethod> {
        vec![PaymentMethod {
            label: "SEPA Transfer".to_string(),
            iban: "DE89370400440532013000".to_string(),
            bic_swift: "COBADEFFXXX".to_string(),
        }]
    }

    fn synthetic_presets() -> Vec<Preset> {
        vec![Preset {
            key: "dev".to_string(),
            description: "Development Services".to_string(),
            default_rate: 100.0,
        }]
    }

    fn synthetic_defaults() -> Defaults {
        Defaults {
            currency: "USD".to_string(),
            invoice_date_day: 5,
            payment_terms_days: 14,
        }
    }

    fn complete_config() -> Config {
        Config {
            sender: Some(synthetic_sender()),
            recipient: None,
            recipients: Some(vec![synthetic_recipient()]),
            default_recipient: Some("bob-corp".to_string()),
            payment: Some(synthetic_payment()),
            presets: Some(synthetic_presets()),
            defaults: Some(synthetic_defaults()),
        }
    }

    fn unwrap_loaded(result: Result<LoadResult, AppError>) -> Config {
        match result.unwrap() {
            LoadResult::Loaded(c) => c,
            LoadResult::NotFound => panic!("Expected Loaded, got NotFound"),
        }
    }

    // ── Cycle 1: test_save_config_creates_file ──

    #[test]
    fn test_save_config_creates_file() {
        // Arrange
        let dir = TempDir::new().unwrap();

        // Act
        let result = save_config(dir.path(), &Config::default());

        // Assert
        assert!(result.is_ok());
        assert!(dir.path().join(CONFIG_FILENAME).exists());
    }

    // ── Cycle 2: test_save_config_complete_round_trips ──

    #[test]
    fn test_save_config_complete_round_trips() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let original = complete_config();

        // Act
        save_config(dir.path(), &original).unwrap();
        let loaded = unwrap_loaded(load_config(dir.path()));

        // Assert
        assert_eq!(loaded, original);
    }

    // ── Cycle 3: test_save_config_partial_sender_only ──

    #[test]
    fn test_save_config_partial_sender_only() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let config = Config {
            sender: Some(synthetic_sender()),
            ..Config::default()
        };

        // Act
        save_config(dir.path(), &config).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(dir.path()));
        assert_eq!(loaded.sender, Some(synthetic_sender()));
        assert!(loaded.recipient.is_none());
        assert!(loaded.payment.is_none());
        assert!(loaded.presets.is_none());
        assert!(loaded.defaults.is_none());

        let raw = std::fs::read_to_string(dir.path().join(CONFIG_FILENAME)).unwrap();
        assert!(!raw.contains("null"), "YAML output should not contain 'null'");
    }

    // ── Cycle 4: test_save_config_overwrites_existing ──

    #[test]
    fn test_save_config_overwrites_existing() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let alice = Config {
            sender: Some(Sender {
                name: "Alice".to_string(),
                address: vec!["Street 1".to_string()],
                email: "alice@example.com".to_string(),
            }),
            ..Config::default()
        };
        save_config(dir.path(), &alice).unwrap();

        let bob = Config {
            sender: Some(Sender {
                name: "Bob".to_string(),
                address: vec!["Street 2".to_string()],
                email: "bob@example.com".to_string(),
            }),
            ..Config::default()
        };

        // Act
        save_config(dir.path(), &bob).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(dir.path()));
        assert_eq!(loaded.sender.unwrap().name, "Bob");
    }

    // ── Cycle 5: test_save_config_produces_valid_yaml ──

    #[test]
    fn test_save_config_produces_valid_yaml() {
        // Arrange
        let dir = TempDir::new().unwrap();

        // Act
        save_config(dir.path(), &complete_config()).unwrap();
        let raw = std::fs::read_to_string(dir.path().join(CONFIG_FILENAME)).unwrap();

        // Assert
        let parsed: Result<Config, _> = serde_yaml::from_str(&raw);
        assert!(parsed.is_ok());
    }

    // ── Cycle 6: test_save_config_io_error ──

    #[test]
    #[cfg(unix)]
    fn test_save_config_io_error() {
        // Arrange
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o444)).unwrap();

        // Act
        let result = save_config(dir.path(), &Config::default());

        // Assert
        assert!(matches!(result, Err(AppError::ConfigIo(_))));

        // Restore permissions so TempDir cleanup works.
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    // ── Cycle 7: test_append_preset_to_existing_presets ──

    #[test]
    fn test_append_preset_to_existing_presets() {
        // Arrange
        let dir = TempDir::new().unwrap();
        save_config(dir.path(), &complete_config()).unwrap();
        let new_preset = Preset {
            key: "design".to_string(),
            description: "Design work".to_string(),
            default_rate: 80.0,
        };

        // Act
        append_preset(dir.path(), new_preset).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(dir.path()));
        let presets = loaded.presets.unwrap();
        assert_eq!(presets.len(), 2);
        assert_eq!(presets[0].key, "dev");
        assert_eq!(presets[1].key, "design");
    }

    // ── Cycle 8: test_append_preset_preserves_other_sections ──

    #[test]
    fn test_append_preset_preserves_other_sections() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let original = complete_config();
        save_config(dir.path(), &original).unwrap();

        // Act
        append_preset(
            dir.path(),
            Preset {
                key: "qa".to_string(),
                description: "QA work".to_string(),
                default_rate: 60.0,
            },
        )
        .unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(dir.path()));
        assert_eq!(loaded.sender, original.sender);
        assert_eq!(loaded.recipient, original.recipient);
        assert_eq!(loaded.payment, original.payment);
        assert_eq!(loaded.defaults, original.defaults);
    }

    // ── Cycle 9: test_append_preset_when_no_presets_field ──

    #[test]
    fn test_append_preset_when_no_presets_field() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let config = Config {
            sender: Some(synthetic_sender()),
            ..Config::default()
        };
        save_config(dir.path(), &config).unwrap();

        // Act
        append_preset(
            dir.path(),
            Preset {
                key: "ops".to_string(),
                description: "Operations".to_string(),
                default_rate: 90.0,
            },
        )
        .unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(dir.path()));
        let presets = loaded.presets.unwrap();
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].key, "ops");
    }

    // ── Cycle 10: test_append_preset_no_config_file ──

    #[test]
    fn test_append_preset_no_config_file() {
        // Arrange
        let dir = TempDir::new().unwrap();

        // Act
        let result = append_preset(
            dir.path(),
            Preset {
                key: "x".to_string(),
                description: "X".to_string(),
                default_rate: 50.0,
            },
        );

        // Assert
        assert!(result.is_err());
        assert!(matches!(result, Err(AppError::ConfigIo(_))));
    }

    // ── remove_preset tests ──

    #[test]
    fn test_remove_preset_deletes_matching_key() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let mut config = complete_config();
        let mut presets = config.presets.take().unwrap();
        presets.push(Preset {
            key: "design".to_string(),
            description: "Design work".to_string(),
            default_rate: 80.0,
        });
        config.presets = Some(presets);
        save_config(dir.path(), &config).unwrap();

        // Act
        let removed = remove_preset(dir.path(), "design").unwrap();

        // Assert
        assert_eq!(removed.key, "design");
        let loaded = unwrap_loaded(load_config(dir.path()));
        let remaining = loaded.presets.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].key, "dev");
    }

    #[test]
    fn test_remove_preset_unknown_key_returns_preset_not_found() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let mut config = complete_config();
        let mut presets = config.presets.take().unwrap();
        presets.push(Preset {
            key: "design".to_string(),
            description: "Design work".to_string(),
            default_rate: 80.0,
        });
        config.presets = Some(presets);
        save_config(dir.path(), &config).unwrap();

        // Act
        let result = remove_preset(dir.path(), "nope");

        // Assert
        assert!(matches!(result, Err(AppError::PresetNotFound(_))));
    }

    #[test]
    fn test_remove_preset_last_preset_returns_last_preset_error() {
        // Arrange
        let dir = TempDir::new().unwrap();
        save_config(dir.path(), &complete_config()).unwrap();

        // Act
        let result = remove_preset(dir.path(), "dev");

        // Assert
        assert!(matches!(result, Err(AppError::LastPreset)));
    }

    #[test]
    fn test_remove_preset_preserves_other_sections() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let mut config = complete_config();
        let mut presets = config.presets.take().unwrap();
        presets.push(Preset {
            key: "design".to_string(),
            description: "Design work".to_string(),
            default_rate: 80.0,
        });
        config.presets = Some(presets);
        save_config(dir.path(), &config).unwrap();

        // Act
        remove_preset(dir.path(), "design").unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(dir.path()));
        assert_eq!(loaded.sender, config.sender);
        assert_eq!(loaded.recipient, config.recipient);
        assert_eq!(loaded.payment, config.payment);
        assert_eq!(loaded.defaults, config.defaults);
    }

    #[test]
    fn test_remove_preset_no_config_file_returns_config_io() {
        // Arrange
        let dir = TempDir::new().unwrap();

        // Act
        let result = remove_preset(dir.path(), "dev");

        // Assert
        assert!(matches!(result, Err(AppError::ConfigIo(_))));
    }

    #[test]
    fn test_remove_preset_from_three_presets_removes_middle() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let config = Config {
            presets: Some(vec![
                Preset {
                    key: "dev".to_string(),
                    description: "Development".to_string(),
                    default_rate: 100.0,
                },
                Preset {
                    key: "design".to_string(),
                    description: "Design".to_string(),
                    default_rate: 80.0,
                },
                Preset {
                    key: "qa".to_string(),
                    description: "QA".to_string(),
                    default_rate: 60.0,
                },
            ]),
            ..complete_config()
        };
        save_config(dir.path(), &config).unwrap();

        // Act
        let removed = remove_preset(dir.path(), "design").unwrap();

        // Assert
        assert_eq!(removed.key, "design");
        let loaded = unwrap_loaded(load_config(dir.path()));
        let remaining = loaded.presets.unwrap();
        assert_eq!(remaining.len(), 2);
        assert_eq!(remaining[0].key, "dev");
        assert_eq!(remaining[1].key, "qa");
    }

    // ── Story 7.1 Phase 6: v2 round-trip tests ──

    #[test]
    fn test_save_config_v2_recipients_round_trips() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let config = Config {
            sender: Some(synthetic_sender()),
            recipient: None,
            recipients: Some(vec![Recipient {
                key: Some("acme".into()),
                name: "Acme Corp".into(),
                address: vec!["123 St".into()],
                company_id: Some("AC-123".into()),
                vat_number: None,
            }]),
            default_recipient: Some("acme".into()),
            payment: Some(synthetic_payment()),
            presets: Some(synthetic_presets()),
            defaults: Some(synthetic_defaults()),
        };

        // Act
        save_config(dir.path(), &config).unwrap();
        let loaded = unwrap_loaded(load_config(dir.path()));

        // Assert
        assert_eq!(loaded.recipients.as_ref().unwrap().len(), 1);
        assert_eq!(
            loaded.recipients.as_ref().unwrap()[0].key,
            Some("acme".into())
        );
        assert_eq!(loaded.default_recipient, Some("acme".into()));
    }

    #[test]
    fn test_save_config_v2_no_null_in_yaml() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let config = Config {
            sender: Some(synthetic_sender()),
            recipient: None,
            recipients: Some(vec![Recipient {
                key: Some("bob".into()),
                name: "Bob Corp".into(),
                address: vec!["St".into()],
                company_id: None,
                vat_number: None,
            }]),
            default_recipient: Some("bob".into()),
            payment: Some(synthetic_payment()),
            presets: Some(synthetic_presets()),
            defaults: Some(synthetic_defaults()),
        };

        // Act
        save_config(dir.path(), &config).unwrap();
        let raw = std::fs::read_to_string(dir.path().join(CONFIG_FILENAME)).unwrap();

        // Assert
        assert!(!raw.contains("null"), "YAML output should not contain 'null'");
    }

    // ── v2 Config Helpers ──

    fn complete_config_v2() -> Config {
        Config {
            sender: Some(synthetic_sender()),
            recipient: None,
            recipients: Some(vec![Recipient {
                key: Some("acme".into()),
                name: "Acme Corp".into(),
                address: vec!["100 Acme Blvd".into()],
                company_id: Some("AC-12345".into()),
                vat_number: None,
            }]),
            default_recipient: Some("acme".into()),
            payment: Some(synthetic_payment()),
            presets: Some(synthetic_presets()),
            defaults: Some(synthetic_defaults()),
        }
    }

    fn complete_config_v2_two_recipients() -> Config {
        Config {
            sender: Some(synthetic_sender()),
            recipient: None,
            recipients: Some(vec![
                Recipient {
                    key: Some("acme".into()),
                    name: "Acme Corp".into(),
                    address: vec!["100 Acme Blvd".into()],
                    company_id: Some("AC-12345".into()),
                    vat_number: None,
                },
                Recipient {
                    key: Some("globex".into()),
                    name: "Globex Inc".into(),
                    address: vec!["200 Globex Ave".into()],
                    company_id: None,
                    vat_number: Some("CZ87654321".into()),
                },
            ]),
            default_recipient: Some("acme".into()),
            payment: Some(synthetic_payment()),
            presets: Some(synthetic_presets()),
            defaults: Some(synthetic_defaults()),
        }
    }

    #[test]
    fn test_remove_preset_key_is_case_sensitive() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let config = Config {
            presets: Some(vec![
                Preset {
                    key: "dev".to_string(),
                    description: "Development".to_string(),
                    default_rate: 100.0,
                },
                Preset {
                    key: "design".to_string(),
                    description: "Design".to_string(),
                    default_rate: 80.0,
                },
            ]),
            ..complete_config()
        };
        save_config(dir.path(), &config).unwrap();

        // Act
        let result = remove_preset(dir.path(), "Dev");

        // Assert
        assert!(matches!(result, Err(AppError::PresetNotFound(_))));
    }

    // ── append_recipient tests ──

    #[test]
    fn test_append_recipient_to_config_without_recipients() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let config = Config {
            sender: Some(synthetic_sender()),
            ..Config::default()
        };
        save_config(dir.path(), &config).unwrap();

        let recipient = Recipient {
            key: Some("acme".into()),
            name: "Acme Corp".into(),
            address: vec!["100 Acme Blvd".into()],
            company_id: None,
            vat_number: None,
        };

        // Act
        append_recipient(dir.path(), recipient, true).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(dir.path()));
        let recipients = loaded.recipients.unwrap();
        assert_eq!(recipients.len(), 1);
        assert_eq!(recipients[0].key, Some("acme".into()));
        assert_eq!(loaded.default_recipient, Some("acme".into()));
    }

    #[test]
    fn test_append_recipient_to_existing_recipients() {
        // Arrange
        let dir = TempDir::new().unwrap();
        save_config(dir.path(), &complete_config_v2()).unwrap();

        let new_recipient = Recipient {
            key: Some("globex".into()),
            name: "Globex Inc".into(),
            address: vec!["200 Globex Ave".into()],
            company_id: None,
            vat_number: None,
        };

        // Act
        append_recipient(dir.path(), new_recipient, false).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(dir.path()));
        let recipients = loaded.recipients.unwrap();
        assert_eq!(recipients.len(), 2);
        assert_eq!(recipients[1].key, Some("globex".into()));
        assert_eq!(loaded.default_recipient, Some("acme".into()));
    }

    #[test]
    fn test_append_recipient_set_default_updates_key() {
        // Arrange
        let dir = TempDir::new().unwrap();
        save_config(dir.path(), &complete_config_v2()).unwrap();

        let new_recipient = Recipient {
            key: Some("globex".into()),
            name: "Globex Inc".into(),
            address: vec!["200 Globex Ave".into()],
            company_id: None,
            vat_number: None,
        };

        // Act
        append_recipient(dir.path(), new_recipient, true).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(dir.path()));
        assert_eq!(loaded.default_recipient, Some("globex".into()));
    }

    #[test]
    fn test_append_recipient_preserves_other_sections() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let original = complete_config_v2();
        save_config(dir.path(), &original).unwrap();

        let new_recipient = Recipient {
            key: Some("globex".into()),
            name: "Globex Inc".into(),
            address: vec!["200 Globex Ave".into()],
            company_id: None,
            vat_number: None,
        };

        // Act
        append_recipient(dir.path(), new_recipient, false).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(dir.path()));
        assert_eq!(loaded.sender, original.sender);
        assert_eq!(loaded.payment, original.payment);
        assert_eq!(loaded.presets, original.presets);
        assert_eq!(loaded.defaults, original.defaults);
    }

    #[test]
    fn test_append_recipient_no_config_returns_error() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let recipient = Recipient {
            key: Some("acme".into()),
            name: "Acme Corp".into(),
            address: vec!["St".into()],
            company_id: None,
            vat_number: None,
        };

        // Act
        let result = append_recipient(dir.path(), recipient, true);

        // Assert
        assert!(matches!(result, Err(AppError::ConfigIo(_))));
    }

    // ── remove_recipient tests ──

    #[test]
    fn test_remove_recipient_deletes_matching_key() {
        // Arrange
        let dir = TempDir::new().unwrap();
        save_config(dir.path(), &complete_config_v2_two_recipients()).unwrap();

        // Act
        let removed = remove_recipient(dir.path(), "globex").unwrap();

        // Assert
        assert_eq!(removed.name, "Globex Inc");
        let loaded = unwrap_loaded(load_config(dir.path()));
        let recipients = loaded.recipients.unwrap();
        assert_eq!(recipients.len(), 1);
        assert_eq!(recipients[0].key, Some("acme".into()));
        assert_eq!(loaded.default_recipient, Some("acme".into()));
    }

    #[test]
    fn test_remove_recipient_unknown_key_returns_error() {
        // Arrange
        let dir = TempDir::new().unwrap();
        save_config(dir.path(), &complete_config_v2_two_recipients()).unwrap();

        // Act
        let result = remove_recipient(dir.path(), "nope");

        // Assert
        assert!(matches!(result, Err(AppError::RecipientNotFound { .. })));
    }

    #[test]
    fn test_remove_recipient_last_returns_error() {
        // Arrange
        let dir = TempDir::new().unwrap();
        save_config(dir.path(), &complete_config_v2()).unwrap();

        // Act
        let result = remove_recipient(dir.path(), "acme");

        // Assert
        assert!(matches!(result, Err(AppError::LastRecipient)));
    }

    #[test]
    fn test_remove_recipient_preserves_other_sections() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let original = complete_config_v2_two_recipients();
        save_config(dir.path(), &original).unwrap();

        // Act
        remove_recipient(dir.path(), "globex").unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(dir.path()));
        assert_eq!(loaded.sender, original.sender);
        assert_eq!(loaded.payment, original.payment);
        assert_eq!(loaded.presets, original.presets);
        assert_eq!(loaded.defaults, original.defaults);
    }

    #[test]
    fn test_remove_recipient_no_config_returns_error() {
        // Arrange
        let dir = TempDir::new().unwrap();

        // Act
        let result = remove_recipient(dir.path(), "acme");

        // Assert
        assert!(matches!(result, Err(AppError::ConfigIo(_))));
    }

    // ── set_default_recipient tests ──

    #[test]
    fn test_set_default_recipient_updates_key() {
        // Arrange
        let dir = TempDir::new().unwrap();
        save_config(dir.path(), &complete_config_v2_two_recipients()).unwrap();

        // Act
        set_default_recipient(dir.path(), "globex").unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(dir.path()));
        assert_eq!(loaded.default_recipient, Some("globex".into()));
    }

    #[test]
    fn test_set_default_recipient_unknown_key_returns_error() {
        // Arrange
        let dir = TempDir::new().unwrap();
        save_config(dir.path(), &complete_config_v2_two_recipients()).unwrap();

        // Act
        let result = set_default_recipient(dir.path(), "nope");

        // Assert
        assert!(matches!(result, Err(AppError::RecipientNotFound { .. })));
    }
}
