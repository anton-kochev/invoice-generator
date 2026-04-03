use std::path::Path;

use crate::error::AppError;

use super::loader::CONFIG_FILENAME;
use super::types::{Config, Preset};

/// Serialize `config` to YAML and write it to `dir/CONFIG_FILENAME`.
pub fn save_config(dir: &Path, config: &Config) -> Result<(), AppError> {
    let yaml = serde_yaml::to_string(config)?;
    let path = dir.join(CONFIG_FILENAME);
    std::fs::write(path, yaml)?;
    Ok(())
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
            recipient: Some(synthetic_recipient()),
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
}
