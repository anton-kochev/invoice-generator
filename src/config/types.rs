use serde::{Deserialize, Serialize};

/// Top-level invoice generator configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Config {
    /// Sender / freelancer info.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sender: Option<Sender>,
    /// Default recipient / client info.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recipient: Option<Recipient>,
    /// Available payment methods.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment: Option<Vec<PaymentMethod>>,
    /// Invoice presets (e.g. hourly-rate templates).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub presets: Option<Vec<Preset>>,
    /// Default values for new invoices.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defaults: Option<Defaults>,
}

/// Information about the invoice sender.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Sender {
    pub name: String,
    pub address: Vec<String>,
    pub email: String,
}

/// Information about the invoice recipient.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Recipient {
    pub name: String,
    pub address: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub company_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none", alias = "vat")]
    pub vat_number: Option<String>,
}

/// A payment method shown on the invoice.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PaymentMethod {
    pub label: String,
    pub iban: String,
    #[serde(alias = "bic")]
    pub bic_swift: String,
}

/// An invoice preset / template.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Preset {
    pub key: String,
    pub description: String,
    pub default_rate: f64,
}

fn default_currency() -> String {
    "EUR".to_string()
}

const fn default_invoice_date_day() -> u32 {
    9
}

const fn default_payment_terms_days() -> u32 {
    30
}

/// Default values applied to new invoices.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Defaults {
    #[serde(default = "default_currency")]
    pub currency: String,
    #[serde(default = "default_invoice_date_day")]
    pub invoice_date_day: u32,
    #[serde(default = "default_payment_terms_days")]
    pub payment_terms_days: u32,
}

impl Default for Defaults {
    fn default() -> Self {
        Self {
            currency: default_currency(),
            invoice_date_day: default_invoice_date_day(),
            payment_terms_days: default_payment_terms_days(),
        }
    }
}
