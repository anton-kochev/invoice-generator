use serde::Serialize;

use crate::config::validator::ValidatedConfig;
use crate::invoice::types::InvoiceSummary;
use crate::locale::Locale;

/// All data needed to render the invoice PDF template.
#[derive(Debug, Serialize)]
pub struct InvoiceData<'a> {
    pub sender: SenderData<'a>,
    pub recipient: RecipientData<'a>,
    pub invoice: InvoiceInfo,
    pub payment: Vec<PaymentData<'a>>,
    pub branding: BrandingData,
}

/// Sender information for the template.
#[derive(Debug, Serialize)]
pub struct SenderData<'a> {
    pub name: &'a str,
    pub address: &'a [String],
    pub email: &'a str,
}

/// Recipient information for the template.
#[derive(Debug, Serialize)]
pub struct RecipientData<'a> {
    pub name: &'a str,
    pub address: &'a [String],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub company_id: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vat_number: Option<&'a str>,
}

/// Core invoice metadata and line items.
#[derive(Debug, Serialize)]
pub struct InvoiceInfo {
    pub number: String,
    pub period: String,
    pub date: String,
    pub due_date: String,
    pub currency: String,
    pub line_items: Vec<LineItemData>,
    pub has_tax: bool,
    pub subtotal: String,
    pub tax_total: String,
    pub total: String,
}

/// A single line item formatted as strings for the template.
#[derive(Debug, Serialize)]
pub struct LineItemData {
    pub description: String,
    pub days: String,
    pub rate: String,
    pub amount: String,
    pub tax_rate: String,
    pub tax_amount: String,
}

/// Payment method details for the template.
///
/// `iban` is rendered as the canonical grouped form (`GB82 WEST 1234 5698 7654 32`)
/// for human readability in the PDF.
#[derive(Debug, Serialize)]
pub struct PaymentData<'a> {
    pub label: &'a str,
    pub iban: String,
    pub bic_swift: &'a str,
}

/// Default font fallback chain matching v1.0 styling.
const DEFAULT_FONTS: &[&str] = &["Helvetica", "Noto Sans", "Liberation Sans"];

/// Branding data for the template.
#[derive(Debug, Serialize)]
pub struct BrandingData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logo_file: Option<String>,
    pub accent_color: String,
    pub font: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub footer_text: Option<String>,
}

impl<'a> InvoiceData<'a> {
    /// Build template data from a computed summary and validated config.
    pub fn from_parts(
        summary: &'a InvoiceSummary,
        config: &'a ValidatedConfig,
        recipient: &'a crate::config::types::Recipient,
        logo_file: Option<String>,
        locale: Locale,
    ) -> Self {
        let period_month =
            time::Month::try_from(summary.period.month() as u8).expect("valid month 1..=12");
        let period_year = summary.period.year() as i32;

        Self {
            sender: SenderData {
                name: &config.sender.name,
                address: &config.sender.address,
                email: &config.sender.email,
            },
            recipient: RecipientData {
                name: &recipient.name,
                address: &recipient.address,
                company_id: recipient.company_id.as_deref(),
                vat_number: recipient.vat_number.as_deref(),
            },
            invoice: InvoiceInfo {
                number: summary.invoice_number.clone(),
                period: locale.format_period(period_month, period_year),
                date: locale.format_date(summary.invoice_date),
                due_date: locale.format_date(summary.due_date),
                currency: summary.currency.clone(),
                line_items: summary
                    .line_items
                    .iter()
                    .map(|item| LineItemData {
                        description: item.description.clone(),
                        days: locale.format_number(item.days, 2),
                        rate: locale.format_number(item.rate, 2),
                        amount: locale.format_number(item.amount, 2),
                        tax_rate: if item.tax_rate > 0.0 {
                            locale.format_number(item.tax_rate, 1)
                        } else {
                            "0".to_string()
                        },
                        tax_amount: if item.tax_rate > 0.0 {
                            locale.format_number(item.tax_amount, 2)
                        } else {
                            "\u{2013}".to_string()
                        },
                    })
                    .collect(),
                has_tax: summary.line_items.iter().any(|i| i.tax_rate > 0.0),
                subtotal: locale.format_number(summary.subtotal, 2),
                tax_total: locale.format_number(summary.tax_total, 2),
                total: locale.format_number(summary.total, 2),
            },
            payment: config
                .payment
                .iter()
                .map(|p| PaymentData {
                    label: &p.label,
                    iban: p.iban.to_string(),
                    bic_swift: &p.bic_swift,
                })
                .collect(),
            branding: BrandingData {
                logo_file,
                accent_color: config.branding.accent_color.as_str().to_string(),
                font: config
                    .branding
                    .font
                    .iter()
                    .cloned()
                    .chain(DEFAULT_FONTS.iter().map(|s| (*s).to_string()))
                    .collect(),
                footer_text: config.branding.footer_text.clone(),
            },
        }
    }
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
            currency: "EUR".into(),
            line_items: vec![
                LineItem::new("Software development".into(), 10.0, 800.0, "EUR".into()),
                LineItem::new("Technical consulting".into(), 5.0, 1000.0, "EUR".into()),
            ],
            subtotal: 13000.0,
            tax_total: 0.0,
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
                iban: crate::domain::Iban::try_new("DE89 3704 0044 0532 0130 00").unwrap(),
                bic_swift: "COBADEFFXXX".into(),
            }],
            presets: vec![Preset {
                key: "dev".into(),
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
    fn from_parts_serializes_to_valid_json() {
        // Arrange
        let summary = make_summary();
        let config = make_config();

        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient, None, Locale::EnUs);
        let json = serde_json::to_value(&data).unwrap();

        // Assert
        assert_eq!(json["invoice"]["number"], "INV-2026-03");
        assert_eq!(json["invoice"]["period"], "March 2026");
        assert_eq!(json["invoice"]["currency"], "EUR");
        assert_eq!(json["invoice"]["total"], "13,000.00");
        assert_eq!(json["sender"]["name"], "Jane Doe");
        assert_eq!(json["recipient"]["name"], "Acme Corp");
        assert_eq!(json["recipient"]["company_id"], "DE123456");
    }

    #[test]
    fn line_items_formatted_as_strings() {
        // Arrange
        let summary = make_summary();
        let config = make_config();

        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient, None, Locale::EnUs);
        let json = serde_json::to_value(&data).unwrap();

        // Assert
        let items = json["invoice"]["line_items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["days"], "10.00");
        assert_eq!(items[0]["rate"], "800.00");
        assert_eq!(items[0]["amount"], "8,000.00");
    }

    #[test]
    fn payment_methods_included() {
        // Arrange
        let summary = make_summary();
        let config = make_config();

        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient, None, Locale::EnUs);
        let json = serde_json::to_value(&data).unwrap();

        // Assert
        let payments = json["payment"].as_array().unwrap();
        assert_eq!(payments.len(), 1);
        assert_eq!(payments[0]["label"], "Primary Bank Account");
        assert_eq!(payments[0]["iban"], "DE89 3704 0044 0532 0130 00");
    }

    fn make_summary_with_tax() -> InvoiceSummary {
        InvoiceSummary {
            invoice_number: "INV-2026-03".into(),
            period: InvoicePeriod::new(3, 2026).unwrap(),
            invoice_date: Date::from_calendar_date(2026, Month::April, 9).unwrap(),
            due_date: Date::from_calendar_date(2026, Month::May, 9).unwrap(),
            currency: "EUR".into(),
            line_items: vec![
                LineItem::with_tax(
                    "Software development".into(),
                    10.0,
                    800.0,
                    "EUR".into(),
                    21.0,
                ),
            ],
            subtotal: 8000.0,
            tax_total: 1680.0,
            total: 9680.0,
        }
    }

    #[test]
    fn from_parts_line_item_includes_tax_rate_string() {
        // Arrange
        let summary = make_summary_with_tax();
        let config = make_config();

        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient, None, Locale::EnUs);

        // Assert
        assert_eq!(data.invoice.line_items[0].tax_rate, "21.0");
    }

    #[test]
    fn from_parts_line_item_includes_tax_amount_string() {
        // Arrange
        let summary = make_summary_with_tax();
        let config = make_config();

        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient, None, Locale::EnUs);

        // Assert
        assert_eq!(data.invoice.line_items[0].tax_amount, "1,680.00");
    }

    #[test]
    fn from_parts_zero_tax_item_uses_dash_for_tax_amount() {
        // Arrange
        let summary = make_summary();
        let config = make_config();

        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient, None, Locale::EnUs);

        // Assert
        assert_eq!(data.invoice.line_items[0].tax_rate, "0");
        assert_eq!(data.invoice.line_items[0].tax_amount, "\u{2013}");
    }

    #[test]
    fn from_parts_invoice_has_tax_true_when_any_item_taxed() {
        // Arrange
        let summary = make_summary_with_tax();
        let config = make_config();

        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient, None, Locale::EnUs);

        // Assert
        assert!(data.invoice.has_tax);
    }

    #[test]
    fn from_parts_invoice_has_tax_false_when_all_zero_tax() {
        // Arrange
        let summary = make_summary();
        let config = make_config();

        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient, None, Locale::EnUs);

        // Assert
        assert!(!data.invoice.has_tax);
    }

    #[test]
    fn from_parts_invoice_includes_subtotal_and_tax_total() {
        // Arrange
        let summary = make_summary_with_tax();
        let config = make_config();

        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient, None, Locale::EnUs);

        // Assert
        assert_eq!(data.invoice.subtotal, "8,000.00");
        assert_eq!(data.invoice.tax_total, "1,680.00");
    }

    #[test]
    fn optional_fields_omitted_when_none() {
        // Arrange
        let summary = make_summary();
        let mut config = make_config();
        config.recipient.company_id = None;
        config.recipient.vat_number = None;

        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient, None, Locale::EnUs);
        let json = serde_json::to_value(&data).unwrap();

        // Assert
        assert!(json["recipient"].get("company_id").is_none());
        assert!(json["recipient"].get("vat_number").is_none());
    }

    // ── Sprint 10: BrandingData tests ──

    #[test]
    fn test_from_parts_includes_branding_with_defaults() {
        // Arrange
        let summary = make_summary();
        let config = make_config();
        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient, None, Locale::EnUs);
        let json = serde_json::to_value(&data).unwrap();
        // Assert
        assert_eq!(json["branding"]["accent_color"], "#2c3e50");
        let fonts = json["branding"]["font"].as_array().unwrap();
        assert_eq!(fonts.len(), 3);
        assert_eq!(fonts[0], "Helvetica");
    }

    #[test]
    fn test_from_parts_branding_custom_accent_color() {
        // Arrange
        let summary = make_summary();
        let mut config = make_config();
        config.branding.accent_color = crate::domain::HexColor::try_new("#ff0000").unwrap();
        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient, None, Locale::EnUs);
        let json = serde_json::to_value(&data).unwrap();
        // Assert
        assert_eq!(json["branding"]["accent_color"], "#ff0000");
    }

    #[test]
    fn test_from_parts_branding_custom_font_prepended() {
        // Arrange
        let summary = make_summary();
        let mut config = make_config();
        config.branding.font = Some("Fira Code".into());
        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient, None, Locale::EnUs);
        let json = serde_json::to_value(&data).unwrap();
        // Assert
        let fonts = json["branding"]["font"].as_array().unwrap();
        assert_eq!(fonts.len(), 4);
        assert_eq!(fonts[0], "Fira Code");
        assert_eq!(fonts[1], "Helvetica");
    }

    #[test]
    fn test_from_parts_branding_no_custom_font_uses_defaults() {
        // Arrange
        let summary = make_summary();
        let config = make_config(); // font: None
        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient, None, Locale::EnUs);
        // Assert
        assert_eq!(
            data.branding.font,
            vec!["Helvetica", "Noto Sans", "Liberation Sans"]
        );
    }

    #[test]
    fn test_from_parts_branding_footer_text_included() {
        // Arrange
        let summary = make_summary();
        let mut config = make_config();
        config.branding.footer_text = Some("Thanks!".into());
        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient, None, Locale::EnUs);
        let json = serde_json::to_value(&data).unwrap();
        // Assert
        assert_eq!(json["branding"]["footer_text"], "Thanks!");
    }

    #[test]
    fn test_from_parts_branding_footer_text_none_omitted() {
        // Arrange
        let summary = make_summary();
        let config = make_config(); // footer_text: None
        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient, None, Locale::EnUs);
        let json = serde_json::to_value(&data).unwrap();
        // Assert
        assert!(json["branding"].get("footer_text").is_none());
    }

    #[test]
    fn test_from_parts_branding_logo_file_included() {
        // Arrange
        let summary = make_summary();
        let config = make_config();
        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient, Some("logo.png".into()), Locale::EnUs);
        let json = serde_json::to_value(&data).unwrap();
        // Assert
        assert_eq!(json["branding"]["logo_file"], "logo.png");
    }

    #[test]
    fn test_from_parts_branding_logo_file_none_omitted() {
        // Arrange
        let summary = make_summary();
        let config = make_config();
        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient, None, Locale::EnUs);
        let json = serde_json::to_value(&data).unwrap();
        // Assert
        assert!(json["branding"].get("logo_file").is_none());
    }

    // ── Story 13.2: locale-aware formatting in from_parts ──

    #[test]
    fn test_from_parts_de_de_period_is_german() {
        // Arrange
        let summary = make_summary();
        let config = make_config();

        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient, None, Locale::DeDe);

        // Assert
        assert_eq!(data.invoice.period, "März 2026");
    }

    #[test]
    fn test_from_parts_de_de_dates_are_german() {
        // Arrange
        let summary = make_summary();
        let config = make_config();

        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient, None, Locale::DeDe);

        // Assert
        assert_eq!(data.invoice.date, "9. April 2026");
        assert_eq!(data.invoice.due_date, "9. Mai 2026");
    }

    #[test]
    fn test_from_parts_de_de_numbers_use_comma() {
        // Arrange
        let summary = make_summary();
        let config = make_config();

        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient, None, Locale::DeDe);

        // Assert
        assert_eq!(data.invoice.total, "13.000,00");
        assert_eq!(data.invoice.line_items[0].rate, "800,00");
        assert_eq!(data.invoice.line_items[0].amount, "8.000,00");
    }
}
