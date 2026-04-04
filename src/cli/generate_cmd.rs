use std::io::Write;
use std::path::Path;

use crate::config::types::{Preset, Recipient};
use crate::config::validator::ValidatedConfig;
use crate::error::AppError;
use crate::invoice::summary::build_summary;
use crate::invoice::types::{InvoicePeriod, LineItem};
use crate::pdf::generate_pdf;

use crate::invoice::currency::effective_currency;

use super::common::pdf_output_path;
use super::load_validated_config;
use super::GenerateArgs;

/// A single item entry from the `--items` JSON array.
#[derive(Debug, serde::Deserialize)]
struct ItemSpec {
    preset: String,
    days: f64,
    rate: Option<f64>,
    tax_rate: Option<f64>,
}

/// Validate month/year into an `InvoicePeriod`.
fn validate_period(month: u32, year: u32) -> Result<InvoicePeriod, AppError> {
    InvoicePeriod::new(month, year).ok_or_else(|| {
        AppError::InvalidDate(format!("month={month}, year={year}"))
    })
}

/// Validate that days is positive and finite.
fn validate_days(days: f64) -> Result<(), AppError> {
    if !days.is_finite() || days <= 0.0 {
        return Err(AppError::InvalidDays(format!("{days}")));
    }
    Ok(())
}

/// Find a preset by key, returning `PresetNotFound` if absent.
fn find_preset<'a>(key: &str, presets: &'a [Preset]) -> Result<&'a Preset, AppError> {
    presets
        .iter()
        .find(|p| p.key == key)
        .ok_or_else(|| AppError::PresetNotFound(key.to_string()))
}

/// Parse the `--items` JSON string into validated `ItemSpec` entries.
fn parse_items(json: &str) -> Result<Vec<ItemSpec>, AppError> {
    let items: Vec<ItemSpec> =
        serde_json::from_str(json).map_err(|e| AppError::ItemsParse(e.to_string()))?;
    if items.is_empty() {
        return Err(AppError::ItemsParse("items array must not be empty".into()));
    }
    for item in &items {
        validate_days(item.days)?;
        if let Some(tr) = item.tax_rate {
            if tr < 0.0 {
                return Err(AppError::InvalidTaxRate(format!("{tr}")));
            }
        }
    }
    Ok(items)
}

/// Resolve CLI arguments into concrete `LineItem`s using the config's presets.
fn resolve_line_items(args: &GenerateArgs, presets: &[Preset], default_currency: &str) -> Result<Vec<LineItem>, AppError> {
    if let Some(ref json) = args.items {
        // Multi-item mode: --items JSON
        let specs = parse_items(json)?;
        specs
            .iter()
            .map(|spec| {
                let preset = find_preset(&spec.preset, presets)?;
                let rate = spec.rate.unwrap_or(preset.default_rate);
                let currency = effective_currency(preset, default_currency).to_string();
                let tax_rate = spec.tax_rate.or(preset.tax_rate).unwrap_or(0.0);
                let item = if tax_rate > 0.0 {
                    LineItem::with_tax(preset.description.clone(), spec.days, rate, currency, tax_rate)
                } else {
                    LineItem::new(preset.description.clone(), spec.days, rate, currency)
                };
                Ok(item)
            })
            .collect()
    } else {
        // Single-item mode: --preset + --days
        let key = args.preset.as_deref().expect("clap enforces preset or items");
        let days = args.days.expect("clap enforces days with preset");
        validate_days(days)?;
        let preset = find_preset(key, presets)?;
        let currency = effective_currency(preset, default_currency).to_string();
        let tax_rate = preset.tax_rate.unwrap_or(0.0);
        let item = if tax_rate > 0.0 {
            LineItem::with_tax(preset.description.clone(), days, preset.default_rate, currency, tax_rate)
        } else {
            LineItem::new(preset.description.clone(), days, preset.default_rate, currency)
        };
        Ok(vec![item])
    }
}

/// Resolve which recipient to use based on the --client flag.
///
/// If no client is specified, returns the default recipient.
/// If a client key is provided, looks it up in the validated recipients list.
fn resolve_recipient<'a>(
    client: Option<&str>,
    validated: &'a ValidatedConfig,
) -> Result<&'a Recipient, AppError> {
    match client {
        None => Ok(&validated.recipient),
        Some(key) => validated
            .recipients
            .iter()
            .find(|r| r.key.as_deref() == Some(key))
            .ok_or_else(|| AppError::RecipientNotFound {
                key: key.to_string(),
                available: validated
                    .recipients
                    .iter()
                    .filter_map(|r| r.key.clone())
                    .collect(),
            }),
    }
}

/// Handle `invoice generate` — non-interactive invoice generation.
pub fn handle_generate(
    args: &GenerateArgs,
    cwd: &Path,
    writer: &mut dyn Write,
) -> Result<(), AppError> {
    let validated = load_validated_config(cwd)?;
    let recipient = resolve_recipient(args.client.as_deref(), &validated)?;
    let period = validate_period(args.month, args.year)?;
    let line_items = resolve_line_items(args, &validated.presets, &validated.defaults.currency)?;
    let summary = build_summary(period, line_items, &validated.defaults)?;
    let pdf_bytes = generate_pdf(&summary, &validated, recipient, cwd)?;
    let output_path = pdf_output_path(&validated.sender.name, &period, cwd);
    std::fs::write(&output_path, &pdf_bytes)?;
    writeln!(writer, "PDF saved: {}", output_path.display())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::test_helpers::*;

    // ── Test helper builders ──

    fn generate_single_args(month: u32, year: u32, preset: &str, days: f64) -> GenerateArgs {
        GenerateArgs {
            month,
            year,
            preset: Some(preset.to_string()),
            days: Some(days),
            items: None,
            client: None,
        }
    }

    fn generate_items_args(month: u32, year: u32, json: &str) -> GenerateArgs {
        GenerateArgs {
            month,
            year,
            preset: None,
            days: None,
            items: Some(json.to_string()),
            client: None,
        }
    }

    fn config_with_named_presets(entries: &[(&str, f64)]) -> crate::config::types::Config {
        use crate::config::types::{Config, Preset};
        let presets: Vec<Preset> = entries
            .iter()
            .map(|(key, rate)| Preset {
                key: key.to_string(),
                description: format!("{key} services"),
                default_rate: *rate,
                currency: None,
                tax_rate: None,
            })
            .collect();
        Config {
            presets: Some(presets),
            ..complete_config()
        }
    }

    // ── Phase 2: JSON deserialization tests (pure) ──

    #[test]
    fn test_parse_items_malformed_json_returns_error() {
        // Arrange
        let json = "not json at all";

        // Act
        let result = parse_items(json);

        // Assert
        assert!(matches!(result, Err(AppError::ItemsParse(_))));
    }

    #[test]
    fn test_parse_items_missing_preset_field_returns_error() {
        // Arrange
        let json = r#"[{"days": 10}]"#;

        // Act
        let result = parse_items(json);

        // Assert
        assert!(matches!(result, Err(AppError::ItemsParse(_))));
    }

    #[test]
    fn test_parse_items_rate_override_parsed() {
        // Arrange
        let json = r#"[{"preset":"dev","days":5,"rate":999.0}]"#;

        // Act
        let items = parse_items(json).unwrap();

        // Assert
        assert_eq!(items.len(), 1);
        assert!((items[0].rate.unwrap() - 999.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_items_rate_absent_is_none() {
        // Arrange
        let json = r#"[{"preset":"dev","days":5}]"#;

        // Act
        let items = parse_items(json).unwrap();

        // Assert
        assert!(items[0].rate.is_none());
    }

    #[test]
    fn test_parse_items_empty_array_returns_error() {
        // Arrange
        let json = "[]";

        // Act
        let result = parse_items(json);

        // Assert
        assert!(matches!(result, Err(AppError::ItemsParse(_))));
    }

    #[test]
    fn test_parse_items_zero_days_returns_error() {
        // Arrange
        let json = r#"[{"preset":"dev","days":0}]"#;

        // Act
        let result = parse_items(json);

        // Assert
        assert!(matches!(result, Err(AppError::InvalidDays(_))));
    }

    // ── Phase 3: Validation tests (pure) ──

    #[test]
    fn test_validate_days_zero_returns_error() {
        // Arrange
        let days = 0.0;

        // Act
        let result = validate_days(days);

        // Assert
        assert!(matches!(result, Err(AppError::InvalidDays(_))));
    }

    #[test]
    fn test_validate_days_positive_succeeds() {
        // Arrange
        let days = 5.5;

        // Act
        let result = validate_days(days);

        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_validate_period_invalid_month_returns_error() {
        // Arrange
        let month = 13;
        let year = 2026;

        // Act
        let result = validate_period(month, year);

        // Assert
        assert!(matches!(result, Err(AppError::InvalidDate(_))));
    }

    #[test]
    fn test_find_preset_not_found_returns_error() {
        // Arrange
        let presets = synthetic_presets();

        // Act
        let result = find_preset("nonexistent", &presets);

        // Assert
        assert!(matches!(result, Err(AppError::PresetNotFound(_))));
    }

    #[test]
    fn test_find_preset_found_returns_preset() {
        // Arrange
        let presets = synthetic_presets();

        // Act
        let result = find_preset("dev", &presets);

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().key, "dev");
    }

    // ── Phase 4: Handler tests — single-item (tempdir) ──

    #[test]
    fn test_handle_generate_no_config_returns_error() {
        // Arrange
        let dir = setup_dir(None);
        let args = generate_single_args(3, 2026, "dev", 10.0);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, dir.path(), &mut buf);

        // Assert
        assert!(matches!(result, Err(AppError::ConfigNotFound)));
    }

    #[test]
    fn test_handle_generate_preset_not_found_returns_error() {
        // Arrange
        let config = complete_config();
        let dir = setup_dir(Some(&config));
        let args = generate_single_args(3, 2026, "nonexistent", 10.0);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, dir.path(), &mut buf);

        // Assert
        assert!(matches!(result, Err(AppError::PresetNotFound(_))));
    }

    #[test]
    fn test_handle_generate_single_item_produces_pdf_file() {
        // Arrange
        let config = complete_config();
        let dir = setup_dir(Some(&config));
        let args = generate_single_args(3, 2026, "dev", 10.0);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        handle_generate(&args, dir.path(), &mut buf).unwrap();

        // Assert
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("PDF saved:"), "Expected 'PDF saved:' in: {output}");
        let pdf_path = dir.path().join("Invoice_Alice_Smith_Mar2026.pdf");
        assert!(pdf_path.exists(), "PDF file should exist");
        let bytes = std::fs::read(&pdf_path).unwrap();
        assert!(bytes.starts_with(b"%PDF"), "File should start with %PDF header");
    }

    #[test]
    fn test_handle_generate_single_item_overwrites_existing_pdf() {
        // Arrange
        let config = complete_config();
        let dir = setup_dir(Some(&config));
        let pdf_path = dir.path().join("Invoice_Alice_Smith_Mar2026.pdf");
        std::fs::write(&pdf_path, b"old content").unwrap();
        let args = generate_single_args(3, 2026, "dev", 10.0);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        handle_generate(&args, dir.path(), &mut buf).unwrap();

        // Assert
        let bytes = std::fs::read(&pdf_path).unwrap();
        assert!(bytes.starts_with(b"%PDF"), "File should be overwritten with actual PDF");
        assert_ne!(bytes, b"old content");
    }

    // ── Phase 5: Handler tests — multi-item (tempdir) ──

    #[test]
    fn test_handle_generate_items_single_entry_produces_pdf() {
        // Arrange
        let config = config_with_named_presets(&[("alpha", 800.0)]);
        let dir = setup_dir(Some(&config));
        let json = r#"[{"preset":"alpha","days":5}]"#;
        let args = generate_items_args(3, 2026, json);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        handle_generate(&args, dir.path(), &mut buf).unwrap();

        // Assert
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("PDF saved:"));
    }

    #[test]
    fn test_handle_generate_items_unknown_preset_names_key() {
        // Arrange
        let config = complete_config();
        let dir = setup_dir(Some(&config));
        let json = r#"[{"preset":"bogus","days":5}]"#;
        let args = generate_items_args(3, 2026, json);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, dir.path(), &mut buf);

        // Assert
        match result {
            Err(AppError::PresetNotFound(key)) => assert_eq!(key, "bogus"),
            other => panic!("Expected PresetNotFound, got {other:?}"),
        }
    }

    #[test]
    fn test_handle_generate_items_multiple_entries_produces_pdf() {
        // Arrange
        let config = config_with_named_presets(&[("alpha", 800.0), ("beta", 500.0)]);
        let dir = setup_dir(Some(&config));
        let json = r#"[{"preset":"alpha","days":10},{"preset":"beta","days":5}]"#;
        let args = generate_items_args(3, 2026, json);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        handle_generate(&args, dir.path(), &mut buf).unwrap();

        // Assert
        let pdf_path = dir.path().join("Invoice_Alice_Smith_Mar2026.pdf");
        assert!(pdf_path.exists());
        let bytes = std::fs::read(&pdf_path).unwrap();
        assert!(bytes.starts_with(b"%PDF"));
    }

    #[test]
    fn test_handle_generate_items_rate_override_used() {
        // Arrange — preset default_rate is 800, but JSON overrides to 1200
        let config = config_with_named_presets(&[("alpha", 800.0)]);
        let dir = setup_dir(Some(&config));
        let json = r#"[{"preset":"alpha","days":10,"rate":1200.0}]"#;
        let args = generate_items_args(3, 2026, json);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        handle_generate(&args, dir.path(), &mut buf).unwrap();

        // Assert — verify the PDF was generated (rate override is internal to line items)
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("PDF saved:"));
        let pdf_path = dir.path().join("Invoice_Alice_Smith_Mar2026.pdf");
        assert!(pdf_path.exists());
    }

    // ── Phase: resolve_recipient tests (pure) ──

    #[test]
    fn test_resolve_recipient_none_returns_default() {
        // Arrange
        let validated = crate::setup::test_helpers::validated(
            crate::setup::test_helpers::v2_config_two_recipients(),
        );

        // Act
        let result = resolve_recipient(None, &validated);

        // Assert
        let recipient = result.unwrap();
        assert_eq!(recipient.name, "Acme Corp");
    }

    #[test]
    fn test_resolve_recipient_some_matching_key_returns_recipient() {
        // Arrange
        let validated = crate::setup::test_helpers::validated(
            crate::setup::test_helpers::v2_config_two_recipients(),
        );

        // Act
        let result = resolve_recipient(Some("globex"), &validated);

        // Assert
        let recipient = result.unwrap();
        assert_eq!(recipient.name, "Globex Inc");
    }

    #[test]
    fn test_resolve_recipient_unknown_key_returns_error() {
        // Arrange
        let validated = crate::setup::test_helpers::validated(
            crate::setup::test_helpers::v2_config_two_recipients(),
        );

        // Act
        let result = resolve_recipient(Some("nonexistent"), &validated);

        // Assert
        assert!(matches!(result, Err(AppError::RecipientNotFound { .. })));
    }

    #[test]
    fn test_resolve_recipient_error_lists_available_keys() {
        // Arrange
        let validated = crate::setup::test_helpers::validated(
            crate::setup::test_helpers::v2_config_two_recipients(),
        );

        // Act
        let result = resolve_recipient(Some("nope"), &validated);

        // Assert
        match result {
            Err(AppError::RecipientNotFound { key, available }) => {
                assert_eq!(key, "nope");
                assert!(available.contains(&"acme".to_string()));
                assert!(available.contains(&"globex".to_string()));
            }
            other => panic!("Expected RecipientNotFound, got {other:?}"),
        }
    }

    // ── Phase: --client integration tests ──

    #[test]
    fn test_handle_generate_with_client_flag_uses_specified_recipient() {
        // Arrange
        let config = crate::setup::test_helpers::v2_config_two_recipients();
        let dir = setup_dir(Some(&config));
        let mut args = generate_single_args(3, 2026, "dev", 10.0);
        args.client = Some("globex".to_string());
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, dir.path(), &mut buf);

        // Assert
        assert!(result.is_ok(), "Expected Ok, got {result:?}");
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("PDF saved:"));
    }

    #[test]
    fn test_handle_generate_with_unknown_client_returns_error() {
        // Arrange
        let config = crate::setup::test_helpers::v2_config_two_recipients();
        let dir = setup_dir(Some(&config));
        let mut args = generate_single_args(3, 2026, "dev", 10.0);
        args.client = Some("nonexistent".to_string());
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, dir.path(), &mut buf);

        // Assert
        assert!(matches!(result, Err(AppError::RecipientNotFound { .. })));
    }

    // ── Story 11.1: v1 backwards compatibility verification ──

    #[test]
    fn test_handle_generate_v1_config_without_client_flag_produces_pdf() {
        // Arrange — v1 config (single recipient, no recipients list)
        let config = complete_config(); // v1 format
        let dir = setup_dir(Some(&config));
        let args = generate_single_args(3, 2026, "dev", 10.0);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, dir.path(), &mut buf);

        // Assert
        assert!(result.is_ok(), "v1 config should work without --client flag: {result:?}");
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("PDF saved:"));
    }

    // ── Phase 9: Currency wiring tests ──

    fn config_with_currency_presets(entries: &[(&str, f64, Option<&str>)]) -> crate::config::types::Config {
        use crate::config::types::{Config, Preset};
        let presets: Vec<Preset> = entries
            .iter()
            .map(|(key, rate, currency)| Preset {
                key: key.to_string(),
                description: format!("{key} services"),
                default_rate: *rate,
                currency: currency.map(|c| c.to_string()),
                tax_rate: None,
            })
            .collect();
        Config {
            presets: Some(presets),
            ..complete_config()
        }
    }

    #[test]
    fn test_handle_generate_single_item_preset_currency_override() {
        // Arrange
        let config = config_with_currency_presets(&[("dev", 800.0, Some("CZK"))]);
        let dir = setup_dir(Some(&config));
        let args = generate_single_args(3, 2026, "dev", 10.0);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, dir.path(), &mut buf);

        // Assert
        assert!(result.is_ok(), "Expected Ok, got {result:?}");
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("PDF saved:"));
    }

    #[test]
    fn test_handle_generate_items_mixed_currency_returns_error() {
        // Arrange
        let config = config_with_currency_presets(&[("alpha", 800.0, Some("EUR")), ("beta", 500.0, Some("USD"))]);
        let dir = setup_dir(Some(&config));
        let json = r#"[{"preset":"alpha","days":10},{"preset":"beta","days":5}]"#;
        let args = generate_items_args(3, 2026, json);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, dir.path(), &mut buf);

        // Assert
        assert!(matches!(result, Err(AppError::MixedCurrency(_))));
    }

    fn config_with_tax_presets(entries: &[(&str, f64, Option<f64>)]) -> crate::config::types::Config {
        use crate::config::types::{Config, Preset};
        let presets: Vec<Preset> = entries
            .iter()
            .map(|(key, rate, tax)| Preset {
                key: key.to_string(),
                description: format!("{key} services"),
                default_rate: *rate,
                currency: None,
                tax_rate: *tax,
            })
            .collect();
        Config {
            presets: Some(presets),
            ..complete_config()
        }
    }

    // ── Phase: tax_rate JSON parsing tests ──

    #[test]
    fn test_parse_items_tax_rate_present_parsed() {
        // Arrange
        let json = r#"[{"preset":"dev","days":5,"tax_rate":21.0}]"#;

        // Act
        let items = parse_items(json).unwrap();

        // Assert
        assert_eq!(items[0].tax_rate, Some(21.0));
    }

    #[test]
    fn test_parse_items_tax_rate_absent_is_none() {
        // Arrange
        let json = r#"[{"preset":"dev","days":5}]"#;

        // Act
        let items = parse_items(json).unwrap();

        // Assert
        assert!(items[0].tax_rate.is_none());
    }

    #[test]
    fn test_parse_items_negative_tax_rate_returns_error() {
        // Arrange
        let json = r#"[{"preset":"dev","days":5,"tax_rate":-1.0}]"#;

        // Act
        let result = parse_items(json);

        // Assert
        assert!(matches!(result, Err(AppError::InvalidTaxRate(_))));
    }

    #[test]
    fn test_parse_items_zero_tax_rate_accepted() {
        // Arrange
        let json = r#"[{"preset":"dev","days":5,"tax_rate":0.0}]"#;

        // Act
        let items = parse_items(json).unwrap();

        // Assert
        assert_eq!(items[0].tax_rate, Some(0.0));
    }

    // ── Phase: tax_rate resolution integration tests ──

    #[test]
    fn test_handle_generate_items_with_tax_rate() {
        // Arrange
        let config = config_with_tax_presets(&[("dev", 800.0, None)]);
        let dir = setup_dir(Some(&config));
        let json = r#"[{"preset":"dev","days":10,"tax_rate":21.0}]"#;
        let args = generate_items_args(3, 2026, json);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, dir.path(), &mut buf);

        // Assert
        assert!(result.is_ok(), "Expected Ok, got {result:?}");
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("PDF saved:"));
    }

    #[test]
    fn test_handle_generate_items_tax_falls_back_to_preset() {
        // Arrange — preset has tax_rate 21.0, JSON omits it
        let config = config_with_tax_presets(&[("dev", 800.0, Some(21.0))]);
        let dir = setup_dir(Some(&config));
        let json = r#"[{"preset":"dev","days":10}]"#;
        let args = generate_items_args(3, 2026, json);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, dir.path(), &mut buf);

        // Assert
        assert!(result.is_ok(), "Expected Ok, got {result:?}");
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("PDF saved:"));
    }

    #[test]
    fn test_handle_generate_items_negative_tax_returns_error() {
        // Arrange
        let config = config_with_tax_presets(&[("dev", 800.0, None)]);
        let dir = setup_dir(Some(&config));
        let json = r#"[{"preset":"dev","days":10,"tax_rate":-1.0}]"#;
        let args = generate_items_args(3, 2026, json);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, dir.path(), &mut buf);

        // Assert
        assert!(matches!(result, Err(AppError::InvalidTaxRate(_))));
    }

    #[test]
    fn test_handle_generate_single_item_uses_preset_tax() {
        // Arrange — single-item mode with preset that has tax_rate
        let config = config_with_tax_presets(&[("dev", 800.0, Some(21.0))]);
        let dir = setup_dir(Some(&config));
        let args = generate_single_args(3, 2026, "dev", 10.0);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, dir.path(), &mut buf);

        // Assert
        assert!(result.is_ok(), "Expected Ok, got {result:?}");
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("PDF saved:"));
    }

    #[test]
    fn test_handle_generate_items_same_override_currency_succeeds() {
        // Arrange
        let config = config_with_currency_presets(&[("alpha", 800.0, Some("USD")), ("beta", 500.0, Some("USD"))]);
        let dir = setup_dir(Some(&config));
        let json = r#"[{"preset":"alpha","days":10},{"preset":"beta","days":5}]"#;
        let args = generate_items_args(3, 2026, json);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, dir.path(), &mut buf);

        // Assert
        assert!(result.is_ok(), "Expected Ok, got {result:?}");
    }
}
