use tempfile::TempDir;

use crate::config::loader::LoadResult;
use crate::config::types::*;
use crate::config::writer::save_config;
use crate::error::AppError;
use super::mock_prompter::MockResponse;

// ── Synthetic Data Factories ──

pub fn synthetic_sender() -> Sender {
    Sender {
        name: "Alice Smith".into(),
        address: vec!["42 Elm Street".into(), "Springfield, IL 62704".into()],
        email: "alice@example.com".into(),
    }
}

pub fn synthetic_recipient() -> Recipient {
    Recipient {
        key: None,
        name: "Bob Corp".into(),
        address: vec!["99 Oak Lane".into()],
        company_id: Some("BC-98765".into()),
        vat_number: Some("CZ12345678".into()),
    }
}

pub fn synthetic_payment() -> Vec<PaymentMethod> {
    vec![PaymentMethod {
        label: "SEPA Transfer".into(),
        iban: "DE89370400440532013000".into(),
        bic_swift: "COBADEFFXXX".into(),
    }]
}

pub fn synthetic_presets() -> Vec<Preset> {
    vec![Preset {
        key: "dev".into(),
        description: "Development Services".into(),
        default_rate: 100.0,
        currency: None,
    }]
}

pub fn synthetic_defaults() -> Defaults {
    Defaults {
        currency: "USD".into(),
        invoice_date_day: 5,
        payment_terms_days: 14,
    }
}

pub fn empty_config() -> Config {
    Config::default()
}

pub fn config_with_sender() -> Config {
    Config {
        sender: Some(synthetic_sender()),
        ..Config::default()
    }
}

pub fn complete_config() -> Config {
    Config {
        sender: Some(synthetic_sender()),
        recipient: Some(synthetic_recipient()),
        recipients: None,
        default_recipient: None,
        payment: Some(synthetic_payment()),
        presets: Some(synthetic_presets()),
        defaults: Some(synthetic_defaults()),
    }
}

pub fn config_with_two_presets() -> Config {
    let mut cfg = complete_config();
    cfg.presets = Some(vec![
        Preset {
            key: "dev".into(),
            description: "Development Services".into(),
            default_rate: 100.0,
            currency: None,
        },
        Preset {
            key: "design".into(),
            description: "Design Work".into(),
            default_rate: 80.0,
            currency: None,
        },
    ]);
    cfg
}

// ── v2 Config Factories ──

pub fn synthetic_recipient_with_key(key: &str, name: &str) -> Recipient {
    Recipient {
        key: Some(key.into()),
        name: name.into(),
        address: vec!["123 Synthetic St".into()],
        company_id: Some(format!("{}-ID", key.to_uppercase())),
        vat_number: None,
    }
}

pub fn synthetic_recipient_acme() -> Recipient {
    Recipient {
        key: Some("acme".into()),
        name: "Acme Corp".into(),
        address: vec!["100 Acme Blvd".into(), "Metropolis, IL 62960".into()],
        company_id: Some("AC-12345".into()),
        vat_number: None,
    }
}

pub fn synthetic_recipient_globex() -> Recipient {
    Recipient {
        key: Some("globex".into()),
        name: "Globex Inc".into(),
        address: vec!["200 Globex Ave".into()],
        company_id: None,
        vat_number: Some("CZ87654321".into()),
    }
}

pub fn v2_complete_config() -> Config {
    Config {
        sender: Some(synthetic_sender()),
        recipient: None,
        recipients: Some(vec![synthetic_recipient_acme()]),
        default_recipient: Some("acme".into()),
        payment: Some(synthetic_payment()),
        presets: Some(synthetic_presets()),
        defaults: Some(synthetic_defaults()),
    }
}

pub fn v2_config_two_recipients() -> Config {
    Config {
        sender: Some(synthetic_sender()),
        recipient: None,
        recipients: Some(vec![synthetic_recipient_acme(), synthetic_recipient_globex()]),
        default_recipient: Some("acme".into()),
        payment: Some(synthetic_payment()),
        presets: Some(synthetic_presets()),
        defaults: Some(synthetic_defaults()),
    }
}

pub fn validated(config: Config) -> crate::config::validator::ValidatedConfig {
    use crate::config::validator::ValidationOutcome;
    match config.validate().unwrap() {
        ValidationOutcome::Complete(v) => v,
        ValidationOutcome::Incomplete { missing, .. } => {
            panic!("Expected Complete, got Incomplete with missing: {missing:?}")
        }
    }
}

// ── Tempdir Helper ──

pub fn setup_dir(config: Option<&Config>) -> TempDir {
    let dir = TempDir::new().unwrap();
    if let Some(cfg) = config {
        save_config(dir.path(), cfg).unwrap();
    }
    dir
}

/// Extract Config from a LoadResult, panicking on NotFound.
pub fn unwrap_loaded(result: Result<LoadResult, AppError>) -> Config {
    match result.unwrap() {
        LoadResult::Loaded(c) => c,
        LoadResult::NotFound => panic!("Expected Loaded, got NotFound"),
    }
}

// ── Mock Response Queues ──

/// Full mock response queue for a complete setup run.
/// Sender + Recipient + Payment(1) + Presets(1) + Defaults
pub fn full_setup_responses() -> Vec<MockResponse> {
    vec![
        // Sender
        MockResponse::Text("Alice Smith".into()),
        MockResponse::Lines(vec!["42 Elm St".into()]),
        MockResponse::Text("alice@example.com".into()),
        // Recipient
        MockResponse::Text("bob".into()),
        MockResponse::Text("Bob Corp".into()),
        MockResponse::Lines(vec!["99 Oak Lane".into()]),
        MockResponse::OptionalText(None),
        MockResponse::OptionalText(None),
        // Payment (1 method)
        MockResponse::U32(1),
        MockResponse::Text("SEPA Transfer".into()),
        MockResponse::Text("DE89370400440532013000".into()),
        MockResponse::Text("COBADEFFXXX".into()),
        // Presets (1 preset, decline more)
        MockResponse::Text("dev".into()),
        MockResponse::Text("Development Services".into()),
        MockResponse::F64(100.0),
        MockResponse::Confirm(false),
        // Defaults
        MockResponse::Text("EUR".into()),
        MockResponse::U32(9),
        MockResponse::U32(30),
    ]
}

/// Mock responses for resuming from recipient onward.
pub fn resume_from_recipient_responses() -> Vec<MockResponse> {
    vec![
        // Recipient
        MockResponse::Text("bob".into()),
        MockResponse::Text("Bob Corp".into()),
        MockResponse::Lines(vec!["99 Oak Lane".into()]),
        MockResponse::OptionalText(None),
        MockResponse::OptionalText(None),
        // Payment
        MockResponse::U32(1),
        MockResponse::Text("SEPA".into()),
        MockResponse::Text("DE00".into()),
        MockResponse::Text("BIC".into()),
        // Presets
        MockResponse::Text("dev".into()),
        MockResponse::Text("Dev".into()),
        MockResponse::F64(100.0),
        MockResponse::Confirm(false),
        // Defaults
        MockResponse::Text("EUR".into()),
        MockResponse::U32(9),
        MockResponse::U32(30),
    ]
}
