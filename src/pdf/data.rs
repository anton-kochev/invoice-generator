use serde::Serialize;

use crate::config::validator::ValidatedConfig;
use crate::invoice::types::InvoiceSummary;

/// All data needed to render the invoice PDF template.
#[derive(Debug, Serialize)]
pub struct InvoiceData<'a> {
    pub sender: SenderData<'a>,
    pub recipient: RecipientData<'a>,
    pub invoice: InvoiceInfo,
    pub payment: Vec<PaymentData<'a>>,
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
#[derive(Debug, Serialize)]
pub struct PaymentData<'a> {
    pub label: &'a str,
    pub iban: &'a str,
    pub bic_swift: &'a str,
}

impl<'a> InvoiceData<'a> {
    /// Build template data from a computed summary and validated config.
    pub fn from_parts(
        summary: &'a InvoiceSummary,
        config: &'a ValidatedConfig,
        recipient: &'a crate::config::types::Recipient,
    ) -> Self {
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
                period: summary.period.display_long(),
                date: summary.invoice_date.to_string(),
                due_date: summary.due_date.to_string(),
                currency: summary.currency.clone(),
                line_items: summary
                    .line_items
                    .iter()
                    .map(|item| LineItemData {
                        description: item.description.clone(),
                        days: format!("{:.2}", item.days),
                        rate: format!("{:.2}", item.rate),
                        amount: format!("{:.2}", item.amount),
                        tax_rate: if item.tax_rate > 0.0 {
                            format!("{:.1}", item.tax_rate)
                        } else {
                            "0".to_string()
                        },
                        tax_amount: if item.tax_rate > 0.0 {
                            format!("{:.2}", item.tax_amount)
                        } else {
                            "\u{2013}".to_string()
                        },
                    })
                    .collect(),
                has_tax: summary.line_items.iter().any(|i| i.tax_rate > 0.0),
                subtotal: format!("{:.2}", summary.subtotal),
                tax_total: format!("{:.2}", summary.tax_total),
                total: format!("{:.2}", summary.total),
            },
            payment: config
                .payment
                .iter()
                .map(|p| PaymentData {
                    label: &p.label,
                    iban: &p.iban,
                    bic_swift: &p.bic_swift,
                })
                .collect(),
        }
    }
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
                iban: "DE89 3704 0044 0532 0130 00".into(),
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
        }
    }

    #[test]
    fn from_parts_serializes_to_valid_json() {
        // Arrange
        let summary = make_summary();
        let config = make_config();

        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient);
        let json = serde_json::to_value(&data).unwrap();

        // Assert
        assert_eq!(json["invoice"]["number"], "INV-2026-03");
        assert_eq!(json["invoice"]["period"], "March 2026");
        assert_eq!(json["invoice"]["currency"], "EUR");
        assert_eq!(json["invoice"]["total"], "13000.00");
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
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient);
        let json = serde_json::to_value(&data).unwrap();

        // Assert
        let items = json["invoice"]["line_items"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0]["days"], "10.00");
        assert_eq!(items[0]["rate"], "800.00");
        assert_eq!(items[0]["amount"], "8000.00");
    }

    #[test]
    fn payment_methods_included() {
        // Arrange
        let summary = make_summary();
        let config = make_config();

        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient);
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
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient);

        // Assert
        assert_eq!(data.invoice.line_items[0].tax_rate, "21.0");
    }

    #[test]
    fn from_parts_line_item_includes_tax_amount_string() {
        // Arrange
        let summary = make_summary_with_tax();
        let config = make_config();

        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient);

        // Assert
        assert_eq!(data.invoice.line_items[0].tax_amount, "1680.00");
    }

    #[test]
    fn from_parts_zero_tax_item_uses_dash_for_tax_amount() {
        // Arrange
        let summary = make_summary();
        let config = make_config();

        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient);

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
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient);

        // Assert
        assert!(data.invoice.has_tax);
    }

    #[test]
    fn from_parts_invoice_has_tax_false_when_all_zero_tax() {
        // Arrange
        let summary = make_summary();
        let config = make_config();

        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient);

        // Assert
        assert!(!data.invoice.has_tax);
    }

    #[test]
    fn from_parts_invoice_includes_subtotal_and_tax_total() {
        // Arrange
        let summary = make_summary_with_tax();
        let config = make_config();

        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient);

        // Assert
        assert_eq!(data.invoice.subtotal, "8000.00");
        assert_eq!(data.invoice.tax_total, "1680.00");
    }

    #[test]
    fn optional_fields_omitted_when_none() {
        // Arrange
        let summary = make_summary();
        let mut config = make_config();
        config.recipient.company_id = None;
        config.recipient.vat_number = None;

        // Act
        let data = InvoiceData::from_parts(&summary, &config, &config.recipient);
        let json = serde_json::to_value(&data).unwrap();

        // Assert
        assert!(json["recipient"].get("company_id").is_none());
        assert!(json["recipient"].get("vat_number").is_none());
    }
}
