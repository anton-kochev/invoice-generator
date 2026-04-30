use std::io::Write;
use std::path::Path;

use crate::error::AppError;

use super::types::{Config, Preset, Recipient};

/// Serialize `config` to YAML and atomically write it to `path`.
///
/// Uses the standard write-temp-then-rename pattern: a [`tempfile::NamedTempFile`]
/// is created in the same directory as the target so [`tempfile::NamedTempFile::persist`]
/// can complete via a single `rename(2)` syscall, which is atomic on POSIX
/// filesystems. If the process is killed (Ctrl-C, OOM, panic, power loss) at
/// any point before the rename, the existing file at `path` is left untouched —
/// callers never observe a truncated or partially written config.
///
/// If `path` has no parent component (e.g. a bare `config.yaml`), the temp
/// file is created in the current working directory. This matches the
/// behaviour of the original `std::fs::write` for relative paths.
pub fn save_config(path: &Path, config: &Config) -> Result<(), AppError> {
    let yaml = serde_yaml::to_string(config)?;

    // Use the same directory as the target so the final rename stays on the
    // same filesystem (cross-fs renames are not atomic on most platforms).
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));

    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(yaml.as_bytes())?;
    // Flush the kernel buffers before the rename so a crash post-rename
    // doesn't leave us with a renamed-but-empty file on some filesystems.
    tmp.as_file().sync_all()?;
    tmp.persist(path).map_err(|e| AppError::from(e.error))?;
    Ok(())
}

/// Remove a preset by key from the config file at `path`.
///
/// Returns the removed preset on success.
/// Checks the last-preset guard BEFORE key lookup.
pub fn remove_preset(path: &Path, key: &str) -> Result<Preset, AppError> {
    use super::loader::{load_config, LoadResult};

    let config = match load_config(path)? {
        LoadResult::Loaded(config) => *config,
        LoadResult::NotFound => {
            return Err(AppError::ConfigIo(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("config file not found at {}", path.display()),
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
        .position(|p| p.key.as_str() == key)
        .ok_or_else(|| AppError::PresetNotFound(key.to_string()))?;

    let removed = presets.remove(pos);
    config.presets = Some(presets);

    save_config(path, &config)?;
    Ok(removed)
}

/// Append a preset to the config file at `path`.
///
/// Loads the existing config, pushes the new preset, and saves it back.
/// Returns an error if no config file exists yet.
pub fn append_preset(path: &Path, preset: Preset) -> Result<(), AppError> {
    use super::loader::{load_config, LoadResult};

    let config = match load_config(path)? {
        LoadResult::Loaded(config) => *config,
        LoadResult::NotFound => {
            return Err(AppError::ConfigIo(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("config file not found at {}", path.display()),
            )));
        }
    };

    let mut config = config;
    let mut presets = config.presets.unwrap_or_default();
    presets.push(preset);
    config.presets = Some(presets);

    save_config(path, &config)
}

/// Migrate a v1 Config (single `recipient:`) to v2 (`recipients:` list).
/// No-op if already v2. Consumes the `recipient` field via `.take()`.
fn ensure_recipients_v2(config: &mut Config) -> Result<(), AppError> {
    use crate::domain::RecipientKey;

    if config.recipients.is_some() {
        return Ok(());
    }
    if let Some(mut legacy) = config.recipient.take() {
        let key = match legacy.key.clone() {
            Some(k) => k,
            None => RecipientKey::from_name(&legacy.name)
                .map_err(|e| AppError::InvalidDefaultRecipient(e.to_string()))?,
        };
        legacy.key = Some(key.clone());
        config.recipients = Some(vec![legacy]);
        if config.default_recipient.is_none() {
            config.default_recipient = Some(key);
        }
    }
    Ok(())
}

/// Append a recipient to the config file at `path`.
///
/// If `set_default` is true, also sets `default_recipient` to the new recipient's key.
pub fn append_recipient(path: &Path, recipient: Recipient, set_default: bool) -> Result<(), AppError> {
    use super::loader::{load_config, LoadResult};

    let config = match load_config(path)? {
        LoadResult::Loaded(config) => *config,
        LoadResult::NotFound => {
            return Err(AppError::ConfigIo(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("config file not found at {}", path.display()),
            )));
        }
    };

    let mut config = config;
    ensure_recipients_v2(&mut config)?;
    let mut recipients = config.recipients.take().unwrap_or_default();

    if set_default {
        config.default_recipient = recipient.key.clone();
    }

    recipients.push(recipient);
    config.recipients = Some(recipients);

    save_config(path, &config)
}

/// Remove a recipient by key from the config file at `path`.
///
/// Returns the removed recipient on success.
/// If the removed recipient was the default, clears `default_recipient` (caller handles reassignment).
pub fn remove_recipient(path: &Path, key: &str) -> Result<Recipient, AppError> {
    use super::loader::{load_config, LoadResult};

    let config = match load_config(path)? {
        LoadResult::Loaded(config) => *config,
        LoadResult::NotFound => {
            return Err(AppError::ConfigIo(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("config file not found at {}", path.display()),
            )));
        }
    };

    let mut config = config;
    ensure_recipients_v2(&mut config)?;
    let mut recipients = config.recipients.take().unwrap_or_default();

    if recipients.len() <= 1 {
        return Err(AppError::LastRecipient);
    }

    let pos = recipients
        .iter()
        .position(|r| r.key.as_ref().is_some_and(|k| k.as_str() == key))
        .ok_or_else(|| AppError::RecipientNotFound {
            key: key.to_string(),
            available: recipients
                .iter()
                .filter_map(|r| r.key.as_ref().map(|k| k.as_str().to_string()))
                .collect(),
        })?;

    let removed = recipients.remove(pos);
    config.recipients = Some(recipients);

    // Clear default if it was the removed recipient
    if config
        .default_recipient
        .as_ref()
        .is_some_and(|k| k.as_str() == key)
    {
        config.default_recipient = None;
    }

    save_config(path, &config)?;
    Ok(removed)
}

/// Set the default recipient key in the config file at `path`.
///
/// Verifies the key exists in the recipients list before updating.
pub fn set_default_recipient(path: &Path, key: &str) -> Result<(), AppError> {
    use super::loader::{load_config, LoadResult};

    let config = match load_config(path)? {
        LoadResult::Loaded(config) => *config,
        LoadResult::NotFound => {
            return Err(AppError::ConfigIo(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("config file not found at {}", path.display()),
            )));
        }
    };

    let mut config = config;
    ensure_recipients_v2(&mut config)?;
    let recipients = config.recipients.as_deref().unwrap_or_default();

    if !recipients
        .iter()
        .any(|r| r.key.as_ref().is_some_and(|k| k.as_str() == key))
    {
        return Err(AppError::RecipientNotFound {
            key: key.to_string(),
            available: recipients
                .iter()
                .filter_map(|r| r.key.as_ref().map(|k| k.as_str().to_string()))
                .collect(),
        });
    }

    config.default_recipient = Some(
        crate::domain::RecipientKey::try_new(key)
            .map_err(|e| AppError::InvalidDefaultRecipient(e.to_string()))?,
    );
    save_config(path, &config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::loader::{load_config, LoadResult};
    use crate::config::types::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Path to the config file inside a tempdir — tests now pass a file path,
    /// not a directory, since the loader/writer functions take `&Path` to a file.
    fn cfg_path(dir: &TempDir) -> PathBuf {
        dir.path().join("config.yaml")
    }

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
            key: Some(crate::domain::RecipientKey::try_new("bob-corp").unwrap()),
            name: "Bob Corp".to_string(),
            address: vec!["99 Oak Lane".to_string(), "Shelbyville, IL 62565".to_string()],
            company_id: Some("BC-98765".to_string()),
            vat_number: Some("CZ12345678".to_string()),
        }
    }

    fn synthetic_payment() -> Vec<PaymentMethod> {
        vec![PaymentMethod {
            label: "SEPA Transfer".to_string(),
            iban: crate::domain::Iban::try_new("DE89370400440532013000").unwrap(),
            bic_swift: "COBADEFFXXX".to_string(),
        }]
    }

    fn synthetic_presets() -> Vec<Preset> {
        vec![Preset {
            key: crate::domain::PresetKey::try_new("dev").unwrap(),
            description: "Development Services".to_string(),
            default_rate: 100.0,
            currency: None,
        tax_rate: None,
        }]
    }

    fn synthetic_defaults() -> Defaults {
        Defaults {
            currency: crate::domain::Currency::Usd,
            invoice_date_day: 5,
            payment_terms_days: 14,
            ..Defaults::default()
        }
    }

    fn complete_config() -> Config {
        Config {
            sender: Some(synthetic_sender()),
            recipient: None,
            recipients: Some(vec![synthetic_recipient()]),
            default_recipient: Some(crate::domain::RecipientKey::try_new("bob-corp").unwrap()),
            payment: Some(synthetic_payment()),
            presets: Some(synthetic_presets()),
            defaults: Some(synthetic_defaults()),
            branding: None,
        }
    }

    fn unwrap_loaded(result: Result<LoadResult, AppError>) -> Config {
        match result.unwrap() {
            LoadResult::Loaded(c) => *c,
            LoadResult::NotFound => panic!("Expected Loaded, got NotFound"),
        }
    }

    // ── Cycle 1: test_save_config_creates_file ──

    #[test]
    fn test_save_config_creates_file() {
        // Arrange
        let dir = TempDir::new().unwrap();

        // Act
        let result = save_config(&cfg_path(&dir), &Config::default());

        // Assert
        assert!(result.is_ok());
        assert!(cfg_path(&dir).exists());
    }

    // ── Cycle 2: test_save_config_complete_round_trips ──

    #[test]
    fn test_save_config_complete_round_trips() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let original = complete_config();

        // Act
        save_config(&cfg_path(&dir), &original).unwrap();
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));

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
        save_config(&cfg_path(&dir), &config).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        assert_eq!(loaded.sender, Some(synthetic_sender()));
        assert!(loaded.recipient.is_none());
        assert!(loaded.payment.is_none());
        assert!(loaded.presets.is_none());
        assert!(loaded.defaults.is_none());

        let raw = std::fs::read_to_string(cfg_path(&dir)).unwrap();
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
        save_config(&cfg_path(&dir), &alice).unwrap();

        let bob = Config {
            sender: Some(Sender {
                name: "Bob".to_string(),
                address: vec!["Street 2".to_string()],
                email: "bob@example.com".to_string(),
            }),
            ..Config::default()
        };

        // Act
        save_config(&cfg_path(&dir), &bob).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        assert_eq!(loaded.sender.unwrap().name, "Bob");
    }

    // ── Cycle 5: test_save_config_produces_valid_yaml ──

    #[test]
    fn test_save_config_produces_valid_yaml() {
        // Arrange
        let dir = TempDir::new().unwrap();

        // Act
        save_config(&cfg_path(&dir), &complete_config()).unwrap();
        let raw = std::fs::read_to_string(cfg_path(&dir)).unwrap();

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
        let result = save_config(&cfg_path(&dir), &Config::default());

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
        save_config(&cfg_path(&dir), &complete_config()).unwrap();
        let new_preset = Preset {
            key: crate::domain::PresetKey::try_new("design").unwrap(),
            description: "Design work".to_string(),
            default_rate: 80.0,
            currency: None,
        tax_rate: None,
        };

        // Act
        append_preset(&cfg_path(&dir), new_preset).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        let presets = loaded.presets.unwrap();
        assert_eq!(presets.len(), 2);
        assert_eq!(presets[0].key.as_str(), "dev");
        assert_eq!(presets[1].key.as_str(), "design");
    }

    // ── Cycle 8: test_append_preset_preserves_other_sections ──

    #[test]
    fn test_append_preset_preserves_other_sections() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let original = complete_config();
        save_config(&cfg_path(&dir), &original).unwrap();

        // Act
        append_preset(
            &cfg_path(&dir),
            Preset {
                key: crate::domain::PresetKey::try_new("qa").unwrap(),
                description: "QA work".to_string(),
                default_rate: 60.0,
                currency: None,
            tax_rate: None,
            },
        )
        .unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
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
        save_config(&cfg_path(&dir), &config).unwrap();

        // Act
        append_preset(
            &cfg_path(&dir),
            Preset {
                key: crate::domain::PresetKey::try_new("ops").unwrap(),
                description: "Operations".to_string(),
                default_rate: 90.0,
                currency: None,
            tax_rate: None,
            },
        )
        .unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        let presets = loaded.presets.unwrap();
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].key.as_str(), "ops");
    }

    // ── Cycle 10: test_append_preset_no_config_file ──

    #[test]
    fn test_append_preset_no_config_file() {
        // Arrange
        let dir = TempDir::new().unwrap();

        // Act
        let result = append_preset(
            &cfg_path(&dir),
            Preset {
                key: crate::domain::PresetKey::try_new("x").unwrap(),
                description: "X".to_string(),
                default_rate: 50.0,
                currency: None,
            tax_rate: None,
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
            key: crate::domain::PresetKey::try_new("design").unwrap(),
            description: "Design work".to_string(),
            default_rate: 80.0,
            currency: None,
        tax_rate: None,
        });
        config.presets = Some(presets);
        save_config(&cfg_path(&dir), &config).unwrap();

        // Act
        let removed = remove_preset(&cfg_path(&dir), "design").unwrap();

        // Assert
        assert_eq!(removed.key.as_str(), "design");
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        let remaining = loaded.presets.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].key.as_str(), "dev");
    }

    #[test]
    fn test_remove_preset_unknown_key_returns_preset_not_found() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let mut config = complete_config();
        let mut presets = config.presets.take().unwrap();
        presets.push(Preset {
            key: crate::domain::PresetKey::try_new("design").unwrap(),
            description: "Design work".to_string(),
            default_rate: 80.0,
            currency: None,
        tax_rate: None,
        });
        config.presets = Some(presets);
        save_config(&cfg_path(&dir), &config).unwrap();

        // Act
        let result = remove_preset(&cfg_path(&dir), "nope");

        // Assert
        assert!(matches!(result, Err(AppError::PresetNotFound(_))));
    }

    #[test]
    fn test_remove_preset_last_preset_returns_last_preset_error() {
        // Arrange
        let dir = TempDir::new().unwrap();
        save_config(&cfg_path(&dir), &complete_config()).unwrap();

        // Act
        let result = remove_preset(&cfg_path(&dir), "dev");

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
            key: crate::domain::PresetKey::try_new("design").unwrap(),
            description: "Design work".to_string(),
            default_rate: 80.0,
            currency: None,
        tax_rate: None,
        });
        config.presets = Some(presets);
        save_config(&cfg_path(&dir), &config).unwrap();

        // Act
        remove_preset(&cfg_path(&dir), "design").unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
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
        let result = remove_preset(&cfg_path(&dir), "dev");

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
                    key: crate::domain::PresetKey::try_new("dev").unwrap(),
                    description: "Development".to_string(),
                    default_rate: 100.0,
                    currency: None,
                tax_rate: None,
                },
                Preset {
                    key: crate::domain::PresetKey::try_new("design").unwrap(),
                    description: "Design".to_string(),
                    default_rate: 80.0,
                    currency: None,
                tax_rate: None,
                },
                Preset {
                    key: crate::domain::PresetKey::try_new("qa").unwrap(),
                    description: "QA".to_string(),
                    default_rate: 60.0,
                    currency: None,
                tax_rate: None,
                },
            ]),
            ..complete_config()
        };
        save_config(&cfg_path(&dir), &config).unwrap();

        // Act
        let removed = remove_preset(&cfg_path(&dir), "design").unwrap();

        // Assert
        assert_eq!(removed.key.as_str(), "design");
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        let remaining = loaded.presets.unwrap();
        assert_eq!(remaining.len(), 2);
        assert_eq!(remaining[0].key.as_str(), "dev");
        assert_eq!(remaining[1].key.as_str(), "qa");
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
                key: Some(crate::domain::RecipientKey::try_new("acme").unwrap()),
                name: "Acme Corp".into(),
                address: vec!["123 St".into()],
                company_id: Some("AC-123".into()),
                vat_number: None,
            }]),
            default_recipient: Some(crate::domain::RecipientKey::try_new("acme").unwrap()),
            payment: Some(synthetic_payment()),
            presets: Some(synthetic_presets()),
            defaults: Some(synthetic_defaults()),
            branding: None,
        };

        // Act
        save_config(&cfg_path(&dir), &config).unwrap();
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));

        // Assert
        assert_eq!(loaded.recipients.as_ref().unwrap().len(), 1);
        assert_eq!(
            loaded.recipients.as_ref().unwrap()[0].key,
            Some(crate::domain::RecipientKey::try_new("acme").unwrap())
        );
        assert_eq!(
            loaded.default_recipient.as_ref().map(|k| k.as_str()),
            Some("acme")
        );
    }

    #[test]
    fn test_save_config_v2_no_null_in_yaml() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let config = Config {
            sender: Some(synthetic_sender()),
            recipient: None,
            recipients: Some(vec![Recipient {
                key: Some(crate::domain::RecipientKey::try_new("bob").unwrap()),
                name: "Bob Corp".into(),
                address: vec!["St".into()],
                company_id: None,
                vat_number: None,
            }]),
            default_recipient: Some(crate::domain::RecipientKey::try_new("bob").unwrap()),
            payment: Some(synthetic_payment()),
            presets: Some(synthetic_presets()),
            defaults: Some(synthetic_defaults()),
            branding: None,
        };

        // Act
        save_config(&cfg_path(&dir), &config).unwrap();
        let raw = std::fs::read_to_string(cfg_path(&dir)).unwrap();

        // Assert
        assert!(!raw.contains("null"), "YAML output should not contain 'null'");
    }

    // ── v2 Config Helpers ──

    fn complete_config_v2() -> Config {
        Config {
            sender: Some(synthetic_sender()),
            recipient: None,
            recipients: Some(vec![Recipient {
                key: Some(crate::domain::RecipientKey::try_new("acme").unwrap()),
                name: "Acme Corp".into(),
                address: vec!["100 Acme Blvd".into()],
                company_id: Some("AC-12345".into()),
                vat_number: None,
            }]),
            default_recipient: Some(crate::domain::RecipientKey::try_new("acme").unwrap()),
            payment: Some(synthetic_payment()),
            presets: Some(synthetic_presets()),
            defaults: Some(synthetic_defaults()),
            branding: None,
        }
    }

    fn complete_config_v2_two_recipients() -> Config {
        Config {
            sender: Some(synthetic_sender()),
            recipient: None,
            recipients: Some(vec![
                Recipient {
                    key: Some(crate::domain::RecipientKey::try_new("acme").unwrap()),
                    name: "Acme Corp".into(),
                    address: vec!["100 Acme Blvd".into()],
                    company_id: Some("AC-12345".into()),
                    vat_number: None,
                },
                Recipient {
                    key: Some(crate::domain::RecipientKey::try_new("globex").unwrap()),
                    name: "Globex Inc".into(),
                    address: vec!["200 Globex Ave".into()],
                    company_id: None,
                    vat_number: Some("CZ87654321".into()),
                },
            ]),
            default_recipient: Some(crate::domain::RecipientKey::try_new("acme").unwrap()),
            payment: Some(synthetic_payment()),
            presets: Some(synthetic_presets()),
            defaults: Some(synthetic_defaults()),
            branding: None,
        }
    }

    #[test]
    fn test_remove_preset_key_is_case_sensitive() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let config = Config {
            presets: Some(vec![
                Preset {
                    key: crate::domain::PresetKey::try_new("dev").unwrap(),
                    description: "Development".to_string(),
                    default_rate: 100.0,
                    currency: None,
                tax_rate: None,
                },
                Preset {
                    key: crate::domain::PresetKey::try_new("design").unwrap(),
                    description: "Design".to_string(),
                    default_rate: 80.0,
                    currency: None,
                tax_rate: None,
                },
            ]),
            ..complete_config()
        };
        save_config(&cfg_path(&dir), &config).unwrap();

        // Act
        let result = remove_preset(&cfg_path(&dir), "Dev");

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
        save_config(&cfg_path(&dir), &config).unwrap();

        let recipient = Recipient {
            key: Some(crate::domain::RecipientKey::try_new("acme").unwrap()),
            name: "Acme Corp".into(),
            address: vec!["100 Acme Blvd".into()],
            company_id: None,
            vat_number: None,
        };

        // Act
        append_recipient(&cfg_path(&dir), recipient, true).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        let recipients = loaded.recipients.unwrap();
        assert_eq!(recipients.len(), 1);
        assert_eq!(
            recipients[0].key.as_ref().map(|k| k.as_str()),
            Some("acme")
        );
        assert_eq!(
            loaded.default_recipient.as_ref().map(|k| k.as_str()),
            Some("acme")
        );
    }

    #[test]
    fn test_append_recipient_to_existing_recipients() {
        // Arrange
        let dir = TempDir::new().unwrap();
        save_config(&cfg_path(&dir), &complete_config_v2()).unwrap();

        let new_recipient = Recipient {
            key: Some(crate::domain::RecipientKey::try_new("globex").unwrap()),
            name: "Globex Inc".into(),
            address: vec!["200 Globex Ave".into()],
            company_id: None,
            vat_number: None,
        };

        // Act
        append_recipient(&cfg_path(&dir), new_recipient, false).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        let recipients = loaded.recipients.unwrap();
        assert_eq!(recipients.len(), 2);
        assert_eq!(
            recipients[1].key.as_ref().map(|k| k.as_str()),
            Some("globex")
        );
        assert_eq!(
            loaded.default_recipient.as_ref().map(|k| k.as_str()),
            Some("acme")
        );
    }

    #[test]
    fn test_append_recipient_set_default_updates_key() {
        // Arrange
        let dir = TempDir::new().unwrap();
        save_config(&cfg_path(&dir), &complete_config_v2()).unwrap();

        let new_recipient = Recipient {
            key: Some(crate::domain::RecipientKey::try_new("globex").unwrap()),
            name: "Globex Inc".into(),
            address: vec!["200 Globex Ave".into()],
            company_id: None,
            vat_number: None,
        };

        // Act
        append_recipient(&cfg_path(&dir), new_recipient, true).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        assert_eq!(
            loaded.default_recipient.as_ref().map(|k| k.as_str()),
            Some("globex")
        );
    }

    #[test]
    fn test_append_recipient_preserves_other_sections() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let original = complete_config_v2();
        save_config(&cfg_path(&dir), &original).unwrap();

        let new_recipient = Recipient {
            key: Some(crate::domain::RecipientKey::try_new("globex").unwrap()),
            name: "Globex Inc".into(),
            address: vec!["200 Globex Ave".into()],
            company_id: None,
            vat_number: None,
        };

        // Act
        append_recipient(&cfg_path(&dir), new_recipient, false).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
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
            key: Some(crate::domain::RecipientKey::try_new("acme").unwrap()),
            name: "Acme Corp".into(),
            address: vec!["St".into()],
            company_id: None,
            vat_number: None,
        };

        // Act
        let result = append_recipient(&cfg_path(&dir), recipient, true);

        // Assert
        assert!(matches!(result, Err(AppError::ConfigIo(_))));
    }

    // ── remove_recipient tests ──

    #[test]
    fn test_remove_recipient_deletes_matching_key() {
        // Arrange
        let dir = TempDir::new().unwrap();
        save_config(&cfg_path(&dir), &complete_config_v2_two_recipients()).unwrap();

        // Act
        let removed = remove_recipient(&cfg_path(&dir), "globex").unwrap();

        // Assert
        assert_eq!(removed.name, "Globex Inc");
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        let recipients = loaded.recipients.unwrap();
        assert_eq!(recipients.len(), 1);
        assert_eq!(
            recipients[0].key.as_ref().map(|k| k.as_str()),
            Some("acme")
        );
        assert_eq!(
            loaded.default_recipient.as_ref().map(|k| k.as_str()),
            Some("acme")
        );
    }

    #[test]
    fn test_remove_recipient_unknown_key_returns_error() {
        // Arrange
        let dir = TempDir::new().unwrap();
        save_config(&cfg_path(&dir), &complete_config_v2_two_recipients()).unwrap();

        // Act
        let result = remove_recipient(&cfg_path(&dir), "nope");

        // Assert
        assert!(matches!(result, Err(AppError::RecipientNotFound { .. })));
    }

    #[test]
    fn test_remove_recipient_last_returns_error() {
        // Arrange
        let dir = TempDir::new().unwrap();
        save_config(&cfg_path(&dir), &complete_config_v2()).unwrap();

        // Act
        let result = remove_recipient(&cfg_path(&dir), "acme");

        // Assert
        assert!(matches!(result, Err(AppError::LastRecipient)));
    }

    #[test]
    fn test_remove_recipient_preserves_other_sections() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let original = complete_config_v2_two_recipients();
        save_config(&cfg_path(&dir), &original).unwrap();

        // Act
        remove_recipient(&cfg_path(&dir), "globex").unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
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
        let result = remove_recipient(&cfg_path(&dir), "acme");

        // Assert
        assert!(matches!(result, Err(AppError::ConfigIo(_))));
    }

    // ── set_default_recipient tests ──

    #[test]
    fn test_set_default_recipient_updates_key() {
        // Arrange
        let dir = TempDir::new().unwrap();
        save_config(&cfg_path(&dir), &complete_config_v2_two_recipients()).unwrap();

        // Act
        set_default_recipient(&cfg_path(&dir), "globex").unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        assert_eq!(
            loaded.default_recipient.as_ref().map(|k| k.as_str()),
            Some("globex")
        );
    }

    // ── v1 migration helpers ──

    fn v1_config_with_recipient() -> Config {
        Config {
            sender: Some(synthetic_sender()),
            recipient: Some(Recipient {
                key: None,
                name: "Client Corp".into(),
                address: vec!["456 Client Ave".into()],
                company_id: Some("CC-12345".into()),
                vat_number: Some("VAT-999".into()),
            }),
            recipients: None,
            default_recipient: None,
            payment: Some(synthetic_payment()),
            presets: Some(synthetic_presets()),
            defaults: Some(Defaults::default()),
            branding: None,
        }
    }

    fn new_recipient() -> Recipient {
        Recipient {
            key: Some(crate::domain::RecipientKey::try_new("macrosoft").unwrap()),
            name: "Macrosoft".into(),
            address: vec!["654 Street".into(), "United States".into()],
            company_id: Some("MS-54321".into()),
            vat_number: None,
        }
    }

    // ── v1→v2 migration tests ──

    #[test]
    fn test_append_recipient_v1_no_key_set_default_false_migrates() {
        // Arrange
        let dir = TempDir::new().unwrap();
        save_config(&cfg_path(&dir), &v1_config_with_recipient()).unwrap();

        // Act
        append_recipient(&cfg_path(&dir), new_recipient(), false).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        let recipients = loaded.recipients.unwrap();
        assert_eq!(recipients.len(), 2);
        assert_eq!(recipients[0].key, Some(crate::domain::RecipientKey::try_new("client-corp").unwrap()));
        assert_eq!(recipients[1].key, Some(crate::domain::RecipientKey::try_new("macrosoft").unwrap()));
        assert_eq!(loaded.default_recipient, Some(crate::domain::RecipientKey::try_new("client-corp").unwrap()));
    }

    #[test]
    fn test_append_recipient_v1_with_key_set_default_false_migrates() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let mut config = v1_config_with_recipient();
        config.recipient.as_mut().unwrap().key = Some(crate::domain::RecipientKey::try_new("cc").unwrap());
        save_config(&cfg_path(&dir), &config).unwrap();

        // Act
        append_recipient(&cfg_path(&dir), new_recipient(), false).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        let recipients = loaded.recipients.unwrap();
        assert_eq!(recipients[0].key, Some(crate::domain::RecipientKey::try_new("cc").unwrap()));
        assert_eq!(loaded.default_recipient, Some(crate::domain::RecipientKey::try_new("cc").unwrap()));
    }

    #[test]
    fn test_append_recipient_v1_set_default_true_new_becomes_default() {
        // Arrange
        let dir = TempDir::new().unwrap();
        save_config(&cfg_path(&dir), &v1_config_with_recipient()).unwrap();

        // Act
        append_recipient(&cfg_path(&dir), new_recipient(), true).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        let recipients = loaded.recipients.unwrap();
        assert_eq!(recipients.len(), 2);
        assert_eq!(loaded.default_recipient, Some(crate::domain::RecipientKey::try_new("macrosoft").unwrap()));
    }

    #[test]
    fn test_append_recipient_v1_clears_v1_field() {
        // Arrange
        let dir = TempDir::new().unwrap();
        save_config(&cfg_path(&dir), &v1_config_with_recipient()).unwrap();

        // Act
        append_recipient(&cfg_path(&dir), new_recipient(), false).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        assert!(loaded.recipient.is_none(), "v1 recipient field should be cleared after migration");
    }

    #[test]
    fn test_append_recipient_v1_preserves_recipient_data() {
        // Arrange
        let dir = TempDir::new().unwrap();
        save_config(&cfg_path(&dir), &v1_config_with_recipient()).unwrap();

        // Act
        append_recipient(&cfg_path(&dir), new_recipient(), false).unwrap();

        // Assert
        let loaded = unwrap_loaded(load_config(&cfg_path(&dir)));
        let migrated = &loaded.recipients.unwrap()[0];
        assert_eq!(migrated.name, "Client Corp");
        assert_eq!(migrated.address, vec!["456 Client Ave"]);
        assert_eq!(migrated.company_id, Some("CC-12345".into()));
        assert_eq!(migrated.vat_number, Some("VAT-999".into()));
    }

    #[test]
    fn test_set_default_recipient_unknown_key_returns_error() {
        // Arrange
        let dir = TempDir::new().unwrap();
        save_config(&cfg_path(&dir), &complete_config_v2_two_recipients()).unwrap();

        // Act
        let result = set_default_recipient(&cfg_path(&dir), "nope");

        // Assert
        assert!(matches!(result, Err(AppError::RecipientNotFound { .. })));
    }

    // ── Atomic-write tests (Option A) ──

    /// On a successful save, no leftover temp files should remain in the parent
    /// directory. This proves we used the persist path (rename) rather than the
    /// old truncate-and-write approach, where any temp files would never have
    /// been created in the first place. With NamedTempFile, a leftover only
    /// happens if persist() fails to rename — so absence == clean rename.
    #[test]
    fn test_save_config_leaves_no_temp_files_in_parent() {
        // Arrange
        let dir = TempDir::new().unwrap();

        // Act
        save_config(&cfg_path(&dir), &complete_config()).unwrap();

        // Assert — only the target file should exist in the directory
        let entries: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert_eq!(
            entries,
            vec!["config.yaml".to_string()],
            "Expected only config.yaml, found: {entries:?}"
        );
    }

    /// Atomicity invariant: if a save fails (here, because the parent
    /// directory is read-only and a NamedTempFile cannot be created in it),
    /// the existing config file must remain byte-identical to before the
    /// attempt. The legacy `std::fs::write` truncate-then-write pattern fails
    /// this — it will happily overwrite the existing file even when the
    /// parent dir is read-only because mutating an existing file's contents
    /// does not require write permission on the parent.
    #[test]
    #[cfg(unix)]
    fn test_save_config_failure_leaves_original_byte_identical() {
        // Arrange — write a known-good config, capture its bytes, then make
        // the parent read-only so the next save cannot create a temp file.
        use std::os::unix::fs::PermissionsExt;
        let dir = TempDir::new().unwrap();
        let path = cfg_path(&dir);
        save_config(&path, &complete_config()).unwrap();
        let original_bytes = std::fs::read(&path).unwrap();
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o555)).unwrap();

        // Act — try to save a different config; with atomic writes this errors
        // because NamedTempFile can't be created in the read-only directory.
        let mut different = complete_config();
        different.sender.as_mut().unwrap().name = "Charlie Mutated".into();
        let result = save_config(&path, &different);

        // Assert — original file untouched (atomicity guarantee)
        let after_bytes = std::fs::read(&path).unwrap();
        // Restore permissions before any potential panic so TempDir cleans up.
        std::fs::set_permissions(dir.path(), std::fs::Permissions::from_mode(0o755)).unwrap();
        assert!(result.is_err(), "Expected save to fail with read-only parent");
        assert_eq!(
            after_bytes, original_bytes,
            "Original config must be byte-identical after failed save"
        );
    }

    /// Probabilistic safety net: race two threads on `save_config` and assert
    /// the final file equals one of the inputs verbatim (never a mix).
    ///
    /// Honest caveat: this test passes on the legacy `std::fs::write`
    /// implementation too, because small YAML payloads rarely interleave
    /// inside an `O_TRUNC`+`write` on modern kernels. It is *not* a
    /// deterministic guard against the original truncate-and-write race —
    /// the byte-identical-on-failure test above is what actually pins the
    /// atomicity guarantee. This test is kept as a regression net for any
    /// future change to the persistence path that might introduce a
    /// different race (e.g. a multi-syscall write splitting under heavier
    /// contention).
    #[test]
    fn test_save_config_concurrent_writes_yield_one_valid_config() {
        // Arrange
        let dir = TempDir::new().unwrap();
        let path = cfg_path(&dir);
        // Seed with a non-conflicting config so the file exists before contention.
        save_config(&path, &Config::default()).unwrap();

        let path_a = path.clone();
        let path_b = path.clone();

        let mut config_a = complete_config();
        config_a.sender.as_mut().unwrap().name = "Thread A".into();
        let mut config_b = complete_config();
        config_b.sender.as_mut().unwrap().name = "Thread B".into();

        let yaml_a = serde_yaml::to_string(&config_a).unwrap();
        let yaml_b = serde_yaml::to_string(&config_b).unwrap();

        // Act — race many writes from two threads. See the doc comment above
        // for why this is probabilistic rather than deterministic.
        let t_a = std::thread::spawn(move || {
            for _ in 0..50 {
                save_config(&path_a, &config_a).unwrap();
            }
        });
        let t_b = std::thread::spawn(move || {
            for _ in 0..50 {
                save_config(&path_b, &config_b).unwrap();
            }
        });
        t_a.join().unwrap();
        t_b.join().unwrap();

        // Assert — final bytes match exactly one of the two inputs
        let final_bytes = std::fs::read_to_string(&path).unwrap();
        assert!(
            final_bytes == yaml_a || final_bytes == yaml_b,
            "Concurrent saves produced corrupted output:\n{final_bytes}"
        );
    }
}
