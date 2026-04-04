mod data;
mod world;

use typst::layout::PagedDocument;

use crate::config::validator::ValidatedConfig;
use crate::error::AppError;
use crate::invoice::types::InvoiceSummary;

/// Generate a PDF from a computed invoice summary and validated config.
pub fn generate_pdf(
    summary: &InvoiceSummary,
    config: &ValidatedConfig,
    recipient: &crate::config::types::Recipient,
) -> Result<Vec<u8>, AppError> {
    let invoice_data = data::InvoiceData::from_parts(summary, config, recipient);

    let json = serde_json::to_vec(&invoice_data)
        .map_err(|e| AppError::PdfCompile(format!("JSON serialization failed: {e}")))?;

    let world = world::InvoiceWorld::new(json);

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
    use crate::invoice::types::*;
    use time::{Date, Month};

    fn make_summary() -> InvoiceSummary {
        InvoiceSummary {
            invoice_number: "INV-2026-03".into(),
            period: InvoicePeriod::new(3, 2026).unwrap(),
            invoice_date: Date::from_calendar_date(2026, Month::April, 9).unwrap(),
            due_date: Date::from_calendar_date(2026, Month::May, 9).unwrap(),
            currency: "EUR".into(),
            line_items: vec![
                LineItem::new("Software development".into(), 10.0, 800.0),
                LineItem::new("Technical consulting".into(), 5.0, 1000.0),
            ],
            total: 13000.0,
        }
    }

    fn make_config() -> ValidatedConfig {
        let recipient = Recipient {
            key: Some("acme-corp".into()),
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
            default_recipient_key: "acme-corp".into(),
            payment: vec![PaymentMethod {
                label: "Primary Bank Account".into(),
                iban: "DE89 3704 0044 0532 0130 00".into(),
                bic_swift: "COBADEFFXXX".into(),
            }],
            presets: vec![Preset {
                key: "dev".into(),
                description: "Software development".into(),
                default_rate: 800.0,
            }],
            defaults: Defaults::default(),
        }
    }

    #[test]
    fn generate_pdf_returns_valid_pdf() {
        // Arrange
        let summary = make_summary();
        let config = make_config();

        // Act
        let result = generate_pdf(&summary, &config, &config.recipient);

        // Assert
        let pdf = result.expect("PDF generation should succeed");
        assert!(pdf.starts_with(b"%PDF"), "Output should start with PDF header");
    }

    #[test]
    fn generate_pdf_returns_nonempty() {
        // Arrange
        let summary = make_summary();
        let config = make_config();

        // Act
        let pdf = generate_pdf(&summary, &config, &config.recipient).unwrap();

        // Assert
        assert!(pdf.len() > 100, "PDF should have substantial content");
    }

    #[test]
    fn generate_pdf_deterministic() {
        // Arrange
        let summary = make_summary();
        let config = make_config();

        // Act
        let pdf1 = generate_pdf(&summary, &config, &config.recipient).unwrap();
        let pdf2 = generate_pdf(&summary, &config, &config.recipient).unwrap();

        // Assert
        assert_eq!(pdf1, pdf2, "Same input should produce identical PDF bytes");
    }
}
