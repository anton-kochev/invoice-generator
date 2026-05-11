use std::path::PathBuf;

use tempfile::TempDir;

use super::mock_prompter::MockResponse;
use crate::config::ConfigError;
use crate::config::loader::LoadResult;
use crate::config::types::*;
use crate::config::writer::save_config;

// ── Synthetic Data Factories ──

pub fn synthetic_sender() -> Sender {
    Sender {
        key: None,
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
        key: Some(crate::domain::PaymentMethodKey::try_new("sepa-transfer").unwrap()),
        label: Some("SEPA Transfer".into()),
        iban: crate::domain::Iban::try_new("DE89370400440532013000")
            .expect("synthetic IBAN must be valid"),
        bic_swift: "COBADEFFXXX".into(),
    }]
}

pub fn synthetic_presets() -> Vec<Preset> {
    vec![Preset {
        key: crate::domain::PresetKey::try_new("dev").unwrap(),
        description: "Development Services".into(),
        default_rate: 100.0,
        currency: None,
        tax_rate: None,
    }]
}

pub fn synthetic_defaults() -> Defaults {
    Defaults {
        currency: crate::domain::Currency::Usd,
        invoice_date_day: 5,
        payment_terms_days: 14,
        // `amalthea` is one of the three bundled-into-the-binary templates,
        // so test fixtures can always render against it without depending on
        // the user having run `template refresh`.
        template: "amalthea".into(),
        locale: crate::locale::Locale::EnUs,
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
        senders: None,
        default_sender: None,
        payment: Some(synthetic_payment()),
        presets: Some(synthetic_presets()),
        defaults: Some(synthetic_defaults()),
        branding: None,
    }
}

pub fn config_with_two_presets() -> Config {
    let mut cfg = complete_config();
    cfg.presets = Some(vec![
        Preset {
            key: crate::domain::PresetKey::try_new("dev").unwrap(),
            description: "Development Services".into(),
            default_rate: 100.0,
            currency: None,
            tax_rate: None,
        },
        Preset {
            key: crate::domain::PresetKey::try_new("design").unwrap(),
            description: "Design Work".into(),
            default_rate: 80.0,
            currency: None,
            tax_rate: None,
        },
    ]);
    cfg
}

// ── v2 Config Factories ──

pub fn synthetic_recipient_acme() -> Recipient {
    Recipient {
        key: Some(crate::domain::RecipientKey::try_new("acme").unwrap()),
        name: "Acme Corp".into(),
        address: vec!["100 Acme Blvd".into(), "Metropolis, IL 62960".into()],
        company_id: Some("AC-12345".into()),
        vat_number: None,
    }
}

pub fn synthetic_recipient_globex() -> Recipient {
    Recipient {
        key: Some(crate::domain::RecipientKey::try_new("globex").unwrap()),
        name: "Globex Inc".into(),
        address: vec!["200 Globex Ave".into()],
        company_id: None,
        vat_number: Some("CZ87654321".into()),
    }
}

/// Synthetic [`ValidatedRecipient`] mirroring [`synthetic_recipient_acme`] for
/// tests that exercise validated-only call sites (e.g. `format_recipient_table`).
pub fn synthetic_validated_acme() -> crate::config::validator::ValidatedRecipient {
    crate::config::validator::ValidatedRecipient::from_validated_parts(
        crate::domain::RecipientKey::try_new("acme").unwrap(),
        "Acme Corp".into(),
        vec!["100 Acme Blvd".into(), "Metropolis, IL 62960".into()],
        Some("AC-12345".into()),
        None,
    )
}

/// Synthetic [`ValidatedRecipient`] mirroring [`synthetic_recipient_globex`].
pub fn synthetic_validated_globex() -> crate::config::validator::ValidatedRecipient {
    crate::config::validator::ValidatedRecipient::from_validated_parts(
        crate::domain::RecipientKey::try_new("globex").unwrap(),
        "Globex Inc".into(),
        vec!["200 Globex Ave".into()],
        None,
        Some("CZ87654321".into()),
    )
}

/// Synthetic [`ValidatedSender`] for tests that exercise validated-only call
/// sites (e.g. `select_sender`, `format_sender_table`). Mirrors the
/// `synthetic_validated_acme` recipient fixture.
pub fn synthetic_validated_alice() -> crate::config::validator::ValidatedSender {
    crate::config::validator::ValidatedSender::from_validated_parts(
        crate::domain::SenderKey::try_new("alice").unwrap(),
        "Alice Smith".into(),
        vec!["42 Elm St".into(), "Springfield, IL 62704".into()],
        "alice@example.com".into(),
    )
}

/// Synthetic [`ValidatedSender`] mirroring [`synthetic_validated_globex`] for
/// the second-sender slot.
pub fn synthetic_validated_bob() -> crate::config::validator::ValidatedSender {
    crate::config::validator::ValidatedSender::from_validated_parts(
        crate::domain::SenderKey::try_new("bob").unwrap(),
        "Bob Jones".into(),
        vec!["100 Oak Ln".into()],
        "bob@example.com".into(),
    )
}

/// Synthetic v2-keyed [`Sender`] mirroring [`synthetic_validated_alice`].
pub fn synthetic_sender_alice() -> Sender {
    Sender {
        key: Some(crate::domain::SenderKey::try_new("alice").unwrap()),
        name: "Alice Smith".into(),
        address: vec!["42 Elm St".into(), "Springfield, IL 62704".into()],
        email: "alice@example.com".into(),
    }
}

/// Synthetic v2-keyed [`Sender`] mirroring [`synthetic_validated_bob`].
pub fn synthetic_sender_bob() -> Sender {
    Sender {
        key: Some(crate::domain::SenderKey::try_new("bob").unwrap()),
        name: "Bob Jones".into(),
        address: vec!["100 Oak Ln".into()],
        email: "bob@example.com".into(),
    }
}

/// v2 config with exactly one keyed sender (`alice`), recipient `acme` default.
pub fn v2_complete_config_with_senders() -> Config {
    Config {
        sender: None,
        recipient: None,
        recipients: Some(vec![synthetic_recipient_acme()]),
        default_recipient: Some(crate::domain::RecipientKey::try_new("acme").unwrap()),
        senders: Some(vec![synthetic_sender_alice()]),
        default_sender: Some(crate::domain::SenderKey::try_new("alice").unwrap()),
        payment: Some(synthetic_payment()),
        presets: Some(synthetic_presets()),
        defaults: Some(synthetic_defaults()),
        branding: None,
    }
}

/// v2 config with two keyed senders (`alice` default, `bob` extra), recipient
/// `acme` default. Mirrors [`v2_config_two_recipients`].
pub fn v2_config_two_senders() -> Config {
    Config {
        sender: None,
        recipient: None,
        recipients: Some(vec![synthetic_recipient_acme()]),
        default_recipient: Some(crate::domain::RecipientKey::try_new("acme").unwrap()),
        senders: Some(vec![synthetic_sender_alice(), synthetic_sender_bob()]),
        default_sender: Some(crate::domain::SenderKey::try_new("alice").unwrap()),
        payment: Some(synthetic_payment()),
        presets: Some(synthetic_presets()),
        defaults: Some(synthetic_defaults()),
        branding: None,
    }
}

pub fn v2_complete_config() -> Config {
    Config {
        sender: Some(synthetic_sender()),
        recipient: None,
        recipients: Some(vec![synthetic_recipient_acme()]),
        default_recipient: Some(crate::domain::RecipientKey::try_new("acme").unwrap()),
        senders: None,
        default_sender: None,
        payment: Some(synthetic_payment()),
        presets: Some(synthetic_presets()),
        defaults: Some(synthetic_defaults()),
        branding: None,
    }
}

pub fn v2_config_two_recipients() -> Config {
    Config {
        sender: Some(synthetic_sender()),
        recipient: None,
        recipients: Some(vec![
            synthetic_recipient_acme(),
            synthetic_recipient_globex(),
        ]),
        default_recipient: Some(crate::domain::RecipientKey::try_new("acme").unwrap()),
        senders: None,
        default_sender: None,
        payment: Some(synthetic_payment()),
        presets: Some(synthetic_presets()),
        defaults: Some(synthetic_defaults()),
        branding: None,
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

/// Test-only indirection over `ValidatedConfig`'s sender.
///
/// Step 5a of the sender-CRUD plan introduced this helper so that step 5b's
/// field-swap — `ValidatedConfig.sender: Sender` →
/// `senders: NonEmpty<ValidatedSender>` + `default_sender_idx` — only needs
/// to change this function's body rather than every test assertion that
/// reads `v.sender.*`. The return type is now `&ValidatedSender`; the
/// `.name`, `.address`, `.email` field-access pattern in tests still works
/// because those fields are `pub(super)` and the helper lives in the same
/// crate.
pub(crate) fn sender_for_test(
    v: &crate::config::validator::ValidatedConfig,
) -> &crate::config::validator::ValidatedSender {
    v.default_sender()
}

// ── Tempdir Helper ──

/// Path to the config file inside a tempdir. Loader/writer functions take a
/// file path (not a directory), so tests use this helper instead of bare
/// `dir.path()`.
pub fn cfg_path(dir: &TempDir) -> PathBuf {
    dir.path().join("config.yaml")
}

pub fn setup_dir(config: Option<&Config>) -> TempDir {
    let dir = TempDir::new().unwrap();
    if let Some(cfg) = config {
        save_config(&cfg_path(&dir), cfg).unwrap();
    }
    dir
}

/// Extract Config from a LoadResult, panicking on NotFound.
pub fn unwrap_loaded(result: Result<LoadResult, ConfigError>) -> Config {
    match result.unwrap() {
        LoadResult::Loaded(c) => *c,
        LoadResult::NotFound => panic!("Expected Loaded, got NotFound"),
    }
}

// ── Mock Response Queues ──

/// Full mock response queue for a complete setup run.
/// Sender + Recipient + Payment(1) + Presets(1) + Defaults + Branding
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
        MockResponse::Text("sepa-transfer".into()),
        MockResponse::OptionalText(Some("SEPA Transfer".into())),
        MockResponse::Text("DE89370400440532013000".into()),
        MockResponse::Text("COBADEFFXXX".into()),
        // Presets (1 preset, decline more)
        MockResponse::Text("dev".into()),
        MockResponse::Text("Development Services".into()),
        MockResponse::F64(100.0),
        MockResponse::OptionalText(None),
        MockResponse::Confirm(false),
        // Defaults
        MockResponse::Text("EUR".into()),
        MockResponse::U32(9),
        MockResponse::U32(30),
        MockResponse::Text("leda".into()),  // template
        MockResponse::Text("en-US".into()), // locale
        // Branding (decline custom footer)
        MockResponse::OptionalText(None),
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
        MockResponse::Text("sepa".into()),
        MockResponse::OptionalText(Some("SEPA".into())),
        MockResponse::Text("DE89370400440532013000".into()),
        MockResponse::Text("BIC".into()),
        // Presets
        MockResponse::Text("dev".into()),
        MockResponse::Text("Dev".into()),
        MockResponse::F64(100.0),
        MockResponse::OptionalText(None),
        MockResponse::Confirm(false),
        // Defaults
        MockResponse::Text("EUR".into()),
        MockResponse::U32(9),
        MockResponse::U32(30),
        MockResponse::Text("leda".into()),  // template
        MockResponse::Text("en-US".into()), // locale
        // Branding (decline custom footer)
        MockResponse::OptionalText(None),
    ]
}
