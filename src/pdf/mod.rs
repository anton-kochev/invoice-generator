mod data;
mod world;

use std::path::Path;

use typst::layout::PagedDocument;

use crate::config::types::TemplateKey;
use crate::config::validator::ValidatedConfig;
use crate::error::AppError;
use crate::invoice::types::InvoiceSummary;

/// Return the Typst template source for the given template key.
fn template_source(key: TemplateKey) -> &'static str {
    match key {
        TemplateKey::Callisto => include_str!("template/callisto.typ"),
        TemplateKey::Leda => include_str!("template/leda.typ"),
        TemplateKey::Thebe => include_str!("template/thebe.typ"),
        TemplateKey::Amalthea => include_str!("template/amalthea.typ"),
        TemplateKey::Metis => include_str!("template/metis.typ"),
    }
}

/// Resolve a logo path relative to the config directory.
/// Returns (virtual_filename, bytes) if the file exists and is a supported format.
/// Prints a warning and returns None if missing or unsupported.
fn resolve_logo(raw_path: &str, config_dir: &Path) -> Option<(String, Vec<u8>)> {
    let path = config_dir.join(raw_path);
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match ext.as_deref() {
        Some("png") | Some("jpg") | Some("jpeg") => {}
        _ => {
            eprintln!(
                "Warning: unsupported logo format '{}', skipping logo",
                raw_path
            );
            return None;
        }
    }

    match std::fs::read(&path) {
        Ok(bytes) => {
            let virtual_name = format!("logo.{}", ext.unwrap());
            Some((virtual_name, bytes))
        }
        Err(e) => {
            eprintln!(
                "Warning: could not read logo '{}': {e}, generating PDF without logo",
                raw_path
            );
            None
        }
    }
}

/// Generate a PDF from a computed invoice summary and validated config.
pub fn generate_pdf(
    summary: &InvoiceSummary,
    config: &ValidatedConfig,
    recipient: &crate::config::types::Recipient,
    config_dir: &Path,
    template: TemplateKey,
    locale: crate::locale::Locale,
) -> Result<Vec<u8>, AppError> {
    let logo = config
        .branding
        .logo
        .as_deref()
        .and_then(|p| resolve_logo(p, config_dir));
    let logo_file = logo.as_ref().map(|(name, _)| name.clone());
    let invoice_data = data::InvoiceData::from_parts(summary, config, recipient, logo_file, locale);

    let json = serde_json::to_vec(&invoice_data)
        .map_err(|e| AppError::PdfCompile(format!("JSON serialization failed: {e}")))?;

    let source = template_source(template);
    let world = world::InvoiceWorld::new(source, json, logo);

    let warned = typst::compile::<PagedDocument>(&world);
    let document = warned.output.map_err(|diagnostics| {
        let messages: Vec<String> = diagnostics
            .iter()
            .map(|d| d.message.to_string())
            .collect();
        AppError::PdfCompile(messages.join("; "))
    })?;

    let pdf =
        typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).map_err(|errors| {
            let messages: Vec<String> =
                errors.iter().map(|e| e.message.to_string()).collect();
            AppError::PdfExport(messages.join("; "))
        })?;

    Ok(pdf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::*;
    use crate::config::validator::ValidatedBranding;
    use crate::invoice::types::*;
    use time::{Date, Month};

    fn make_summary() -> InvoiceSummary {
        InvoiceSummary {
            invoice_number: "INV-2026-03".into(),
            period: InvoicePeriod::new(3, 2026).unwrap(),
            invoice_date: Date::from_calendar_date(2026, Month::April, 9).unwrap(),
            due_date: Date::from_calendar_date(2026, Month::May, 9).unwrap(),
            currency: crate::domain::Currency::Eur,
            line_items: vec![
                LineItem::new("Software development".into(), 10.0, 800.0, crate::domain::Currency::Eur),
                LineItem::new("Technical consulting".into(), 5.0, 1000.0, crate::domain::Currency::Eur),
            ],
            subtotal: 13000.0,
            tax_total: 0.0,
            total: 13000.0,
        }
    }

    fn make_config() -> ValidatedConfig {
        let recipient = Recipient {
            key: Some(crate::domain::RecipientKey::try_new("acme-corp").unwrap()),
            name: "Acme Corp".into(),
            address: vec!["456 Oak Ave".into(), "Berlin, Germany".into()],
            company_id: Some("DE123456".into()),
            vat_number: Some("ATU12345678".into()),
        };
        ValidatedConfig {
            sender: Sender {
                name: "Jane Doe".into(),
                address: vec!["123 Main St".into(), "Vienna, Austria".into()],
                email: "jane@example.com".into(),
            },
            recipient: recipient.clone(),
            recipients: vec![recipient],
            default_recipient_key: crate::domain::RecipientKey::try_new("acme-corp").unwrap(),
            payment: vec![PaymentMethod {
                label: "Primary Bank Account".into(),
                iban: crate::domain::Iban::try_new("DE89 3704 0044 0532 0130 00").unwrap(),
                bic_swift: "COBADEFFXXX".into(),
            }],
            presets: vec![Preset {
                key: crate::domain::PresetKey::try_new("dev").unwrap(),
                description: "Software development".into(),
                default_rate: 800.0,
                currency: None,
                tax_rate: None,
            }],
            defaults: Defaults::default(),
            branding: ValidatedBranding::default(),
            template: TemplateKey::Leda,
            locale: crate::locale::Locale::EnUs,
        }
    }

    #[test]
    fn test_template_source_leda_returns_nonempty_string() {
        // Arrange & Act
        let source = template_source(TemplateKey::Leda);
        // Assert
        assert!(!source.is_empty());
        assert!(source.contains("#"), "Should contain Typst syntax");
    }

    #[test]
    fn test_template_source_all_keys_return_nonempty() {
        // Arrange & Act & Assert
        for key in TemplateKey::ALL {
            let source = template_source(key);
            assert!(!source.is_empty(), "template_source({key}) should be non-empty");
        }
    }

    #[test]
    fn test_generate_pdf_with_explicit_leda_template() {
        // Arrange
        let summary = make_summary();
        let config = make_config();
        // Act
        let result = generate_pdf(&summary, &config, &config.recipient, Path::new("."), TemplateKey::Leda, crate::locale::Locale::EnUs);
        // Assert
        let pdf = result.expect("PDF generation should succeed");
        assert!(pdf.starts_with(b"%PDF"), "Output should start with PDF header");
    }

    #[test]
    fn test_generate_pdf_deterministic_with_template() {
        // Arrange
        let summary = make_summary();
        let config = make_config();
        // Act
        let pdf1 = generate_pdf(&summary, &config, &config.recipient, Path::new("."), TemplateKey::Leda, crate::locale::Locale::EnUs).unwrap();
        let pdf2 = generate_pdf(&summary, &config, &config.recipient, Path::new("."), TemplateKey::Leda, crate::locale::Locale::EnUs).unwrap();
        // Assert
        assert_eq!(pdf1, pdf2, "Same input should produce identical PDF bytes");
    }

    #[test]
    fn test_generate_pdf_with_non_leda_key_succeeds() {
        // Arrange
        let summary = make_summary();
        let config = make_config();
        // Act
        let result = generate_pdf(&summary, &config, &config.recipient, Path::new("."), TemplateKey::Callisto, crate::locale::Locale::EnUs);
        // Assert
        assert!(result.is_ok(), "Callisto template should produce a valid PDF");
    }

    // ── Sprint 10 Step 5: resolve_logo + logo integration tests ──

    #[test]
    fn test_resolve_logo_existing_file_returns_bytes() {
        // Arrange
        let dir = tempfile::tempdir().unwrap();
        let logo_path = dir.path().join("logo.png");
        // Minimal PNG header (8 bytes)
        std::fs::write(&logo_path, b"\x89PNG\r\n\x1a\n").unwrap();
        // Act
        let result = resolve_logo("logo.png", dir.path());
        // Assert
        assert!(result.is_some());
        let (name, bytes) = result.unwrap();
        assert_eq!(name, "logo.png");
        assert_eq!(bytes, b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn test_resolve_logo_missing_file_returns_none() {
        // Arrange
        let dir = tempfile::tempdir().unwrap();
        // Act
        let result = resolve_logo("nonexistent.png", dir.path());
        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_logo_relative_path_resolved() {
        // Arrange
        let dir = tempfile::tempdir().unwrap();
        let subdir = dir.path().join("assets");
        std::fs::create_dir(&subdir).unwrap();
        std::fs::write(subdir.join("logo.jpg"), b"\xFF\xD8\xFF").unwrap();
        // Act
        let result = resolve_logo("assets/logo.jpg", dir.path());
        // Assert
        assert!(result.is_some());
        let (name, _) = result.unwrap();
        assert_eq!(name, "logo.jpg");
    }

    #[test]
    fn test_resolve_logo_unsupported_format_returns_none() {
        // Arrange
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("logo.svg"), b"<svg></svg>").unwrap();
        // Act
        let result = resolve_logo("logo.svg", dir.path());
        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_generate_pdf_with_logo_none_succeeds() {
        // Arrange
        let summary = make_summary();
        let config = make_config(); // branding.logo is None
        // Act
        let result = generate_pdf(&summary, &config, &config.recipient, Path::new("."), TemplateKey::Leda, crate::locale::Locale::EnUs);
        // Assert
        assert!(result.is_ok());
    }

    #[test]
    fn test_generate_pdf_with_custom_branding_succeeds() {
        // Arrange
        let summary = make_summary();
        let mut config = make_config();
        config.branding.accent_color = crate::domain::HexColor::try_new("#ff5500").unwrap();
        config.branding.font = Some("Arial".into());
        config.branding.footer_text = Some("Custom footer text".into());
        // Act
        let result = generate_pdf(&summary, &config, &config.recipient, Path::new("."), TemplateKey::Leda, crate::locale::Locale::EnUs);
        // Assert
        let pdf = result.expect("PDF with custom branding should succeed");
        assert!(pdf.starts_with(b"%PDF"));
    }

    #[test]
    fn test_generate_pdf_with_empty_footer_succeeds() {
        // Arrange
        let summary = make_summary();
        let mut config = make_config();
        config.branding.footer_text = Some("".into());
        // Act
        let result = generate_pdf(&summary, &config, &config.recipient, Path::new("."), TemplateKey::Leda, crate::locale::Locale::EnUs);
        // Assert
        assert!(result.is_ok());
    }

    // ── Sprint 13: Template distinctness and per-template tests ──

    fn make_summary_with_tax() -> InvoiceSummary {
        InvoiceSummary {
            invoice_number: "INV-2026-03".into(),
            period: InvoicePeriod::new(3, 2026).unwrap(),
            invoice_date: Date::from_calendar_date(2026, Month::April, 9).unwrap(),
            due_date: Date::from_calendar_date(2026, Month::May, 9).unwrap(),
            currency: crate::domain::Currency::Eur,
            line_items: vec![
                LineItem::with_tax("Software development".into(), 10.0, 800.0, crate::domain::Currency::Eur, 21.0),
                LineItem::with_tax("Technical consulting".into(), 5.0, 1000.0, crate::domain::Currency::Eur, 21.0),
            ],
            subtotal: 13000.0,
            tax_total: 2730.0,
            total: 15730.0,
        }
    }

    fn make_config_without_optional_fields() -> ValidatedConfig {
        let recipient = Recipient {
            key: Some(crate::domain::RecipientKey::try_new("acme-corp").unwrap()),
            name: "Acme Corp".into(),
            address: vec!["456 Oak Ave".into(), "Berlin, Germany".into()],
            company_id: None,
            vat_number: None,
        };
        ValidatedConfig {
            sender: Sender {
                name: "Jane Doe".into(),
                address: vec!["123 Main St".into(), "Vienna, Austria".into()],
                email: "jane@example.com".into(),
            },
            recipient: recipient.clone(),
            recipients: vec![recipient],
            default_recipient_key: crate::domain::RecipientKey::try_new("acme-corp").unwrap(),
            payment: vec![PaymentMethod {
                label: "Primary Bank Account".into(),
                iban: crate::domain::Iban::try_new("DE89 3704 0044 0532 0130 00").unwrap(),
                bic_swift: "COBADEFFXXX".into(),
            }],
            presets: vec![Preset {
                key: crate::domain::PresetKey::try_new("dev").unwrap(),
                description: "Software development".into(),
                default_rate: 800.0,
                currency: None,
                tax_rate: None,
            }],
            defaults: Defaults::default(),
            branding: ValidatedBranding::default(),
            template: TemplateKey::Leda,
            locale: crate::locale::Locale::EnUs,
        }
    }

    #[test]
    fn test_template_source_each_key_returns_distinct_content() {
        // Arrange
        let keys = TemplateKey::ALL;

        // Act
        let sources: Vec<&str> = keys.iter().map(|k| template_source(*k)).collect();

        // Assert
        for i in 0..sources.len() {
            for j in (i + 1)..sources.len() {
                assert_ne!(
                    sources[i], sources[j],
                    "template_source({}) and template_source({}) should be distinct",
                    keys[i], keys[j]
                );
            }
        }
    }

    #[test]
    fn test_generate_pdf_callisto_produces_valid_pdf() {
        // Arrange
        let summary = make_summary();
        let config = make_config();

        // Act
        let result = generate_pdf(&summary, &config, &config.recipient, Path::new("."), TemplateKey::Callisto, crate::locale::Locale::EnUs);

        // Assert
        let pdf = result.expect("Callisto template should produce a valid PDF");
        assert!(pdf.starts_with(b"%PDF"), "Output should start with PDF header");
    }

    #[test]
    fn test_generate_pdf_thebe_produces_valid_pdf() {
        // Arrange
        let summary = make_summary();
        let config = make_config();

        // Act
        let result = generate_pdf(&summary, &config, &config.recipient, Path::new("."), TemplateKey::Thebe, crate::locale::Locale::EnUs);

        // Assert
        let pdf = result.expect("Thebe template should produce a valid PDF");
        assert!(pdf.starts_with(b"%PDF"), "Output should start with PDF header");
    }

    #[test]
    fn test_generate_pdf_amalthea_produces_valid_pdf() {
        // Arrange
        let summary = make_summary();
        let config = make_config();

        // Act
        let result = generate_pdf(&summary, &config, &config.recipient, Path::new("."), TemplateKey::Amalthea, crate::locale::Locale::EnUs);

        // Assert
        let pdf = result.expect("Amalthea template should produce a valid PDF");
        assert!(pdf.starts_with(b"%PDF"), "Output should start with PDF header");
    }

    #[test]
    fn test_generate_pdf_metis_produces_valid_pdf() {
        // Arrange
        let summary = make_summary();
        let config = make_config();

        // Act
        let result = generate_pdf(&summary, &config, &config.recipient, Path::new("."), TemplateKey::Metis, crate::locale::Locale::EnUs);

        // Assert
        let pdf = result.expect("Metis template should produce a valid PDF");
        assert!(pdf.starts_with(b"%PDF"), "Output should start with PDF header");
    }

    #[test]
    fn test_generate_pdf_all_templates_with_tax_succeed() {
        // Arrange
        let summary = make_summary_with_tax();
        let config = make_config();

        // Act & Assert
        for key in TemplateKey::ALL {
            let result = generate_pdf(&summary, &config, &config.recipient, Path::new("."), key, crate::locale::Locale::EnUs);
            assert!(result.is_ok(), "Template {key} should succeed with tax line items");
        }
    }

    #[test]
    fn test_generate_pdf_all_templates_without_optional_fields_succeed() {
        // Arrange
        let summary = make_summary();
        let config = make_config_without_optional_fields();

        // Act & Assert
        for key in TemplateKey::ALL {
            let result = generate_pdf(&summary, &config, &config.recipient, Path::new("."), key, crate::locale::Locale::EnUs);
            assert!(result.is_ok(), "Template {key} should succeed without optional fields");
        }
    }

    #[test]
    fn test_generate_pdf_all_templates_with_custom_branding_succeed() {
        // Arrange
        let summary = make_summary();
        let mut config = make_config();
        config.branding.accent_color = crate::domain::HexColor::try_new("#ff5500").unwrap();
        config.branding.font = Some("Arial".into());
        config.branding.footer_text = Some("Custom footer".into());

        // Act & Assert
        for key in TemplateKey::ALL {
            let result = generate_pdf(&summary, &config, &config.recipient, Path::new("."), key, crate::locale::Locale::EnUs);
            assert!(result.is_ok(), "Template {key} should succeed with custom branding");
        }
    }

    #[test]
    fn test_generate_pdf_all_templates_with_empty_footer_succeed() {
        // Arrange
        let summary = make_summary();
        let mut config = make_config();
        config.branding.footer_text = Some("".into());

        // Act & Assert
        for key in TemplateKey::ALL {
            let result = generate_pdf(&summary, &config, &config.recipient, Path::new("."), key, crate::locale::Locale::EnUs);
            assert!(result.is_ok(), "Template {key} should succeed with empty footer");
        }
    }
}
