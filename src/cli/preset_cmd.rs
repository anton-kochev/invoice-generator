use std::io::Write;
use std::path::Path;

use crate::config::ConfigError;
use crate::config::loader::{load_config, LoadResult};
use crate::config::types::Preset;
use crate::config::validator::ValidatedConfig;
use crate::config::writer::remove_preset;
use crate::domain::Currency;
use crate::error::AppError;
use crate::invoice::currency::effective_currency;
use crate::setup::prompter::Prompter;

/// Format presets as a table string with columns: Key, Description, Default Rate, Currency.
///
/// Dynamic column widths based on data (minimum widths: Key=3, Description=11, Rate=12).
/// Rate is formatted with 2 decimal places and right-aligned.
pub fn format_preset_table(presets: &[Preset], default_currency: Currency) -> String {
    let min_key = 3;
    let min_desc = 11;
    let min_rate = 12;

    let key_w = presets
        .iter()
        .map(|p| p.key.as_str().len())
        .max()
        .unwrap_or(0)
        .max(min_key);
    let desc_w = presets
        .iter()
        .map(|p| p.description.len())
        .max()
        .unwrap_or(0)
        .max(min_desc);
    let rate_w = presets
        .iter()
        .map(|p| format!("{:.2}", p.default_rate).len())
        .max()
        .unwrap_or(0)
        .max(min_rate);
    let curr_w = presets
        .iter()
        .map(|p| effective_currency(p, default_currency).code().len())
        .max()
        .unwrap_or(0)
        .max(8); // "Currency".len()

    let mut out = String::new();

    // Header
    out.push_str(&format!(
        "{:<key_w$}  {:<desc_w$}  {:>rate_w$}  {:<curr_w$}\n",
        "Key", "Description", "Default Rate", "Currency",
    ));

    // Separator
    out.push_str(&format!(
        "{}  {}  {}  {}\n",
        "-".repeat(key_w),
        "-".repeat(desc_w),
        "-".repeat(rate_w),
        "-".repeat(curr_w),
    ));

    // Data rows
    for p in presets {
        let curr = effective_currency(p, default_currency);
        out.push_str(&format!(
            "{:<key_w$}  {:<desc_w$}  {:>rate_w$.2}  {:<curr_w$}\n",
            p.key, p.description, p.default_rate, curr,
        ));
    }

    out
}

/// Handle `invoice preset list` — print formatted preset table.
pub fn handle_preset_list(
    validated: &ValidatedConfig,
    writer: &mut dyn Write,
) -> Result<(), AppError> {
    let table = format_preset_table(&validated.presets, validated.defaults.currency);
    writer
        .write_all(table.as_bytes())
        .map_err(crate::pdf::PdfError::Write)?;
    Ok(())
}

/// Handle `invoice preset delete <key>` — confirm and remove preset.
pub fn handle_preset_delete(
    prompter: &dyn Prompter,
    config_path: &Path,
    key: &str,
    writer: &mut dyn Write,
) -> Result<(), AppError> {
    // Load config to get preset details for confirmation
    let config = match load_config(config_path)? {
        LoadResult::Loaded(c) => *c,
        LoadResult::NotFound => return Err(ConfigError::NotFound.into()),
    };

    let presets = config.presets.as_deref().unwrap_or_default();

    // Find the preset first to get its description
    let preset = presets
        .iter()
        .find(|p| p.key.as_str() == key)
        .ok_or_else(|| ConfigError::PresetNotFound(key.to_string()))?;

    // Guard: cannot delete the last preset
    if presets.len() <= 1 {
        return Err(ConfigError::LastPreset.into());
    }

    let prompt = format!("Delete preset \"{}\" ({})?", preset.key, preset.description);

    if !prompter.confirm(&prompt, false)? {
        return Ok(());
    }

    remove_preset(config_path, key)?;
    writeln!(
        writer,
        "✓ Preset \"{}\" deleted from {}",
        key,
        config_path.display()
    )
    .map_err(crate::pdf::PdfError::Write)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::Preset;

    fn dev_preset() -> Preset {
        Preset {
            key: crate::domain::PresetKey::try_new("dev").unwrap(),
            description: "Development Services".to_string(),
            default_rate: 100.0,
            currency: None,
            tax_rate: None,
        }
    }

    #[test]
    fn test_format_preset_table_contains_header_row() {
        // Arrange
        let presets = vec![dev_preset()];

        // Act
        let output = format_preset_table(&presets, Currency::Eur);

        // Assert
        assert!(output.contains("Key"), "Missing 'Key' header");
        assert!(output.contains("Description"), "Missing 'Description' header");
        assert!(output.contains("Default Rate"), "Missing 'Default Rate' header");
        assert!(output.contains("Currency"), "Missing 'Currency' header");
    }

    #[test]
    fn test_format_preset_table_contains_preset_data() {
        // Arrange
        let presets = vec![dev_preset()];

        // Act
        let output = format_preset_table(&presets, Currency::Eur);

        // Assert
        assert!(output.contains("dev"), "Missing key 'dev'");
        assert!(
            output.contains("Development Services"),
            "Missing description"
        );
        assert!(output.contains("100.00"), "Missing rate '100.00'");
        assert!(output.contains("EUR"), "Missing currency 'EUR'");
    }

    #[test]
    fn test_format_preset_table_contains_currency() {
        // Arrange
        let presets = vec![dev_preset()];

        // Act
        let output = format_preset_table(&presets, Currency::Usd);

        // Assert
        assert!(output.contains("USD"), "Missing currency 'USD'");
    }

    #[test]
    fn test_format_preset_table_multiple_presets_shows_all() {
        // Arrange
        let presets = vec![
            dev_preset(),
            Preset {
                key: crate::domain::PresetKey::try_new("design").unwrap(),
                description: "Design work".to_string(),
                default_rate: 80.0,
                currency: None,
                tax_rate: None,
            },
        ];

        // Act
        let output = format_preset_table(&presets, Currency::Eur);

        // Assert
        assert!(output.contains("dev"), "Missing 'dev'");
        assert!(output.contains("design"), "Missing 'design'");
    }

    #[test]
    fn test_format_preset_table_empty_presets_shows_only_header() {
        // Arrange
        let presets: Vec<Preset> = vec![];

        // Act
        let output = format_preset_table(&presets, Currency::Eur);

        // Assert
        assert!(output.contains("Key"), "Missing header in empty table");
        assert!(!output.contains("dev"), "Should not contain preset data");
    }

    #[test]
    fn test_format_preset_table_long_description_not_truncated() {
        // Arrange
        let long_desc = "A".repeat(80);
        let presets = vec![Preset {
            key: crate::domain::PresetKey::try_new("lng").unwrap(),
            description: long_desc.clone(),
            default_rate: 50.0,
            currency: None,
            tax_rate: None,
        }];

        // Act
        let output = format_preset_table(&presets, Currency::Eur);

        // Assert
        assert!(
            output.contains(&long_desc),
            "80-char description should appear in full"
        );
    }

    // ── Handler tests ──

    use crate::config::ConfigError;
    use crate::config::loader::{load_config, LoadResult};
    use crate::config::validator::ValidationOutcome;
    use crate::error::AppError;
    use crate::setup::mock_prompter::{MockPrompter, MockResponse};
    use crate::setup::test_helpers::*;

    #[test]
    fn test_handle_preset_list_returns_formatted_table() {
        // Arrange
        let config = complete_config();
        let validated = match config.validate().unwrap() {
            ValidationOutcome::Complete(v) => v,
            _ => panic!("Expected Complete"),
        };
        let mut buf: Vec<u8> = Vec::new();

        // Act
        handle_preset_list(&validated, &mut buf).unwrap();

        // Assert
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("dev"), "Missing key 'dev'");
        assert!(
            output.contains("Development Services"),
            "Missing description"
        );
        assert!(
            output.contains(validated.defaults.currency.code()),
            "Missing currency"
        );
    }

    #[test]
    fn test_handle_preset_list_no_config_returns_error() {
        // Arrange
        let dir = setup_dir(None);

        // Act
        let result = crate::cli::load_validated_config(&cfg_path(&dir));

        // Assert
        assert!(matches!(result, Err(AppError::Config(ConfigError::NotFound))));
    }

    #[test]
    fn test_handle_preset_delete_confirmed_removes_preset() {
        // Arrange
        let config = config_with_two_presets();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![MockResponse::Confirm(true)]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_preset_delete(&prompter, &cfg_path(&dir), "design", &mut buf);

        // Assert
        assert!(result.is_ok());
        let loaded = match load_config(&cfg_path(&dir)).unwrap() {
            LoadResult::Loaded(c) => *c,
            _ => panic!("Expected Loaded"),
        };
        let presets = loaded.presets.unwrap();
        assert_eq!(presets.len(), 1);
        assert_eq!(presets[0].key.as_str(), "dev");
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("deleted"), "Expected 'deleted' in output");
    }

    #[test]
    fn test_handle_preset_delete_user_declines() {
        // Arrange
        let config = config_with_two_presets();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![MockResponse::Confirm(false)]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_preset_delete(&prompter, &cfg_path(&dir), "design", &mut buf);

        // Assert
        assert!(result.is_ok());
        let loaded = match load_config(&cfg_path(&dir)).unwrap() {
            LoadResult::Loaded(c) => *c,
            _ => panic!("Expected Loaded"),
        };
        let presets = loaded.presets.unwrap();
        assert_eq!(presets.len(), 2);
        let output = String::from_utf8(buf).unwrap();
        assert!(output.is_empty(), "Expected no output on decline");
    }

    #[test]
    fn test_handle_preset_delete_unknown_key_returns_error() {
        // Arrange
        let config = config_with_two_presets();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_preset_delete(&prompter, &cfg_path(&dir), "nope", &mut buf);

        // Assert
        assert!(matches!(result, Err(AppError::Config(ConfigError::PresetNotFound(_)))));
        prompter.assert_exhausted();
    }

    #[test]
    fn test_handle_preset_delete_last_preset_refused() {
        // Arrange
        let config = complete_config();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_preset_delete(&prompter, &cfg_path(&dir), "dev", &mut buf);

        // Assert
        assert!(matches!(result, Err(AppError::Config(ConfigError::LastPreset))));
        prompter.assert_exhausted();
    }

    #[test]
    fn test_handle_preset_delete_confirmation_includes_key_and_description() {
        // Arrange
        let config = config_with_two_presets();
        let dir = setup_dir(Some(&config));
        let prompter = MockPrompter::new(vec![MockResponse::Confirm(true)]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        handle_preset_delete(&prompter, &cfg_path(&dir), "design", &mut buf).unwrap();

        // Assert
        let prompts = prompter.prompts.borrow();
        assert_eq!(prompts.len(), 1);
        assert!(
            prompts[0].contains("design"),
            "Expected 'design' in prompt: {}",
            prompts[0]
        );
        assert!(
            prompts[0].contains("Design Work"),
            "Expected 'Design Work' in prompt: {}",
            prompts[0]
        );
    }

    #[test]
    fn test_handle_preset_delete_no_config_returns_error() {
        // Arrange
        let dir = setup_dir(None);
        let prompter = MockPrompter::new(vec![]);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_preset_delete(&prompter, &cfg_path(&dir), "dev", &mut buf);

        // Assert
        assert!(matches!(result, Err(AppError::Config(ConfigError::NotFound))));
        prompter.assert_exhausted();
    }

    // ── Phase 10: Per-preset currency in table ──

    #[test]
    fn test_format_preset_table_shows_per_preset_currency() {
        // Arrange — UAH replaces the old CZK fixture (closed Currency enum).
        let presets = vec![Preset {
            key: crate::domain::PresetKey::try_new("dev").unwrap(),
            description: "Development".into(),
            default_rate: 800.0,
            currency: Some(Currency::Uah),
            tax_rate: None,
        }];

        // Act
        let table = format_preset_table(&presets, Currency::Eur);

        // Assert
        assert!(table.contains("UAH"), "Expected 'UAH' in table:\n{table}");
        // Should NOT show the default EUR for this preset
        let data_lines: Vec<&str> = table.lines().skip(2).collect(); // skip header + separator
        let dev_line = data_lines.iter().find(|l| l.contains("dev")).unwrap();
        assert!(dev_line.contains("UAH"), "Dev row should show UAH: {dev_line}");
    }

    #[test]
    fn test_format_preset_table_shows_default_when_none() {
        // Arrange
        let presets = vec![Preset {
            key: crate::domain::PresetKey::try_new("dev").unwrap(),
            description: "Development".into(),
            default_rate: 800.0,
            currency: None,
            tax_rate: None,
        }];

        // Act
        let table = format_preset_table(&presets, Currency::Eur);

        // Assert
        let data_lines: Vec<&str> = table.lines().skip(2).collect();
        let dev_line = data_lines.iter().find(|l| l.contains("dev")).unwrap();
        assert!(dev_line.contains("EUR"), "Dev row should show EUR: {dev_line}");
    }

    #[test]
    fn test_format_preset_table_mixed_currencies() {
        // Arrange
        let presets = vec![
            Preset {
                key: crate::domain::PresetKey::try_new("dev").unwrap(),
                description: "Development".into(),
                default_rate: 800.0,
                currency: Some(Currency::Usd),
                tax_rate: None,
            },
            Preset {
                key: crate::domain::PresetKey::try_new("qa").unwrap(),
                description: "QA work".into(),
                default_rate: 600.0,
                currency: None,
                tax_rate: None,
            },
        ];

        // Act
        let table = format_preset_table(&presets, Currency::Eur);

        // Assert
        let data_lines: Vec<&str> = table.lines().skip(2).collect();
        let dev_line = data_lines.iter().find(|l| l.contains("dev")).unwrap();
        let qa_line = data_lines.iter().find(|l| l.contains("qa")).unwrap();
        assert!(dev_line.contains("USD"), "Dev row should show USD: {dev_line}");
        assert!(qa_line.contains("EUR"), "QA row should show EUR: {qa_line}");
    }
}
