use std::io::Write;
use std::path::Path;
use std::str::FromStr;

use crate::config::ConfigError;
use crate::config::types::Preset;
use crate::config::validator::{ValidatedConfig, ValidatedRecipient, ValidatedSender};
use crate::domain::{BillingUnit, Currency};
use crate::error::AppError;
use crate::invoice::InvoiceError;
use crate::invoice::summary::build_summary;
use crate::invoice::types::{InvoicePeriod, LineItem};
use crate::locale::Locale;
use crate::pdf::registry::{Template, TemplateRegistry};
use crate::pdf::{PdfError, generate_pdf};

use crate::invoice::currency::effective_currency;

use super::GenerateArgs;
use super::common::pdf_output_path;
use super::load_validated_config;

/// A single item entry from the `--items` JSON array.
///
/// `quantity` is expressed in the referenced preset's billing unit. The `days`
/// alias keeps pre-billing-unit payloads working; there is deliberately no
/// `hours` alias, because the preset — not the payload — decides the unit, so
/// an `hours` key would just be `quantity` under a misleading name.
#[derive(Debug, serde::Deserialize)]
struct ItemSpec {
    preset: String,
    #[serde(alias = "days")]
    quantity: f64,
    rate: Option<f64>,
    tax_rate: Option<f64>,
}

/// Validate month/year into an `InvoicePeriod`.
fn validate_period(month: u32, year: u32) -> Result<InvoicePeriod, InvoiceError> {
    InvoicePeriod::new(month, year)
        .ok_or_else(|| InvoiceError::InvalidDate(format!("month={month}, year={year}")))
}

/// Validate that a quantity is positive and finite.
fn validate_quantity(quantity: f64) -> Result<(), InvoiceError> {
    if !quantity.is_finite() || quantity <= 0.0 {
        return Err(InvoiceError::InvalidQuantity(format!("{quantity}")));
    }
    Ok(())
}

/// Pick the quantity for single-item mode out of the three amount flags.
///
/// `--quantity` is unit-agnostic and simply takes the preset's unit. `--days`
/// and `--hours` additionally *assert* the unit: using one against a preset
/// billed in the other unit is rejected rather than silently billed, since the
/// resulting figure would be wrong on a money document with no other signal.
fn resolve_quantity(args: &GenerateArgs, preset: &Preset) -> Result<f64, InvoiceError> {
    let mismatch = |flag: &'static str| InvoiceError::UnitMismatch {
        flag,
        preset: preset.key.as_str().to_string(),
        unit: preset.unit,
    };
    match (args.quantity, args.days, args.hours) {
        (Some(quantity), _, _) => Ok(quantity),
        (_, Some(days), _) if preset.unit == BillingUnit::Day => Ok(days),
        (_, Some(_), _) => Err(mismatch("--days")),
        (_, _, Some(hours)) if preset.unit == BillingUnit::Hour => Ok(hours),
        (_, _, Some(_)) => Err(mismatch("--hours")),
        // Unreachable while clap's `amount` group holds; surfaced as an error
        // rather than a panic so a wiring regression stays a usage message.
        (None, None, None) => Err(InvoiceError::MissingQuantity),
    }
}

/// Find a preset by key, returning `PresetNotFound` if absent.
fn find_preset<'a>(key: &str, presets: &'a [Preset]) -> Result<&'a Preset, ConfigError> {
    presets
        .iter()
        .find(|p| p.key.as_str() == key)
        .ok_or_else(|| ConfigError::PresetNotFound(key.to_string()))
}

/// Parse the `--items` JSON string into validated `ItemSpec` entries.
fn parse_items(json: &str) -> Result<Vec<ItemSpec>, InvoiceError> {
    // serde_json::Error → InvoiceError::ItemsParse via #[from].
    let items: Vec<ItemSpec> = serde_json::from_str(json)?;
    if items.is_empty() {
        return Err(InvoiceError::EmptyItems);
    }
    for item in &items {
        validate_quantity(item.quantity)?;
        if let Some(tr) = item.tax_rate
            && tr < 0.0
        {
            return Err(InvoiceError::InvalidTaxRate(format!("{tr}")));
        }
    }
    Ok(items)
}

/// Resolve CLI arguments into concrete `LineItem`s using the config's presets.
fn resolve_line_items(
    args: &GenerateArgs,
    presets: &[Preset],
    default_currency: Currency,
) -> Result<Vec<LineItem>, AppError> {
    if let Some(ref json) = args.items {
        // Multi-item mode: --items JSON
        let specs = parse_items(json)?;
        specs
            .iter()
            .map(|spec| {
                let preset = find_preset(&spec.preset, presets)?;
                let rate = spec.rate.unwrap_or(preset.default_rate);
                let currency = effective_currency(preset, default_currency);
                let tax_rate = spec.tax_rate.or(preset.tax_rate).unwrap_or(0.0);
                // The preset is authoritative for the unit: an --items entry
                // inherits it from the preset it references.
                Ok(LineItem::new(
                    preset.description.clone(),
                    spec.quantity,
                    preset.unit,
                    rate,
                    currency,
                    tax_rate,
                ))
            })
            .collect()
    } else {
        // Single-item mode: --preset + one of --quantity/--days/--hours
        let key = args
            .preset
            .as_deref()
            .expect("clap enforces preset or items");
        let preset = find_preset(key, presets)?;
        let quantity = resolve_quantity(args, preset)?;
        validate_quantity(quantity)?;
        let currency = effective_currency(preset, default_currency);
        let tax_rate = preset.tax_rate.unwrap_or(0.0);
        Ok(vec![LineItem::new(
            preset.description.clone(),
            quantity,
            preset.unit,
            preset.default_rate,
            currency,
            tax_rate,
        )])
    }
}

/// Resolve which recipient to use based on the --client flag.
///
/// If no client is specified, returns the default recipient.
/// If a client key is provided, looks it up in the validated recipients list.
fn resolve_recipient<'a>(
    client: Option<&str>,
    validated: &'a ValidatedConfig,
) -> Result<&'a ValidatedRecipient, ConfigError> {
    match client {
        None => Ok(validated.default_recipient()),
        Some(key) => validated
            .recipients
            .iter()
            .find(|r| r.key().as_str() == key)
            .ok_or_else(|| ConfigError::RecipientNotFound {
                key: key.to_string(),
                available: validated
                    .recipients
                    .iter()
                    .map(|r| r.key().as_str().to_string())
                    .collect(),
            }),
    }
}

/// Resolve which sender to use based on the --sender flag.
///
/// If no sender is specified, returns the default sender.
/// If a sender key is provided, looks it up in the validated senders list.
/// Mirrors [`resolve_recipient`].
fn resolve_sender<'a>(
    sender_key: Option<&str>,
    validated: &'a ValidatedConfig,
) -> Result<&'a ValidatedSender, ConfigError> {
    match sender_key {
        None => Ok(validated.default_sender()),
        Some(k) => validated
            .senders
            .iter()
            .find(|s| s.key().as_str() == k)
            .ok_or_else(|| ConfigError::SenderNotFound {
                key: k.to_string(),
                available: validated
                    .senders
                    .iter()
                    .map(|s| s.key().as_str().to_string())
                    .collect(),
            }),
    }
}

/// Resolve the template to use for this invoice.
///
/// Priority: explicit `--template <name>` flag, then config-default. The
/// resolved name must exist in the local templates dir; if not, the user is
/// pointed at `template refresh` and the remote-install prompt for next time.
fn resolve_template(
    registry: &TemplateRegistry,
    flag: Option<&str>,
    config_default: &str,
) -> Result<Template, PdfError> {
    let name = flag.unwrap_or(config_default);
    match registry.find_by_name(name) {
        Some(t) => Ok(t.clone()),
        None => Err(PdfError::TemplateNotFound {
            name: name.to_string(),
            available: registry.names(),
        }),
    }
}

/// Handle `invoice generate` — non-interactive invoice generation.
///
/// `config_path` is the path to the config file (e.g. `~/.config/invoice-generator/config.yaml`).
/// `output_dir` is the directory the resulting PDF is written to (typically the user's CWD).
/// Logo paths in the config are resolved relative to the config file's parent directory.
pub fn handle_generate(
    args: &GenerateArgs,
    config_path: &Path,
    output_dir: &Path,
    writer: &mut dyn Write,
) -> Result<(), AppError> {
    let validated = load_validated_config(config_path)?;
    let recipient = resolve_recipient(args.client.as_deref(), &validated)?;
    let sender = resolve_sender(args.sender.as_deref(), &validated)?;

    // Build a registry snapshot — seeding bundled templates first so the
    // first-run UX still resolves the default `amalthea` template even when
    // the local templates dir doesn't exist yet.
    if let Err(e) = TemplateRegistry::write_builtins_if_missing() {
        eprintln!("Warning: could not seed bundled templates: {e}");
    }
    let registry = TemplateRegistry::scan_local()?;
    let template = resolve_template(&registry, args.template.as_deref(), &validated.template)?;

    let period = validate_period(args.month, args.year)?;
    let line_items = resolve_line_items(args, &validated.presets, validated.defaults.currency)?;
    let locale = match args.locale.as_deref() {
        Some(code) => match Locale::from_str(code) {
            Ok(l) => l,
            Err(_) => {
                eprintln!("Warning: unsupported locale \"{code}\", using en-US");
                Locale::EnUs
            }
        },
        None => validated.locale,
    };
    let summary = build_summary(period, line_items, &validated.defaults)?;
    let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));
    let pdf_bytes = generate_pdf(
        &summary, &validated, sender, recipient, config_dir, &template, locale,
    )?;
    let output_path = pdf_output_path(sender.name(), &period, output_dir);
    std::fs::write(&output_path, &pdf_bytes).map_err(crate::pdf::PdfError::Write)?;
    writeln!(writer, "PDF saved: {}", output_path.display())
        .map_err(crate::pdf::PdfError::Write)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::setup::test_helpers::*;

    // ── Test helper builders ──

    /// `GenerateArgs` with no item source — the base every builder below fills in.
    fn generate_args(month: u32, year: u32) -> GenerateArgs {
        GenerateArgs {
            month,
            year,
            preset: None,
            quantity: None,
            days: None,
            hours: None,
            items: None,
            client: None,
            sender: None,
            template: None,
            locale: None,
        }
    }

    fn generate_single_args(month: u32, year: u32, preset: &str, days: f64) -> GenerateArgs {
        GenerateArgs {
            preset: Some(preset.to_string()),
            days: Some(days),
            ..generate_args(month, year)
        }
    }

    fn generate_quantity_args(month: u32, year: u32, preset: &str, quantity: f64) -> GenerateArgs {
        GenerateArgs {
            preset: Some(preset.to_string()),
            quantity: Some(quantity),
            ..generate_args(month, year)
        }
    }

    fn generate_hours_args(month: u32, year: u32, preset: &str, hours: f64) -> GenerateArgs {
        GenerateArgs {
            preset: Some(preset.to_string()),
            hours: Some(hours),
            ..generate_args(month, year)
        }
    }

    fn generate_items_args(month: u32, year: u32, json: &str) -> GenerateArgs {
        GenerateArgs {
            items: Some(json.to_string()),
            ..generate_args(month, year)
        }
    }

    fn config_with_named_presets(entries: &[(&str, f64)]) -> crate::config::types::Config {
        use crate::config::types::{Config, Preset};
        use crate::domain::PresetKey;
        let presets: Vec<Preset> = entries
            .iter()
            .map(|(key, rate)| Preset {
                key: PresetKey::try_new(*key).unwrap(),
                description: format!("{key} services"),
                default_rate: *rate,
                currency: None,
                tax_rate: None,
                unit: BillingUnit::Day,
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
        assert!(matches!(result, Err(InvoiceError::ItemsParse(_))));
    }

    #[test]
    fn test_parse_items_missing_preset_field_returns_error() {
        // Arrange
        let json = r#"[{"days": 10}]"#;

        // Act
        let result = parse_items(json);

        // Assert
        assert!(matches!(result, Err(InvoiceError::ItemsParse(_))));
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
        assert!(matches!(result, Err(InvoiceError::EmptyItems)));
    }

    #[test]
    fn test_parse_items_zero_days_returns_error() {
        // Arrange
        let json = r#"[{"preset":"dev","days":0}]"#;

        // Act
        let result = parse_items(json);

        // Assert
        assert!(matches!(result, Err(InvoiceError::InvalidQuantity(_))));
    }

    #[test]
    fn test_parse_items_days_key_still_parses() {
        // Arrange — back-compat pin: pre-billing-unit payloads use `days`.
        let json = r#"[{"preset":"dev","days":10}]"#;

        // Act
        let items = parse_items(json).unwrap();

        // Assert
        assert!((items[0].quantity - 10.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_items_quantity_key_parses() {
        // Arrange
        let json = r#"[{"preset":"dev","quantity":8}]"#;

        // Act
        let items = parse_items(json).unwrap();

        // Assert
        assert!((items[0].quantity - 8.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_items_both_days_and_quantity_returns_error() {
        // Arrange — serde treats the alias as a duplicate field, which is the
        // right outcome: the two keys would otherwise silently disagree.
        let json = r#"[{"preset":"dev","days":10,"quantity":8}]"#;

        // Act
        let result = parse_items(json);

        // Assert
        assert!(matches!(result, Err(InvoiceError::ItemsParse(_))));
    }

    // ── Phase 3: Validation tests (pure) ──

    #[test]
    fn test_validate_quantity_zero_returns_error() {
        // Arrange
        let quantity = 0.0;

        // Act
        let result = validate_quantity(quantity);

        // Assert
        assert!(matches!(result, Err(InvoiceError::InvalidQuantity(_))));
    }

    #[test]
    fn test_validate_quantity_negative_returns_error() {
        // Arrange
        let quantity = -3.0;

        // Act
        let result = validate_quantity(quantity);

        // Assert
        assert!(matches!(result, Err(InvoiceError::InvalidQuantity(_))));
    }

    #[test]
    fn test_validate_quantity_non_finite_returns_error() {
        // Arrange
        let quantity = f64::INFINITY;

        // Act
        let result = validate_quantity(quantity);

        // Assert
        assert!(matches!(result, Err(InvoiceError::InvalidQuantity(_))));
    }

    #[test]
    fn test_validate_quantity_positive_succeeds() {
        // Arrange
        let quantity = 5.5;

        // Act
        let result = validate_quantity(quantity);

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
        assert!(matches!(result, Err(InvoiceError::InvalidDate(_))));
    }

    #[test]
    fn test_find_preset_not_found_returns_error() {
        // Arrange
        let presets = synthetic_presets();

        // Act
        let result = find_preset("nonexistent", &presets);

        // Assert
        assert!(matches!(result, Err(ConfigError::PresetNotFound(_))));
    }

    #[test]
    fn test_find_preset_found_returns_preset() {
        // Arrange
        let presets = synthetic_presets();

        // Act
        let result = find_preset("dev", &presets);

        // Assert
        assert!(result.is_ok());
        assert_eq!(result.unwrap().key.as_str(), "dev");
    }

    // ── Billing unit propagation ──

    fn presets_with_units() -> Vec<Preset> {
        use crate::domain::PresetKey;
        vec![
            Preset {
                key: PresetKey::try_new("dev").unwrap(),
                description: "Software development".into(),
                default_rate: 800.0,
                currency: None,
                tax_rate: None,
                unit: BillingUnit::Day,
            },
            Preset {
                key: PresetKey::try_new("support").unwrap(),
                description: "Support retainer".into(),
                default_rate: 120.0,
                currency: None,
                tax_rate: None,
                unit: BillingUnit::Hour,
            },
        ]
    }

    #[test]
    fn test_resolve_single_item_carries_preset_unit() {
        // Arrange — --quantity is unit-agnostic, so the preset supplies the unit.
        let args = generate_quantity_args(3, 2026, "support", 7.5);

        // Act
        let items = resolve_line_items(&args, &presets_with_units(), Currency::Eur).unwrap();

        // Assert
        assert_eq!(items[0].unit, BillingUnit::Hour);
        assert!((items[0].amount - 900.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_resolve_items_entry_inherits_unit_from_referenced_preset() {
        // Arrange — the preset is authoritative for the unit, --items only
        // carries the quantity.
        let args = generate_items_args(
            3,
            2026,
            r#"[{"preset": "dev", "days": 10}, {"preset": "support", "days": 7.5}]"#,
        );

        // Act
        let items = resolve_line_items(&args, &presets_with_units(), Currency::Eur).unwrap();

        // Assert
        assert_eq!(items[0].unit, BillingUnit::Day);
        assert_eq!(items[1].unit, BillingUnit::Hour);
    }

    #[test]
    fn test_resolve_items_rate_override_does_not_change_unit() {
        // Arrange
        let args = generate_items_args(
            3,
            2026,
            r#"[{"preset": "support", "days": 4, "rate": 150}]"#,
        );

        // Act
        let items = resolve_line_items(&args, &presets_with_units(), Currency::Eur).unwrap();

        // Assert
        assert_eq!(items[0].unit, BillingUnit::Hour);
        assert!((items[0].amount - 600.0).abs() < f64::EPSILON);
    }

    // ── Amount flags: --quantity / --days / --hours ──

    #[test]
    fn test_quantity_flag_on_hourly_preset_produces_hour_line_item() {
        // Arrange
        let args = generate_quantity_args(3, 2026, "support", 8.0);

        // Act
        let items = resolve_line_items(&args, &presets_with_units(), Currency::Eur).unwrap();

        // Assert
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].unit, BillingUnit::Hour);
        assert!((items[0].quantity - 8.0).abs() < f64::EPSILON);
        assert!((items[0].amount - 960.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_days_flag_on_daily_preset_unchanged() {
        // Arrange — regression pin: existing scripts must behave exactly as before.
        let args = generate_single_args(3, 2026, "dev", 10.0);

        // Act
        let items = resolve_line_items(&args, &presets_with_units(), Currency::Eur).unwrap();

        // Assert
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].unit, BillingUnit::Day);
        assert!((items[0].quantity - 10.0).abs() < f64::EPSILON);
        assert!((items[0].amount - 8000.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_hours_flag_on_daily_preset_returns_unit_mismatch() {
        // Arrange
        let args = generate_hours_args(3, 2026, "dev", 8.0);

        // Act
        let result = resolve_line_items(&args, &presets_with_units(), Currency::Eur);

        // Assert
        match result {
            Err(AppError::Invoice(InvoiceError::UnitMismatch { flag, preset, unit })) => {
                assert_eq!(flag, "--hours");
                assert_eq!(preset, "dev");
                assert_eq!(unit, BillingUnit::Day);
            }
            other => panic!("Expected UnitMismatch, got {other:?}"),
        }
    }

    #[test]
    fn test_days_flag_on_hourly_preset_returns_unit_mismatch() {
        // Arrange
        let args = generate_single_args(3, 2026, "support", 10.0);

        // Act
        let result = resolve_line_items(&args, &presets_with_units(), Currency::Eur);

        // Assert
        match result {
            Err(AppError::Invoice(InvoiceError::UnitMismatch { flag, preset, unit })) => {
                assert_eq!(flag, "--days");
                assert_eq!(preset, "support");
                assert_eq!(unit, BillingUnit::Hour);
            }
            other => panic!("Expected UnitMismatch, got {other:?}"),
        }
    }

    #[test]
    fn test_quantity_flag_never_mismatches_either_unit() {
        // Arrange — --quantity is unit-agnostic by design.
        let presets = presets_with_units();

        // Act
        let daily = resolve_quantity(&generate_quantity_args(3, 2026, "dev", 4.0), &presets[0]);
        let hourly = resolve_quantity(
            &generate_quantity_args(3, 2026, "support", 4.0),
            &presets[1],
        );

        // Assert
        assert!((daily.unwrap() - 4.0).abs() < f64::EPSILON);
        assert!((hourly.unwrap() - 4.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_resolve_quantity_without_any_amount_flag_returns_error() {
        // Arrange — clap's `amount` group prevents this; the handler must not panic.
        let args = GenerateArgs {
            preset: Some("dev".into()),
            ..generate_args(3, 2026)
        };

        // Act
        let result = resolve_quantity(&args, &presets_with_units()[0]);

        // Assert
        assert!(matches!(result, Err(InvoiceError::MissingQuantity)));
    }

    #[test]
    fn test_single_item_non_positive_quantity_returns_invalid_quantity() {
        // Arrange
        let args = generate_quantity_args(3, 2026, "dev", 0.0);

        // Act
        let result = resolve_line_items(&args, &presets_with_units(), Currency::Eur);

        // Assert
        assert!(matches!(
            result,
            Err(AppError::Invoice(InvoiceError::InvalidQuantity(_)))
        ));
    }

    // ── Phase 4: Handler tests — single-item (tempdir) ──

    #[test]
    fn test_handle_generate_no_config_returns_error() {
        // Arrange
        let dir = setup_dir(None);
        let args = generate_single_args(3, 2026, "dev", 10.0);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf);

        // Assert
        assert!(matches!(
            result,
            Err(AppError::Config(ConfigError::NotFound))
        ));
    }

    #[test]
    fn test_handle_generate_preset_not_found_returns_error() {
        // Arrange
        let config = complete_config();
        let dir = setup_dir(Some(&config));
        let args = generate_single_args(3, 2026, "nonexistent", 10.0);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf);

        // Assert
        assert!(matches!(
            result,
            Err(AppError::Config(ConfigError::PresetNotFound(_)))
        ));
    }

    #[test]
    fn test_handle_generate_single_item_produces_pdf_file() {
        // Arrange
        let config = complete_config();
        let dir = setup_dir(Some(&config));
        let args = generate_single_args(3, 2026, "dev", 10.0);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf).unwrap();

        // Assert
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("PDF saved:"),
            "Expected 'PDF saved:' in: {output}"
        );
        let pdf_path = dir.path().join("Invoice_Alice_Smith_Mar2026.pdf");
        assert!(pdf_path.exists(), "PDF file should exist");
        let bytes = std::fs::read(&pdf_path).unwrap();
        assert!(
            bytes.starts_with(b"%PDF"),
            "File should start with %PDF header"
        );
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
        handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf).unwrap();

        // Assert
        let bytes = std::fs::read(&pdf_path).unwrap();
        assert!(
            bytes.starts_with(b"%PDF"),
            "File should be overwritten with actual PDF"
        );
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
        handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf).unwrap();

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
        let result = handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf);

        // Assert
        match result {
            Err(AppError::Config(ConfigError::PresetNotFound(key))) => assert_eq!(key, "bogus"),
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
        handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf).unwrap();

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
        handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf).unwrap();

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
        assert_eq!(recipient.name(), "Acme Corp");
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
        assert_eq!(recipient.name(), "Globex Inc");
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
        assert!(matches!(result, Err(ConfigError::RecipientNotFound { .. })));
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
            Err(ConfigError::RecipientNotFound { key, available }) => {
                assert_eq!(key, "nope");
                assert!(available.contains(&"acme".to_string()));
                assert!(available.contains(&"globex".to_string()));
            }
            other => panic!("Expected RecipientNotFound, got {other:?}"),
        }
    }

    // ── Phase: resolve_sender tests (pure) ──

    /// Build a `ValidatedConfig` with two keyed senders (`alice` default, `bob`
    /// extra), reusing the recipient v2 fixture for everything else. Lives
    /// locally in this test module to avoid mutating `test_helpers.rs` ahead
    /// of step 9 (which is where the shared sender fixtures land).
    fn validated_two_senders() -> crate::config::validator::ValidatedConfig {
        use crate::config::types::{Config, Sender};
        use crate::domain::SenderKey;
        let alice = Sender {
            key: Some(SenderKey::try_new("alice").unwrap()),
            name: "Alice Smith".into(),
            address: vec!["42 Elm Street".into()],
            email: "alice@example.com".into(),
            extras: None,
        };
        let bob = Sender {
            key: Some(SenderKey::try_new("bob").unwrap()),
            name: "Bob Jones".into(),
            address: vec!["7 Oak Avenue".into()],
            email: "bob@example.com".into(),
            extras: None,
        };
        let base = crate::setup::test_helpers::v2_config_two_recipients();
        let config = Config {
            sender: None,
            senders: Some(vec![alice, bob]),
            default_sender: Some(SenderKey::try_new("alice").unwrap()),
            ..base
        };
        crate::setup::test_helpers::validated(config)
    }

    #[test]
    fn test_resolve_sender_default_used_when_flag_absent() {
        // Arrange
        let validated = validated_two_senders();

        // Act
        let result = resolve_sender(None, &validated);

        // Assert
        let sender = result.unwrap();
        assert_eq!(sender.key().as_str(), "alice");
        assert_eq!(sender.name(), "Alice Smith");
    }

    #[test]
    fn test_resolve_sender_flag_overrides_default() {
        // Arrange
        let validated = validated_two_senders();

        // Act
        let result = resolve_sender(Some("bob"), &validated);

        // Assert
        let sender = result.unwrap();
        assert_eq!(sender.key().as_str(), "bob");
        assert_eq!(sender.name(), "Bob Jones");
    }

    #[test]
    fn test_resolve_sender_unknown_key_returns_error() {
        // Arrange
        let validated = validated_two_senders();

        // Act
        let result = resolve_sender(Some("nonexistent"), &validated);

        // Assert
        match result {
            Err(ConfigError::SenderNotFound { key, available }) => {
                assert_eq!(key, "nonexistent");
                assert!(available.contains(&"alice".to_string()));
                assert!(available.contains(&"bob".to_string()));
            }
            other => panic!("Expected SenderNotFound, got {other:?}"),
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
        let result = handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf);

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
        let result = handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf);

        // Assert
        assert!(matches!(
            result,
            Err(AppError::Config(ConfigError::RecipientNotFound { .. }))
        ));
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
        let result = handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf);

        // Assert
        assert!(
            result.is_ok(),
            "v1 config should work without --client flag: {result:?}"
        );
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("PDF saved:"));
    }

    // ── Phase 9: Currency wiring tests ──

    fn config_with_currency_presets(
        entries: &[(&str, f64, Option<Currency>)],
    ) -> crate::config::types::Config {
        use crate::config::types::{Config, Preset};
        use crate::domain::PresetKey;
        let presets: Vec<Preset> = entries
            .iter()
            .map(|(key, rate, currency)| Preset {
                key: PresetKey::try_new(*key).unwrap(),
                description: format!("{key} services"),
                default_rate: *rate,
                currency: *currency,
                tax_rate: None,
                unit: BillingUnit::Day,
            })
            .collect();
        Config {
            presets: Some(presets),
            ..complete_config()
        }
    }

    #[test]
    fn test_handle_generate_single_item_preset_currency_override() {
        // Arrange — UAH replaces the old CZK fixture (closed Currency enum).
        let config = config_with_currency_presets(&[("dev", 800.0, Some(Currency::Uah))]);
        let dir = setup_dir(Some(&config));
        let args = generate_single_args(3, 2026, "dev", 10.0);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf);

        // Assert
        assert!(result.is_ok(), "Expected Ok, got {result:?}");
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("PDF saved:"));
    }

    #[test]
    fn test_handle_generate_items_mixed_currency_returns_error() {
        // Arrange
        let config = config_with_currency_presets(&[
            ("alpha", 800.0, Some(Currency::Eur)),
            ("beta", 500.0, Some(Currency::Usd)),
        ]);
        let dir = setup_dir(Some(&config));
        let json = r#"[{"preset":"alpha","days":10},{"preset":"beta","days":5}]"#;
        let args = generate_items_args(3, 2026, json);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf);

        // Assert
        assert!(matches!(
            result,
            Err(AppError::Invoice(InvoiceError::MixedCurrency { .. }))
        ));
    }

    fn config_with_tax_presets(
        entries: &[(&str, f64, Option<f64>)],
    ) -> crate::config::types::Config {
        use crate::config::types::{Config, Preset};
        use crate::domain::PresetKey;
        let presets: Vec<Preset> = entries
            .iter()
            .map(|(key, rate, tax)| Preset {
                key: PresetKey::try_new(*key).unwrap(),
                description: format!("{key} services"),
                default_rate: *rate,
                currency: None,
                tax_rate: *tax,
                unit: BillingUnit::Day,
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
        assert!(matches!(result, Err(InvoiceError::InvalidTaxRate(_))));
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
        let result = handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf);

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
        let result = handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf);

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
        let result = handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf);

        // Assert
        assert!(matches!(
            result,
            Err(AppError::Invoice(InvoiceError::InvalidTaxRate(_)))
        ));
    }

    #[test]
    fn test_handle_generate_single_item_uses_preset_tax() {
        // Arrange — single-item mode with preset that has tax_rate
        let config = config_with_tax_presets(&[("dev", 800.0, Some(21.0))]);
        let dir = setup_dir(Some(&config));
        let args = generate_single_args(3, 2026, "dev", 10.0);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf);

        // Assert
        assert!(result.is_ok(), "Expected Ok, got {result:?}");
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("PDF saved:"));
    }

    #[test]
    fn test_handle_generate_items_same_override_currency_succeeds() {
        // Arrange
        let config = config_with_currency_presets(&[
            ("alpha", 800.0, Some(Currency::Usd)),
            ("beta", 500.0, Some(Currency::Usd)),
        ]);
        let dir = setup_dir(Some(&config));
        let json = r#"[{"preset":"alpha","days":10},{"preset":"beta","days":5}]"#;
        let args = generate_items_args(3, 2026, json);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf);

        // Assert
        assert!(result.is_ok(), "Expected Ok, got {result:?}");
    }

    // ── Story 12.8: --template flag handler tests ──

    #[test]
    fn test_handle_generate_with_template_flag_produces_pdf() {
        // Arrange
        let config = complete_config();
        let dir = setup_dir(Some(&config));
        let mut args = generate_single_args(3, 2026, "dev", 10.0);
        args.template = Some("amalthea".to_string());
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf);

        // Assert
        assert!(result.is_ok(), "Expected Ok, got {result:?}");
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("PDF saved:"));
    }

    #[test]
    fn test_handle_generate_without_template_uses_config_default() {
        // Arrange
        let config = complete_config();
        let dir = setup_dir(Some(&config));
        let args = generate_single_args(3, 2026, "dev", 10.0);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf);

        // Assert
        assert!(result.is_ok(), "Expected Ok, got {result:?}");
    }

    #[test]
    fn test_handle_generate_invalid_template_returns_error() {
        // Arrange
        let config = complete_config();
        let dir = setup_dir(Some(&config));
        let mut args = generate_single_args(3, 2026, "dev", 10.0);
        args.template = Some("nonexistent".to_string());
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf);

        // Assert
        assert!(matches!(
            result,
            Err(AppError::Pdf(PdfError::TemplateNotFound { .. }))
        ));
    }

    // ── Story 13.3: --locale flag handler tests ──

    #[test]
    fn test_handle_generate_with_locale_flag_de_de() {
        // Arrange
        let config = complete_config();
        let dir = setup_dir(Some(&config));
        let mut args = generate_single_args(3, 2026, "dev", 10.0);
        args.locale = Some("de-DE".into());
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf);

        // Assert
        assert!(result.is_ok(), "Expected Ok, got {result:?}");
        let pdf_path = dir.path().join("Invoice_Alice_Smith_Mar2026.pdf");
        let bytes = std::fs::read(&pdf_path).unwrap();
        assert!(!bytes.is_empty(), "PDF should be non-empty");
        assert!(
            bytes.starts_with(b"%PDF"),
            "File should start with %PDF header"
        );
    }

    #[test]
    fn test_handle_generate_without_locale_uses_config_default() {
        // Arrange
        let config = complete_config();
        let dir = setup_dir(Some(&config));
        let args = generate_single_args(3, 2026, "dev", 10.0);
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf);

        // Assert
        assert!(result.is_ok(), "Expected Ok, got {result:?}");
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("PDF saved:"));
    }

    #[test]
    fn test_handle_generate_unsupported_locale_warns_and_falls_back() {
        // Arrange
        let config = complete_config();
        let dir = setup_dir(Some(&config));
        let mut args = generate_single_args(3, 2026, "dev", 10.0);
        args.locale = Some("xx-YY".into());
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf);

        // Assert — should succeed (falls back to en-US), not error
        assert!(
            result.is_ok(),
            "Unsupported locale should fall back, not error: {result:?}"
        );
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("PDF saved:"));
    }

    #[test]
    fn test_handle_generate_locale_with_items_mode() {
        // Arrange
        let config = config_with_named_presets(&[("alpha", 800.0)]);
        let dir = setup_dir(Some(&config));
        let json = r#"[{"preset":"alpha","days":5}]"#;
        let mut args = generate_items_args(3, 2026, json);
        args.locale = Some("fr-FR".into());
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf);

        // Assert
        assert!(result.is_ok(), "Expected Ok, got {result:?}");
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("PDF saved:"));
    }

    #[test]
    fn test_handle_generate_invalid_template_error_lists_available() {
        // Arrange
        let config = complete_config();
        let dir = setup_dir(Some(&config));
        let mut args = generate_single_args(3, 2026, "dev", 10.0);
        args.template = Some("xyz".to_string());
        let mut buf: Vec<u8> = Vec::new();

        // Act
        let result = handle_generate(&args, &cfg_path(&dir), dir.path(), &mut buf);

        // Assert
        match result {
            Err(AppError::Pdf(PdfError::TemplateNotFound { name, available })) => {
                assert_eq!(name, "xyz");
                // After the refactor, only the three built-in templates
                // (amalthea/metis/thebe) are guaranteed to be installed
                // in a fresh test environment.
                assert!(
                    available.contains(&"amalthea".to_string()),
                    "Expected 'amalthea' in available: {available:?}"
                );
            }
            other => panic!("Expected TemplateNotFound, got {other:?}"),
        }
    }
}
