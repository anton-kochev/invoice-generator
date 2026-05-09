mod data;
pub mod error;
pub mod manifest;
pub mod registry;
pub mod remote;
mod world;

pub use error::PdfError;
pub use registry::Template;

use std::path::Path;

use typst::layout::PagedDocument;

use crate::config::validator::{ValidatedConfig, ValidatedRecipient};
use crate::invoice::types::InvoiceSummary;

/// Read the on-disk Typst source for a template.
fn read_template_source(template: &Template) -> Result<String, PdfError> {
    std::fs::read_to_string(&template.source).map_err(|e| {
        PdfError::Manifest(format!(
            "read template '{}' at {}: {e}",
            template.name,
            template.source.display()
        ))
    })
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
    recipient: &ValidatedRecipient,
    config_dir: &Path,
    template: &Template,
    locale: crate::locale::Locale,
) -> Result<Vec<u8>, PdfError> {
    let logo = config
        .branding
        .logo
        .as_deref()
        .and_then(|p| resolve_logo(p, config_dir));
    let logo_file = logo.as_ref().map(|(name, _)| name.clone());
    let invoice_data = data::InvoiceData::from_parts(summary, config, recipient, logo_file, locale);

    let json = serde_json::to_vec(&invoice_data)
        .map_err(|e| PdfError::Compile(format!("JSON serialization failed: {e}")))?;

    let source = read_template_source(template)?;
    let world = world::InvoiceWorld::new(&source, json, logo);

    let warned = typst::compile::<PagedDocument>(&world);
    let document = warned.output.map_err(|diagnostics| {
        let messages: Vec<String> = diagnostics.iter().map(|d| d.message.to_string()).collect();
        PdfError::Compile(messages.join("; "))
    })?;

    let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default()).map_err(|errors| {
        let messages: Vec<String> = errors.iter().map(|e| e.message.to_string()).collect();
        PdfError::Export(messages.join("; "))
    })?;

    Ok(pdf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::types::*;
    use crate::config::validator::{ValidatedBranding, ValidatedPaymentMethod};
    use crate::invoice::types::*;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use time::{Date, Month};

    /// Path to the repo's `templates/` dir, used by tests so they don't depend
    /// on the user's `<XDG_CONFIG>` state.
    fn repo_templates_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("templates")
    }

    /// Build a `Template` pointing at a `.typ` file in the repo's
    /// `templates/` dir. Used by tests to render every shipped template
    /// without going through the user's local install.
    fn repo_template(name: &str) -> Template {
        Template {
            name: name.to_string(),
            description: None,
            source: repo_templates_dir().join(format!("{name}.typ")),
        }
    }

    /// All shipped template names. Tests iterate over this in lieu of the
    /// removed `TemplateKey::ALL`.
    const ALL_TEMPLATE_NAMES: [&str; 7] = [
        "callisto", "leda", "thebe", "amalthea", "metis", "io", "europa",
    ];

    fn make_summary() -> InvoiceSummary {
        InvoiceSummary {
            invoice_number: "INV-2026-03".into(),
            period: InvoicePeriod::new(3, 2026).unwrap(),
            invoice_date: Date::from_calendar_date(2026, Month::April, 9).unwrap(),
            due_date: Date::from_calendar_date(2026, Month::May, 9).unwrap(),
            currency: crate::domain::Currency::Eur,
            line_items: vec![
                LineItem::new(
                    "Software development".into(),
                    10.0,
                    800.0,
                    crate::domain::Currency::Eur,
                ),
                LineItem::new(
                    "Technical consulting".into(),
                    5.0,
                    1000.0,
                    crate::domain::Currency::Eur,
                ),
            ],
            subtotal: 13000.0,
            tax_total: 0.0,
            total: 13000.0,
        }
    }

    fn make_config() -> ValidatedConfig {
        let recipient = ValidatedRecipient::from_validated_parts(
            crate::domain::RecipientKey::try_new("acme-corp").unwrap(),
            "Acme Corp".into(),
            vec!["456 Oak Ave".into(), "Berlin, Germany".into()],
            Some("DE123456".into()),
            Some("ATU12345678".into()),
        );
        ValidatedConfig::from_validated_parts(
            Sender {
                name: "Jane Doe".into(),
                address: vec!["123 Main St".into(), "Vienna, Austria".into()],
                email: "jane@example.com".into(),
            },
            crate::domain::NonEmpty::try_from_vec(vec![recipient]).unwrap(),
            0,
            crate::domain::NonEmpty::try_from_vec(vec![
                ValidatedPaymentMethod::from_validated_parts(
                    crate::domain::PaymentMethodKey::try_new("primary").unwrap(),
                    Some("Primary Bank Account".into()),
                    crate::domain::Iban::try_new("DE89 3704 0044 0532 0130 00").unwrap(),
                    "COBADEFFXXX".into(),
                ),
            ])
            .unwrap(),
            crate::domain::NonEmpty::try_from_vec(vec![Preset {
                key: crate::domain::PresetKey::try_new("dev").unwrap(),
                description: "Software development".into(),
                default_rate: 800.0,
                currency: None,
                tax_rate: None,
            }])
            .unwrap(),
            Defaults::default(),
            ValidatedBranding::default(),
            "leda".into(),
            crate::locale::Locale::EnUs,
        )
    }

    #[test]
    fn test_read_template_source_leda_returns_nonempty() {
        // Arrange & Act
        let template = repo_template("leda");
        let source = read_template_source(&template).expect("leda source readable");
        // Assert
        assert!(!source.is_empty());
        assert!(source.contains("#"), "Should contain Typst syntax");
    }

    #[test]
    fn test_read_template_source_all_shipped_return_nonempty() {
        // Arrange & Act & Assert
        for name in ALL_TEMPLATE_NAMES {
            let template = repo_template(name);
            let source = read_template_source(&template)
                .unwrap_or_else(|e| panic!("read {name}: {e}"));
            assert!(!source.is_empty(), "{name} should be non-empty");
        }
    }

    #[test]
    fn test_generate_pdf_with_explicit_leda_template() {
        // Arrange
        let summary = make_summary();
        let config = make_config();
        let template = repo_template("leda");
        // Act
        let result = generate_pdf(
            &summary,
            &config,
            config.default_recipient(),
            Path::new("."),
            &template,
            crate::locale::Locale::EnUs,
        );
        // Assert
        let pdf = result.expect("PDF generation should succeed");
        assert!(
            pdf.starts_with(b"%PDF"),
            "Output should start with PDF header"
        );
    }

    #[test]
    fn test_generate_pdf_deterministic_with_template() {
        // Arrange
        let summary = make_summary();
        let config = make_config();
        let template = repo_template("leda");
        // Act
        let pdf1 = generate_pdf(
            &summary,
            &config,
            config.default_recipient(),
            Path::new("."),
            &template,
            crate::locale::Locale::EnUs,
        )
        .unwrap();
        let pdf2 = generate_pdf(
            &summary,
            &config,
            config.default_recipient(),
            Path::new("."),
            &template,
            crate::locale::Locale::EnUs,
        )
        .unwrap();
        // Assert
        assert_eq!(pdf1, pdf2, "Same input should produce identical PDF bytes");
    }

    #[test]
    fn test_generate_pdf_with_non_leda_key_succeeds() {
        // Arrange
        let summary = make_summary();
        let config = make_config();
        let template = repo_template("callisto");
        // Act
        let result = generate_pdf(
            &summary,
            &config,
            config.default_recipient(),
            Path::new("."),
            &template,
            crate::locale::Locale::EnUs,
        );
        // Assert
        assert!(
            result.is_ok(),
            "Callisto template should produce a valid PDF"
        );
    }

    // ── Sprint 10 Step 5: resolve_logo + logo integration tests ──

    #[test]
    fn test_resolve_logo_existing_file_returns_bytes() {
        // Arrange
        let dir = TempDir::new().unwrap();
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
        let dir = TempDir::new().unwrap();
        // Act
        let result = resolve_logo("nonexistent.png", dir.path());
        // Assert
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_logo_relative_path_resolved() {
        // Arrange
        let dir = TempDir::new().unwrap();
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
        let dir = TempDir::new().unwrap();
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
        let template = repo_template("leda");
        // Act
        let result = generate_pdf(
            &summary,
            &config,
            config.default_recipient(),
            Path::new("."),
            &template,
            crate::locale::Locale::EnUs,
        );
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
        let template = repo_template("leda");
        // Act
        let result = generate_pdf(
            &summary,
            &config,
            config.default_recipient(),
            Path::new("."),
            &template,
            crate::locale::Locale::EnUs,
        );
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
        let template = repo_template("leda");
        // Act
        let result = generate_pdf(
            &summary,
            &config,
            config.default_recipient(),
            Path::new("."),
            &template,
            crate::locale::Locale::EnUs,
        );
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
                LineItem::with_tax(
                    "Software development".into(),
                    10.0,
                    800.0,
                    crate::domain::Currency::Eur,
                    21.0,
                ),
                LineItem::with_tax(
                    "Technical consulting".into(),
                    5.0,
                    1000.0,
                    crate::domain::Currency::Eur,
                    21.0,
                ),
            ],
            subtotal: 13000.0,
            tax_total: 2730.0,
            total: 15730.0,
        }
    }

    fn make_config_without_optional_fields() -> ValidatedConfig {
        let recipient = ValidatedRecipient::from_validated_parts(
            crate::domain::RecipientKey::try_new("acme-corp").unwrap(),
            "Acme Corp".into(),
            vec!["456 Oak Ave".into(), "Berlin, Germany".into()],
            None,
            None,
        );
        ValidatedConfig::from_validated_parts(
            Sender {
                name: "Jane Doe".into(),
                address: vec!["123 Main St".into(), "Vienna, Austria".into()],
                email: "jane@example.com".into(),
            },
            crate::domain::NonEmpty::try_from_vec(vec![recipient]).unwrap(),
            0,
            crate::domain::NonEmpty::try_from_vec(vec![
                ValidatedPaymentMethod::from_validated_parts(
                    crate::domain::PaymentMethodKey::try_new("primary").unwrap(),
                    Some("Primary Bank Account".into()),
                    crate::domain::Iban::try_new("DE89 3704 0044 0532 0130 00").unwrap(),
                    "COBADEFFXXX".into(),
                ),
            ])
            .unwrap(),
            crate::domain::NonEmpty::try_from_vec(vec![Preset {
                key: crate::domain::PresetKey::try_new("dev").unwrap(),
                description: "Software development".into(),
                default_rate: 800.0,
                currency: None,
                tax_rate: None,
            }])
            .unwrap(),
            Defaults::default(),
            ValidatedBranding::default(),
            "leda".into(),
            crate::locale::Locale::EnUs,
        )
    }

    /// All non-`io` templates render with the standard data model produced by
    /// `InvoiceData::from_parts`. The `io` template requires a hand-crafted
    /// bilingual data blob and is exercised separately via
    /// [`regen_io_sample`].
    const STANDARD_TEMPLATE_NAMES: [&str; 6] =
        ["callisto", "leda", "thebe", "amalthea", "metis", "europa"];

    #[test]
    fn test_template_source_each_key_returns_distinct_content() {
        // Arrange
        let templates: Vec<Template> =
            ALL_TEMPLATE_NAMES.iter().map(|n| repo_template(n)).collect();

        // Act
        let sources: Vec<String> = templates
            .iter()
            .map(|t| read_template_source(t).expect("template readable"))
            .collect();

        // Assert
        for i in 0..sources.len() {
            for j in (i + 1)..sources.len() {
                assert_ne!(
                    sources[i], sources[j],
                    "{} and {} should be distinct",
                    templates[i].name, templates[j].name
                );
            }
        }
    }

    #[test]
    fn test_generate_pdf_callisto_produces_valid_pdf() {
        // Arrange
        let summary = make_summary();
        let config = make_config();
        let template = repo_template("callisto");

        // Act
        let result = generate_pdf(
            &summary,
            &config,
            config.default_recipient(),
            Path::new("."),
            &template,
            crate::locale::Locale::EnUs,
        );

        // Assert
        let pdf = result.expect("Callisto template should produce a valid PDF");
        assert!(
            pdf.starts_with(b"%PDF"),
            "Output should start with PDF header"
        );
    }

    #[test]
    fn test_generate_pdf_thebe_produces_valid_pdf() {
        // Arrange
        let summary = make_summary();
        let config = make_config();
        let template = repo_template("thebe");

        // Act
        let result = generate_pdf(
            &summary,
            &config,
            config.default_recipient(),
            Path::new("."),
            &template,
            crate::locale::Locale::EnUs,
        );

        // Assert
        let pdf = result.expect("Thebe template should produce a valid PDF");
        assert!(
            pdf.starts_with(b"%PDF"),
            "Output should start with PDF header"
        );
    }

    #[test]
    fn test_generate_pdf_amalthea_produces_valid_pdf() {
        // Arrange
        let summary = make_summary();
        let config = make_config();
        let template = repo_template("amalthea");

        // Act
        let result = generate_pdf(
            &summary,
            &config,
            config.default_recipient(),
            Path::new("."),
            &template,
            crate::locale::Locale::EnUs,
        );

        // Assert
        let pdf = result.expect("Amalthea template should produce a valid PDF");
        assert!(
            pdf.starts_with(b"%PDF"),
            "Output should start with PDF header"
        );
    }

    #[test]
    fn test_generate_pdf_metis_produces_valid_pdf() {
        // Arrange
        let summary = make_summary();
        let config = make_config();
        let template = repo_template("metis");

        // Act
        let result = generate_pdf(
            &summary,
            &config,
            config.default_recipient(),
            Path::new("."),
            &template,
            crate::locale::Locale::EnUs,
        );

        // Assert
        let pdf = result.expect("Metis template should produce a valid PDF");
        assert!(
            pdf.starts_with(b"%PDF"),
            "Output should start with PDF header"
        );
    }

    #[test]
    fn test_generate_pdf_all_templates_with_tax_succeed() {
        // Arrange
        let summary = make_summary_with_tax();
        let config = make_config();

        // Act & Assert — `io` uses a different data model (see regen helper).
        for name in STANDARD_TEMPLATE_NAMES {
            let template = repo_template(name);
            let result = generate_pdf(
                &summary,
                &config,
                config.default_recipient(),
                Path::new("."),
                &template,
                crate::locale::Locale::EnUs,
            );
            assert!(
                result.is_ok(),
                "Template {name} should succeed with tax line items"
            );
        }
    }

    #[test]
    fn test_generate_pdf_all_templates_without_optional_fields_succeed() {
        // Arrange
        let summary = make_summary();
        let config = make_config_without_optional_fields();

        // Act & Assert — `io` uses a different data model (see regen helper).
        for name in STANDARD_TEMPLATE_NAMES {
            let template = repo_template(name);
            let result = generate_pdf(
                &summary,
                &config,
                config.default_recipient(),
                Path::new("."),
                &template,
                crate::locale::Locale::EnUs,
            );
            assert!(
                result.is_ok(),
                "Template {name} should succeed without optional fields"
            );
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

        // Act & Assert — `io` uses a different data model (see regen helper).
        for name in STANDARD_TEMPLATE_NAMES {
            let template = repo_template(name);
            let result = generate_pdf(
                &summary,
                &config,
                config.default_recipient(),
                Path::new("."),
                &template,
                crate::locale::Locale::EnUs,
            );
            assert!(
                result.is_ok(),
                "Template {name} should succeed with custom branding"
            );
        }
    }

    #[test]
    fn test_generate_pdf_all_templates_with_empty_footer_succeed() {
        // Arrange
        let summary = make_summary();
        let mut config = make_config();
        config.branding.footer_text = Some("".into());

        // Act & Assert — `io` uses a different data model (see regen helper).
        for name in STANDARD_TEMPLATE_NAMES {
            let template = repo_template(name);
            let result = generate_pdf(
                &summary,
                &config,
                config.default_recipient(),
                Path::new("."),
                &template,
                crate::locale::Locale::EnUs,
            );
            assert!(
                result.is_ok(),
                "Template {name} should succeed with empty footer"
            );
        }
    }

    /// Build a [`ValidatedConfig`] whose payment method has `label: None`.
    /// Used to verify the conditional `#if "label" in method` branch in every
    /// template renders correctly when the label is absent.
    fn make_config_payment_no_label() -> ValidatedConfig {
        let recipient = ValidatedRecipient::from_validated_parts(
            crate::domain::RecipientKey::try_new("acme-corp").unwrap(),
            "Acme Corp".into(),
            vec!["456 Oak Ave".into(), "Berlin, Germany".into()],
            None,
            None,
        );
        ValidatedConfig::from_validated_parts(
            Sender {
                name: "Jane Doe".into(),
                address: vec!["123 Main St".into(), "Vienna, Austria".into()],
                email: "jane@example.com".into(),
            },
            crate::domain::NonEmpty::try_from_vec(vec![recipient]).unwrap(),
            0,
            crate::domain::NonEmpty::try_from_vec(vec![
                ValidatedPaymentMethod::from_validated_parts(
                    crate::domain::PaymentMethodKey::try_new("mono-eur-sepa").unwrap(),
                    None, // ← absent label: this is the bug-fix scenario
                    crate::domain::Iban::try_new("DE89 3704 0044 0532 0130 00").unwrap(),
                    "COBADEFFXXX".into(),
                ),
            ])
            .unwrap(),
            crate::domain::NonEmpty::try_from_vec(vec![Preset {
                key: crate::domain::PresetKey::try_new("dev").unwrap(),
                description: "Software development".into(),
                default_rate: 800.0,
                currency: None,
                tax_rate: None,
            }])
            .unwrap(),
            Defaults::default(),
            ValidatedBranding::default(),
            "leda".into(),
            crate::locale::Locale::EnUs,
        )
    }

    /// Regen helper: writes `samples/sample_io.pdf` by hand-crafting a
    /// bilingual UA/EN JSON blob (the `io` template needs fields not produced
    /// by `InvoiceData::from_parts`). Bypasses the data mapper entirely and
    /// feeds the JSON directly to `InvoiceWorld`.
    ///
    /// Run with: `cargo test regen_io_sample -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn regen_io_sample() {
        // Arrange — hand-crafted JSON matching the `io.typ` field contract.
        let data = serde_json::json!({
            "sender": {
                "name": "PE Surname Given Patronymic",
                "name_ua": "ФОП Прізвище Імʼя Побатькові",
                "address": [
                    "00000, Ukraine, Kyiv,",
                    "Example St, 1/1"
                ],
                "address_ua": [
                    "00000, Україна, м. Київ,",
                    "вул. Прикладна, 1, кв. 1"
                ],
                "email": "synthetic@example.com"
            },
            "recipient": {
                "name": "Globex OÜ",
                "address": [
                    "Harju maakond, Tallinn,",
                    "Kesklinna linnaosa, Tornimäe tn 5",
                    "10145, Estonia"
                ],
                "address_ua": [
                    "Harju maakond, Tallinn,",
                    "Kesklinna linnaosa, Tornimäe tn 5",
                    "10145, Estonia"
                ]
            },
            "invoice": {
                "number": "2026-04",
                "date": "30.04.2026",
                "currency": "USD",
                "currency_name_ua": "Долар США",
                "description": "Payment for consulting on informatization",
                "description_ua": "Консультування з питань інформатизації",
                "amount_in_words": "Five hundred sixty USD",
                "amount_in_words_ua": "П'ятсот шістдесят доларів США",
                "total": "560.00",
                "line_items": [
                    {
                        "description": "Consulting on informatization. Period: March 2026",
                        "description_ua": "Консультування з питань інформатизації, березень 2026",
                        "days": "1",
                        "rate": "560.00",
                        "amount": "560.00"
                    }
                ]
            },
            "payment": [
                {
                    "beneficiary": "PE SURNAME GIVEN",
                    "beneficiary_bank": "JSC EXAMPLE BANK, KYIV, UKRAINE",
                    "account_number": "UA00 0000 0000 0000 0000 0000 000",
                    "iban": "UA000000000000000000000000000",
                    "bic_swift": "EXMPUAUKXXX",
                    "correspondent_bank": "WISE",
                    "correspondent_account": "BE00 0000 0000 0000",
                    "correspondent_swift": "TRWIBEB1XXX"
                }
            ],
            "branding": {
                "accent_color": "#6b3421",
                "font": ["Helvetica", "Noto Sans", "Liberation Sans"]
            }
        });

        let json = serde_json::to_vec(&data).expect("JSON serialization");
        let template = repo_template("io");
        let source = read_template_source(&template).expect("io source readable");
        let world = world::InvoiceWorld::new(&source, json, None);

        // Act
        let warned = typst::compile::<PagedDocument>(&world);
        let document = warned.output.expect("Io template should compile");
        let pdf = typst_pdf::pdf(&document, &typst_pdf::PdfOptions::default())
            .expect("PDF export should succeed");

        // Assert + write artifact
        assert!(pdf.starts_with(b"%PDF"), "Output should start with PDF header");
        std::fs::write("samples/sample_io.pdf", &pdf).expect("write sample PDF");
    }

    #[test]
    fn test_generate_pdf_all_templates_render_without_label() {
        // Arrange — load-bearing smoke test for the bug fix: every standard
        // template must compile when `label` is absent from the payment dict.
        let summary = make_summary();
        let config = make_config_payment_no_label();

        // Act & Assert — `io` uses a different data model (see regen helper).
        for name in STANDARD_TEMPLATE_NAMES {
            let template = repo_template(name);
            let result = generate_pdf(
                &summary,
                &config,
                config.default_recipient(),
                Path::new("."),
                &template,
                crate::locale::Locale::EnUs,
            );
            assert!(
                result.is_ok(),
                "Template {name} should compile when payment method has no label, got: {:?}",
                result.err()
            );
        }
    }
}
