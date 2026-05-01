use std::fmt;

use super::error::ConfigError;
use super::types::*;
use crate::domain::{HexColor, Iban, NonEmpty, PaymentMethodKey, RecipientKey};
use crate::locale::Locale;

const DEFAULT_ACCENT_COLOR: &str = "#2c3e50";

fn default_accent_color() -> HexColor {
    HexColor::try_new(DEFAULT_ACCENT_COLOR)
        .expect("DEFAULT_ACCENT_COLOR is a valid hex color literal")
}

/// Branding with validated values, ready for PDF generation.
///
/// As of the `HexColor` migration, `accent_color` is a parsed [`HexColor`]
/// (not a raw string). Invalid colors are now rejected at config-deserialize
/// time, so this struct is effectively a passthrough that fills in defaults
/// for missing fields.
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedBranding {
    /// Raw logo path from config (resolved to absolute path later in pdf module).
    pub logo: Option<String>,
    /// Validated hex color (`#rrggbb`, lowercase).
    pub accent_color: HexColor,
    /// Font family name override, or None for default.
    pub font: Option<String>,
    /// Custom footer text, or None for default.
    pub footer_text: Option<String>,
}

impl Default for ValidatedBranding {
    fn default() -> Self {
        Self {
            logo: None,
            accent_color: default_accent_color(),
            font: None,
            footer_text: None,
        }
    }
}

/// Identifies a top-level config section for validation reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfigSection {
    Sender,
    Recipient,
    Payment,
    Presets,
}

impl fmt::Display for ConfigSection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sender => write!(f, "sender"),
            Self::Recipient => write!(f, "recipient"),
            Self::Payment => write!(f, "payment"),
            Self::Presets => write!(f, "presets"),
        }
    }
}

/// A recipient with all post-validation invariants encoded in the type.
///
/// Compared to the raw [`Recipient`], `key` is non-`Option`: the validator
/// either supplies a derived key (v1 configs) or asserts the user-provided
/// one is well-formed. Eliminating the `Option` removes a swarm of
/// `.as_ref().is_some_and(...)` checks at every call site that already knows
/// the key must be present.
///
/// Fields are `pub(super)` so only the [`crate::config`] module can construct
/// instances directly. External callers — including tests in other modules —
/// must go through [`ValidatedRecipient::from_validated_parts`] (test-only) or
/// obtain instances by validating a [`Config`].
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedRecipient {
    pub(super) key: RecipientKey,
    pub(super) name: String,
    pub(super) address: Vec<String>,
    pub(super) company_id: Option<String>,
    pub(super) vat_number: Option<String>,
}

impl ValidatedRecipient {
    /// Convert from a raw [`Recipient`] whose `key` is known to be `Some`.
    ///
    /// Panics if `key` is `None`. Only the validator should call this — it is
    /// responsible for filling in derived keys before invoking `from_partial`.
    fn from_partial(r: Recipient) -> Self {
        Self {
            key: r.key.expect(
                "validator must populate Recipient.key before constructing ValidatedRecipient",
            ),
            name: r.name,
            address: r.address,
            company_id: r.company_id,
            vat_number: r.vat_number,
        }
    }

    /// Borrow the recipient's key.
    pub fn key(&self) -> &RecipientKey {
        &self.key
    }

    /// Borrow the recipient's display name.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Borrow the recipient's address lines.
    pub fn address(&self) -> &[String] {
        &self.address
    }

    /// Borrow the recipient's company ID, if present.
    pub fn company_id(&self) -> Option<&str> {
        self.company_id.as_deref()
    }

    /// Borrow the recipient's VAT number, if present.
    pub fn vat_number(&self) -> Option<&str> {
        self.vat_number.as_deref()
    }

    /// Test-only constructor that bypasses the normal validator path.
    ///
    /// Used by test fixtures in other modules that need to assemble a
    /// [`ValidatedRecipient`] without round-tripping through [`Config::validate`].
    /// Production code paths construct via [`ValidatedRecipient::from_partial`]
    /// inside the validator.
    #[cfg(test)]
    pub(crate) fn from_validated_parts(
        key: RecipientKey,
        name: String,
        address: Vec<String>,
        company_id: Option<String>,
        vat_number: Option<String>,
    ) -> Self {
        Self {
            key,
            name,
            address,
            company_id,
            vat_number,
        }
    }
}

/// A payment method with all post-validation invariants encoded in the type.
///
/// Compared to the raw [`PaymentMethod`], `key` is non-`Option`: the validator
/// either supplies a key derived from the label (legacy v1 configs) or
/// asserts the user-provided one is well-formed. `label` remains optional,
/// since it is purely for display.
///
/// Fields are `pub(super)` so only the [`crate::config`] module can construct
/// instances directly. External callers — including tests in other modules —
/// must go through [`ValidatedPaymentMethod::from_validated_parts`] (test-only)
/// or obtain instances by validating a [`Config`].
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedPaymentMethod {
    pub(super) key: PaymentMethodKey,
    pub(super) label: Option<String>,
    pub(super) iban: Iban,
    pub(super) bic_swift: String,
}

impl ValidatedPaymentMethod {
    /// Convert from a raw [`PaymentMethod`] whose `key` is known to be `Some`.
    ///
    /// Panics if `key` is `None`. Only the validator should call this — it is
    /// responsible for filling in derived keys before invoking `from_partial`.
    fn from_partial(p: PaymentMethod) -> Self {
        Self {
            key: p.key.expect(
                "validator must populate PaymentMethod.key before constructing ValidatedPaymentMethod",
            ),
            label: p.label,
            iban: p.iban,
            bic_swift: p.bic_swift,
        }
    }

    /// Borrow the payment method's key.
    ///
    /// Currently used only by tests; kept on the public surface for symmetry
    /// with [`ValidatedRecipient::key`] and so future `payment list|delete`
    /// CLI subcommands can reach the slug without needing a friend-module
    /// accessor.
    #[allow(dead_code)]
    pub fn key(&self) -> &PaymentMethodKey {
        &self.key
    }

    /// Borrow the payment method's display label, if present.
    pub fn label(&self) -> Option<&str> {
        self.label.as_deref()
    }

    /// Borrow the payment method's IBAN.
    pub fn iban(&self) -> &Iban {
        &self.iban
    }

    /// Borrow the payment method's BIC/SWIFT code.
    pub fn bic_swift(&self) -> &str {
        &self.bic_swift
    }

    /// Test-only constructor that bypasses the normal validator path.
    ///
    /// Used by test fixtures in other modules that need to assemble a
    /// [`ValidatedPaymentMethod`] without round-tripping through
    /// [`Config::validate`]. Production code paths construct via
    /// [`ValidatedPaymentMethod::from_partial`] inside the validator.
    #[cfg(test)]
    pub(crate) fn from_validated_parts(
        key: PaymentMethodKey,
        label: Option<String>,
        iban: Iban,
        bic_swift: String,
    ) -> Self {
        Self {
            key,
            label,
            iban,
            bic_swift,
        }
    }
}

/// A fully validated configuration with all required sections present.
///
/// Invariants encoded in the type system (post-validation):
/// - `recipients` has at least one entry.
/// - The default-recipient index is in-bounds for `recipients` (encapsulated
///   as a private field; use [`default_recipient`](Self::default_recipient)
///   to dereference safely).
/// - `payment` has at least one entry, every entry has a non-`Option` key.
/// - `presets` has at least one entry.
/// - Every recipient's key is `Some` (encoded as a non-`Option` field on
///   [`ValidatedRecipient`]).
#[derive(Debug, Clone, PartialEq)]
pub struct ValidatedConfig {
    pub sender: Sender,
    /// All available recipients (guaranteed non-empty).
    pub recipients: NonEmpty<ValidatedRecipient>,
    /// Index of the default recipient within `recipients`.
    /// Invariant: `< recipients.len()` (validator enforces). Private so the
    /// invariant cannot be violated by external construction; access via
    /// [`default_recipient`](Self::default_recipient) /
    /// [`default_recipient_key`](Self::default_recipient_key).
    default_recipient_idx: usize,
    /// Guaranteed non-empty.
    pub payment: NonEmpty<ValidatedPaymentMethod>,
    /// Guaranteed non-empty.
    pub presets: NonEmpty<Preset>,
    pub defaults: Defaults,
    pub branding: ValidatedBranding,
    pub template: TemplateKey,
    pub locale: Locale,
}

impl ValidatedConfig {
    /// Borrow the default recipient — the one referenced by the (private)
    /// default-recipient index.
    pub fn default_recipient(&self) -> &ValidatedRecipient {
        // Safe by construction: validator guarantees the index is in-bounds.
        &self.recipients[self.default_recipient_idx]
    }

    /// Borrow the default recipient's key. Convenience accessor for call
    /// sites that previously used `default_recipient_key` directly.
    pub fn default_recipient_key(&self) -> &RecipientKey {
        &self.default_recipient().key
    }

    /// Test-only constructor that assembles a [`ValidatedConfig`] directly,
    /// asserting the index invariant rather than re-validating from scratch.
    ///
    /// External tests use this to build fixtures without round-tripping
    /// through [`Config::validate`]. Production code paths assemble inline at
    /// the end of [`Config::validate`].
    #[cfg(test)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_validated_parts(
        sender: Sender,
        recipients: NonEmpty<ValidatedRecipient>,
        default_recipient_idx: usize,
        payment: NonEmpty<ValidatedPaymentMethod>,
        presets: NonEmpty<Preset>,
        defaults: Defaults,
        branding: ValidatedBranding,
        template: TemplateKey,
        locale: Locale,
    ) -> Self {
        assert!(
            default_recipient_idx < recipients.len(),
            "default_recipient_idx must be in bounds"
        );
        Self {
            sender,
            recipients,
            default_recipient_idx,
            payment,
            presets,
            defaults,
            branding,
            template,
            locale,
        }
    }
}

/// Result of validating a [`Config`].
#[derive(Debug)]
pub enum ValidationOutcome {
    /// All required sections are present.
    Complete(ValidatedConfig),
    /// One or more required sections are missing.
    Incomplete {
        #[allow(dead_code)] // needed by setup wizard (Story 2.1) to resume from partial config
        config: Config,
        missing: Vec<ConfigSection>,
    },
}

/// Outcome of recipient normalization: collapse v1 + v2 inputs to a single
/// `(list, default_key)` pair, signaling whether the section is fully missing.
enum RecipientsNormalized {
    /// A non-empty recipient list and (optionally) a default-recipient key.
    /// The list is guaranteed non-empty; every recipient's `key` is `Some`.
    Present(Vec<Recipient>, Option<RecipientKey>),
    /// No recipients configured (or an empty v2 list). Caller should push
    /// [`ConfigSection::Recipient`] onto its `missing` accumulator.
    Missing,
}

/// Normalize the v1 (`recipient`) + v2 (`recipients` + `default_recipient`)
/// inputs to a single canonical pair.
///
/// v2 wins when both are present. v1 inputs without a key get one derived
/// from the recipient name; if that derivation fails, returns
/// [`ConfigError::InvalidDefaultRecipient`].
fn normalize_recipients(
    v1: Option<Recipient>,
    v2: Option<Vec<Recipient>>,
    default_key: Option<RecipientKey>,
) -> Result<RecipientsNormalized, ConfigError> {
    match (v1, v2, default_key) {
        (_, Some(list), dk) if !list.is_empty() => Ok(RecipientsNormalized::Present(list, dk)),
        (_, Some(_), _) => Ok(RecipientsNormalized::Missing), // empty v2 list
        (Some(mut r), None, _) => {
            let key = match r.key.clone() {
                Some(k) => k,
                None => RecipientKey::from_name(&r.name)
                    .map_err(|e| ConfigError::InvalidDefaultRecipient(e.to_string()))?,
            };
            r.key = Some(key.clone());
            Ok(RecipientsNormalized::Present(vec![r], Some(key)))
        }
        (None, None, _) => Ok(RecipientsNormalized::Missing),
    }
}

/// Verify recipient-list invariants: every recipient has a key, no duplicate
/// keys, and the `default_key` (when present) references one of them.
///
/// Pure check — does not mutate inputs. The `default_key` is required: a
/// non-empty recipient list with `None` default is reported as
/// [`ConfigError::MissingDefaultRecipient`].
fn validate_recipient_invariants(
    list: &[Recipient],
    default_key: Option<&RecipientKey>,
) -> Result<(), ConfigError> {
    let mut seen = std::collections::HashSet::new();
    for r in list {
        let k = r.key.as_ref().ok_or_else(|| {
            ConfigError::InvalidDefaultRecipient("recipient has missing key".into())
        })?;
        if !seen.insert(k.clone()) {
            return Err(ConfigError::DuplicateRecipientKey(k.as_str().to_string()));
        }
    }
    match default_key {
        Some(dk) => {
            if !list.iter().any(|r| r.key.as_ref() == Some(dk)) {
                return Err(ConfigError::InvalidDefaultRecipient(
                    dk.as_str().to_string(),
                ));
            }
        }
        None => return Err(ConfigError::MissingDefaultRecipient),
    }
    Ok(())
}

/// Phase A of the payment pipeline: fill in missing keys (deriving from
/// `label` when needed) and verify uniqueness across the resulting set.
///
/// Each entry must have at least one of `key` or `label`; explicit `key`
/// always wins over a `label`-derived one. Derivation runs *before* the
/// uniqueness check, so two entries — one with `key="sepa"` and another with
/// `label="SEPA"` — are correctly detected as a collision.
///
/// Returns the input vec with every entry's `key` populated, or a
/// [`ConfigError::InvalidPaymentMethod`] / [`ConfigError::DuplicatePaymentKey`]
/// if validation fails.
fn normalize_payment_methods(
    methods: Vec<PaymentMethod>,
) -> Result<Vec<PaymentMethod>, ConfigError> {
    let mut normalized = Vec::with_capacity(methods.len());
    for mut p in methods {
        if p.key.is_none() {
            let derived = match p.label.as_deref() {
                Some(label) => PaymentMethodKey::from_name(label).map_err(|e| {
                    ConfigError::InvalidPaymentMethod(format!(
                        "could not derive key from label \"{label}\": {e}"
                    ))
                })?,
                None => {
                    return Err(ConfigError::InvalidPaymentMethod(
                        "payment method requires a `key` or `label`".into(),
                    ));
                }
            };
            p.key = Some(derived);
        }
        normalized.push(p);
    }

    // Uniqueness check happens after derivation so derived and explicit keys
    // collide on the same set.
    let mut seen = std::collections::HashSet::new();
    for p in &normalized {
        let k = p
            .key
            .as_ref()
            .expect("key populated by normalization step above");
        if !seen.insert(k.clone()) {
            return Err(ConfigError::DuplicatePaymentKey {
                key: k.as_str().to_string(),
            });
        }
    }

    Ok(normalized)
}

/// Fold an optional [`Branding`] into a fully resolved [`ValidatedBranding`],
/// substituting defaults for absent fields.
fn resolve_branding(branding: Option<Branding>) -> ValidatedBranding {
    match branding {
        Some(b) => ValidatedBranding {
            logo: b.logo,
            accent_color: b.accent_color.unwrap_or_else(default_accent_color),
            font: b.font,
            footer_text: b.footer_text,
        },
        None => ValidatedBranding::default(),
    }
}

impl Config {
    /// Validate that all required sections are present.
    ///
    /// Returns `Ok(ValidationOutcome::Complete)` with a [`ValidatedConfig`] when
    /// all sections are filled in, or `Ok(ValidationOutcome::Incomplete)` listing
    /// which sections are missing.
    ///
    /// Returns `Err(ConfigError)` for hard errors like duplicate keys or invalid
    /// default recipient references.
    pub fn validate(self) -> Result<ValidationOutcome, ConfigError> {
        let Config {
            sender,
            recipient,
            recipients: v2_recipients,
            default_recipient,
            payment,
            presets,
            defaults,
            branding,
        } = self;

        // Order matters: `missing` is asserted on by tests in the exact sequence
        // Sender → Recipient → Payment → Presets.
        let mut missing = Vec::new();
        if sender.is_none() {
            missing.push(ConfigSection::Sender);
        }

        // Normalize v1/v2 recipient inputs and validate invariants on the
        // resulting list. Hard errors short-circuit; a missing section is
        // recorded for the partial-config path.
        let (recipients, default_key) =
            match normalize_recipients(recipient, v2_recipients, default_recipient)? {
                RecipientsNormalized::Present(list, dk) => {
                    validate_recipient_invariants(&list, dk.as_ref())?;
                    (Some(list), dk)
                }
                RecipientsNormalized::Missing => {
                    missing.push(ConfigSection::Recipient);
                    (None, None)
                }
            };

        if payment.as_ref().is_none_or(|v| v.is_empty()) {
            missing.push(ConfigSection::Payment);
        }
        if presets.as_ref().is_none_or(|v| v.is_empty()) {
            missing.push(ConfigSection::Presets);
        }

        if !missing.is_empty() {
            return Ok(ValidationOutcome::Incomplete {
                config: Config {
                    sender,
                    recipient: None, // already consumed by normalization
                    recipients,
                    default_recipient: default_key,
                    payment,
                    presets,
                    defaults,
                    branding,
                },
                missing,
            });
        }

        // All sections present. Assemble the validated config.
        let recipients_vec = recipients.expect("recipients present when no missing sections");
        let dk = default_key.expect("default key validated above");

        // Locate the default recipient's index before consuming the vec, so
        // the resulting NonEmpty + idx pair is invariant-by-construction.
        let default_recipient_idx = recipients_vec
            .iter()
            .position(|r| r.key.as_ref() == Some(&dk))
            .expect("default key validated against recipients above");

        // Tighten Recipient (Option<RecipientKey>) → ValidatedRecipient.
        // Validator already enforced `key.is_some()` for every recipient.
        let validated_recipients: Vec<ValidatedRecipient> = recipients_vec
            .into_iter()
            .map(ValidatedRecipient::from_partial)
            .collect();
        let recipients_ne = NonEmpty::try_from_vec(validated_recipients)
            .expect("recipients non-empty checked above");

        // Normalize payment methods: derive missing keys from labels, then
        // verify uniqueness. Hard errors (missing key+label, slugify failure,
        // duplicate key) bubble up — `Incomplete` is reserved for sections
        // that are entirely absent.
        let normalized_payment = normalize_payment_methods(payment.expect("payment present"))?;

        // `payment` and `presets` are guaranteed non-empty by the missing-section
        // check above; lift them into `NonEmpty` to encode that statically.
        let validated_payment: Vec<ValidatedPaymentMethod> = normalized_payment
            .into_iter()
            .map(ValidatedPaymentMethod::from_partial)
            .collect();
        let payment_ne =
            NonEmpty::try_from_vec(validated_payment).expect("payment non-empty checked above");
        let presets_ne = NonEmpty::try_from_vec(presets.expect("presets present"))
            .expect("presets non-empty checked above");

        let resolved_defaults = defaults.unwrap_or_default();
        let template = resolved_defaults.template;
        let locale = resolved_defaults.locale;

        Ok(ValidationOutcome::Complete(ValidatedConfig {
            sender: sender.expect("sender present when no missing sections"),
            recipients: recipients_ne,
            default_recipient_idx,
            payment: payment_ne,
            presets: presets_ne,
            defaults: resolved_defaults,
            branding: resolve_branding(branding),
            template,
            locale,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_section_display() {
        assert_eq!(ConfigSection::Sender.to_string(), "sender");
        assert_eq!(ConfigSection::Recipient.to_string(), "recipient");
        assert_eq!(ConfigSection::Payment.to_string(), "payment");
        assert_eq!(ConfigSection::Presets.to_string(), "presets");
    }

    // ── Cycle 2 ──

    #[test]
    fn test_validate_empty_config_returns_all_missing() {
        // Act
        let result = Config::default().validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Incomplete { missing, .. } => {
                assert_eq!(
                    missing,
                    vec![
                        ConfigSection::Sender,
                        ConfigSection::Recipient,
                        ConfigSection::Payment,
                        ConfigSection::Presets,
                    ]
                );
            }
            ValidationOutcome::Complete(_) => panic!("Expected Incomplete"),
        }
    }

    // ── Cycle 3 ──

    #[test]
    fn test_validate_complete_config_returns_validated() {
        // Act
        let result = make_complete_config().validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.sender.name, "Alice");
                assert_eq!(v.default_recipient().name, "Bob Corp");
                assert_eq!(v.recipients.len(), 1);
                assert_eq!(v.default_recipient_key().as_str(), "bob-corp");
                assert_eq!(
                    v.default_recipient().key,
                    RecipientKey::try_new("bob-corp").unwrap()
                );
                assert_eq!(v.payment.len(), 1);
                assert_eq!(v.presets.len(), 1);
                assert_eq!(v.defaults.currency, crate::domain::Currency::Eur);
            }
            ValidationOutcome::Incomplete { .. } => panic!("Expected Complete"),
        }
    }

    // ── Cycle 4 ──

    #[test]
    fn test_validate_missing_defaults_filled_with_default() {
        // Arrange
        let mut config = make_complete_config();
        config.defaults = None;

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.defaults.currency, crate::domain::Currency::Eur);
                assert_eq!(v.defaults.invoice_date_day, 9);
                assert_eq!(v.defaults.payment_terms_days, 30);
            }
            ValidationOutcome::Incomplete { .. } => panic!("Expected Complete"),
        }
    }

    // ── Cycle 5 ──

    #[test]
    fn test_validate_sender_only_returns_three_missing() {
        // Arrange
        let config = Config {
            sender: Some(make_sender()),
            ..Config::default()
        };

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Incomplete { missing, .. } => {
                assert_eq!(
                    missing,
                    vec![
                        ConfigSection::Recipient,
                        ConfigSection::Payment,
                        ConfigSection::Presets,
                    ]
                );
            }
            ValidationOutcome::Complete(_) => panic!("Expected Incomplete"),
        }
    }

    // ── Cycle 6 ──

    #[test]
    fn test_validate_sender_and_recipient_returns_two_missing() {
        // Arrange
        let config = Config {
            sender: Some(make_sender()),
            recipient: Some(make_recipient()),
            ..Config::default()
        };

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Incomplete { missing, .. } => {
                assert_eq!(
                    missing,
                    vec![ConfigSection::Payment, ConfigSection::Presets]
                );
            }
            ValidationOutcome::Complete(_) => panic!("Expected Incomplete"),
        }
    }

    // ── Cycle 7 ──

    #[test]
    fn test_validate_empty_payment_vec_treated_as_missing() {
        // Arrange
        let mut config = make_complete_config();
        config.payment = Some(vec![]);

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Incomplete { missing, .. } => {
                assert_eq!(missing, vec![ConfigSection::Payment]);
            }
            ValidationOutcome::Complete(_) => panic!("Expected Incomplete"),
        }
    }

    // ── Cycle 8 ──

    #[test]
    fn test_validate_empty_presets_vec_treated_as_missing() {
        // Arrange
        let mut config = make_complete_config();
        config.presets = Some(vec![]);

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Incomplete { missing, .. } => {
                assert_eq!(missing, vec![ConfigSection::Presets]);
            }
            ValidationOutcome::Complete(_) => panic!("Expected Incomplete"),
        }
    }

    // ── Story 7.1 Phase 4: Recipient validation ──

    #[test]
    fn test_validate_v1_config_normalizes_to_recipients_list() {
        // Arrange — v1 style with single recipient
        let config = Config {
            sender: Some(make_sender()),
            recipient: Some(make_recipient()),
            recipients: None,
            default_recipient: None,
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
            branding: None,
        };

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.recipients.len(), 1);
                assert_eq!(v.default_recipient_key().as_str(), "bob-corp");
                assert_eq!(v.default_recipient().name, "Bob Corp");
                assert_eq!(
                    v.default_recipient().key,
                    RecipientKey::try_new("bob-corp").unwrap()
                );
            }
            ValidationOutcome::Incomplete { .. } => panic!("Expected Complete"),
        }
    }

    #[test]
    fn test_validate_v2_config_with_multiple_recipients() {
        // Arrange
        let config = Config {
            sender: Some(make_sender()),
            recipient: None,
            recipients: Some(vec![
                Recipient {
                    key: Some(RecipientKey::try_new("acme").unwrap()),
                    name: "Acme Corp".into(),
                    address: vec!["123 St".into()],
                    company_id: None,
                    vat_number: None,
                },
                Recipient {
                    key: Some(RecipientKey::try_new("globex").unwrap()),
                    name: "Globex Inc".into(),
                    address: vec!["456 Ave".into()],
                    company_id: None,
                    vat_number: None,
                },
            ]),
            default_recipient: Some(RecipientKey::try_new("globex").unwrap()),
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
            branding: None,
        };

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.recipients.len(), 2);
                assert_eq!(v.default_recipient_key().as_str(), "globex");
                assert_eq!(v.default_recipient().name, "Globex Inc");
            }
            ValidationOutcome::Incomplete { .. } => panic!("Expected Complete"),
        }
    }

    #[test]
    fn test_validate_missing_recipients_returns_incomplete() {
        // Arrange
        let config = Config {
            sender: Some(make_sender()),
            recipient: None,
            recipients: None,
            default_recipient: None,
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
            branding: None,
        };

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Incomplete { missing, .. } => {
                assert!(missing.contains(&ConfigSection::Recipient));
            }
            ValidationOutcome::Complete(_) => panic!("Expected Incomplete"),
        }
    }

    #[test]
    fn test_validate_empty_recipients_vec_treated_as_missing_section() {
        // Arrange
        let config = Config {
            sender: Some(make_sender()),
            recipient: None,
            recipients: Some(vec![]),
            default_recipient: None,
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
            branding: None,
        };

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Incomplete { missing, .. } => {
                assert!(missing.contains(&ConfigSection::Recipient));
            }
            ValidationOutcome::Complete(_) => panic!("Expected Incomplete"),
        }
    }

    #[test]
    fn test_validate_invalid_default_recipient_key_returns_error() {
        // Arrange
        let config = Config {
            sender: Some(make_sender()),
            recipient: None,
            recipients: Some(vec![make_recipient_with_key()]),
            default_recipient: Some(RecipientKey::try_new("nonexistent").unwrap()),
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
            branding: None,
        };

        // Act
        let result = config.validate();

        // Assert
        assert!(matches!(
            result,
            Err(ConfigError::InvalidDefaultRecipient(_))
        ));
    }

    #[test]
    fn test_validate_missing_default_recipient_with_recipients_returns_error() {
        // Arrange
        let config = Config {
            sender: Some(make_sender()),
            recipient: None,
            recipients: Some(vec![make_recipient_with_key()]),
            default_recipient: None,
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
            branding: None,
        };

        // Act
        let result = config.validate();

        // Assert
        assert!(matches!(result, Err(ConfigError::MissingDefaultRecipient)));
    }

    #[test]
    fn test_validate_duplicate_recipient_keys_returns_error() {
        // Arrange
        let config = Config {
            sender: Some(make_sender()),
            recipient: None,
            recipients: Some(vec![
                Recipient {
                    key: Some(RecipientKey::try_new("acme").unwrap()),
                    name: "Acme Corp".into(),
                    address: vec!["123 St".into()],
                    company_id: None,
                    vat_number: None,
                },
                Recipient {
                    key: Some(RecipientKey::try_new("acme").unwrap()),
                    name: "Acme LLC".into(),
                    address: vec!["456 Ave".into()],
                    company_id: None,
                    vat_number: None,
                },
            ]),
            default_recipient: Some(RecipientKey::try_new("acme").unwrap()),
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
            branding: None,
        };

        // Act
        let result = config.validate();

        // Assert
        assert!(matches!(result, Err(ConfigError::DuplicateRecipientKey(_))));
    }

    #[test]
    fn test_empty_recipient_key_rejected_at_deserialize() {
        // Arrange — empty keys are no longer constructible via RecipientKey,
        // so the failure path is at YAML parse time, not validate().
        let yaml = "key: \"\"\nname: Acme Corp\naddress:\n  - 123 St\n";

        // Act
        let result: Result<Recipient, _> = serde_yaml::from_str(yaml);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_v1_and_v2_both_present_v2_wins() {
        // Arrange — pathological: both recipient and recipients set
        let config = Config {
            sender: Some(make_sender()),
            recipient: Some(Recipient {
                key: None,
                name: "Old Corp".into(),
                address: vec!["Old St".into()],
                company_id: None,
                vat_number: None,
            }),
            recipients: Some(vec![make_recipient_with_key()]),
            default_recipient: Some(RecipientKey::try_new("bob-corp").unwrap()),
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
            branding: None,
        };

        // Act
        let result = config.validate().unwrap();

        // Assert — v2 recipients wins, not the v1 recipient
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.default_recipient().name, "Bob Corp");
                assert_eq!(v.recipients.len(), 1);
            }
            ValidationOutcome::Incomplete { .. } => panic!("Expected Complete"),
        }
    }

    // ── Story 7.1 Phase 5: default_recipient() ──

    #[test]
    fn test_default_recipient_returns_configured_default() {
        // Arrange
        let config = Config {
            sender: Some(make_sender()),
            recipient: None,
            recipients: Some(vec![
                Recipient {
                    key: Some(RecipientKey::try_new("acme").unwrap()),
                    name: "Acme Corp".into(),
                    address: vec!["123 St".into()],
                    company_id: None,
                    vat_number: None,
                },
                Recipient {
                    key: Some(RecipientKey::try_new("globex").unwrap()),
                    name: "Globex Inc".into(),
                    address: vec!["456 Ave".into()],
                    company_id: None,
                    vat_number: None,
                },
            ]),
            default_recipient: Some(RecipientKey::try_new("globex").unwrap()),
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
            branding: None,
        };

        // Act
        let validated = match config.validate().unwrap() {
            ValidationOutcome::Complete(v) => v,
            _ => panic!("Expected Complete"),
        };

        // Assert
        assert_eq!(validated.default_recipient().name, "Globex Inc");
    }

    #[test]
    fn test_default_recipient_single_recipient() {
        // Arrange
        let config = make_complete_config();

        // Act
        let validated = match config.validate().unwrap() {
            ValidationOutcome::Complete(v) => v,
            _ => panic!("Expected Complete"),
        };

        // Assert
        assert_eq!(validated.default_recipient().name, "Bob Corp");
    }

    // ── Story 11.1: v1 backwards compatibility verification ──

    #[test]
    fn test_v1_config_round_trips_through_validation_with_single_recipient() {
        // Arrange — pure v1 config with no recipients list or default_recipient
        let config = Config {
            sender: Some(make_sender()),
            recipient: Some(make_recipient()),
            recipients: None,
            default_recipient: None,
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
            branding: None,
        };

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(
                    v.recipients.len(),
                    1,
                    "v1 config should normalize to single-element recipients list"
                );
                assert_eq!(v.default_recipient().name, "Bob Corp");
                assert!(
                    !v.default_recipient_key().as_str().is_empty(),
                    "default key should be auto-derived"
                );
            }
            ValidationOutcome::Incomplete { .. } => panic!("Expected Complete for v1 config"),
        }
    }

    // ── Story 12.1 Cycle 6: ValidatedConfig.template ──

    #[test]
    fn test_validated_config_includes_template_from_defaults() {
        // Arrange
        let config = make_complete_config();

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.template, TemplateKey::Leda);
            }
            ValidationOutcome::Incomplete { .. } => panic!("Expected Complete"),
        }
    }

    #[test]
    fn test_validated_config_template_custom_value() {
        // Arrange
        let mut config = make_complete_config();
        config.defaults = Some(Defaults {
            template: TemplateKey::Callisto,
            ..Defaults::default()
        });

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.template, TemplateKey::Callisto);
            }
            ValidationOutcome::Incomplete { .. } => panic!("Expected Complete"),
        }
    }

    #[test]
    fn test_validated_config_missing_defaults_gets_leda_template() {
        // Arrange
        let mut config = make_complete_config();
        config.defaults = None;

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.template, TemplateKey::Leda);
            }
            ValidationOutcome::Incomplete { .. } => panic!("Expected Complete"),
        }
    }

    // ── PaymentMethod validation: key/label split ──

    #[test]
    fn test_validate_payment_v1_label_only_derives_key_from_label() {
        // Arrange — legacy v1 shape: label only, no key.
        let mut config = make_complete_config();
        config.payment = Some(vec![make_payment_label_only(
            "SEPA Transfer",
            "DE89370400440532013000",
        )]);

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                let p = &v.payment[0];
                assert_eq!(p.key().as_str(), "sepa-transfer");
                assert_eq!(p.label(), Some("SEPA Transfer"));
            }
            ValidationOutcome::Incomplete { .. } => panic!("Expected Complete"),
        }
    }

    #[test]
    fn test_validate_payment_explicit_key_takes_precedence_over_label() {
        // Arrange — both present: explicit key wins, label preserved.
        let mut config = make_complete_config();
        config.payment = Some(vec![PaymentMethod {
            key: Some(PaymentMethodKey::try_new("custom-key").unwrap()),
            label: Some("SEPA Transfer".into()),
            iban: crate::domain::Iban::try_new("DE89370400440532013000").unwrap(),
            bic_swift: "BIC".into(),
        }]);

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                let p = &v.payment[0];
                assert_eq!(p.key().as_str(), "custom-key");
                assert_eq!(p.label(), Some("SEPA Transfer"));
            }
            ValidationOutcome::Incomplete { .. } => panic!("Expected Complete"),
        }
    }

    #[test]
    fn test_validate_payment_no_key_no_label_returns_error() {
        // Arrange
        let mut config = make_complete_config();
        config.payment = Some(vec![PaymentMethod {
            key: None,
            label: None,
            iban: crate::domain::Iban::try_new("DE89370400440532013000").unwrap(),
            bic_swift: "BIC".into(),
        }]);

        // Act
        let result = config.validate();

        // Assert
        assert!(matches!(result, Err(ConfigError::InvalidPaymentMethod(_))));
    }

    #[test]
    fn test_validate_payment_label_that_slugifies_to_empty_returns_error() {
        // Arrange — `!!!` has no alphanumerics → slugify yields empty.
        let mut config = make_complete_config();
        config.payment = Some(vec![make_payment_label_only(
            "!!!",
            "DE89370400440532013000",
        )]);

        // Act
        let result = config.validate();

        // Assert
        assert!(matches!(result, Err(ConfigError::InvalidPaymentMethod(_))));
    }

    #[test]
    fn test_validate_duplicate_payment_keys_returns_error() {
        // Arrange — two methods with the same explicit key.
        let mut config = make_complete_config();
        config.payment = Some(vec![
            make_payment_key_only("sepa", "DE89370400440532013000"),
            make_payment_key_only("sepa", "GB82WEST12345698765432"),
        ]);

        // Act
        let result = config.validate();

        // Assert
        assert!(matches!(
            result,
            Err(ConfigError::DuplicatePaymentKey { .. })
        ));
    }

    #[test]
    fn test_validate_duplicate_keys_one_explicit_one_derived() {
        // Arrange — method A has explicit key="sepa-transfer"; method B has
        // label="SEPA Transfer" which slugifies to "sepa-transfer". Both end
        // up with the same key.
        //
        // CRITICAL: this test catches the bug where uniqueness is checked
        // before derivation — the wrong order would let this slip through.
        let mut config = make_complete_config();
        config.payment = Some(vec![
            make_payment_key_only("sepa-transfer", "DE89370400440532013000"),
            make_payment_label_only("SEPA Transfer", "GB82WEST12345698765432"),
        ]);

        // Act
        let result = config.validate();

        // Assert
        assert!(matches!(
            result,
            Err(ConfigError::DuplicatePaymentKey { .. })
        ));
    }

    #[test]
    fn test_validated_config_payment_is_non_empty() {
        // Arrange — confirms the NonEmpty<ValidatedPaymentMethod> invariant
        // survives the refactor.
        let config = make_complete_config();

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert!(!v.payment.is_empty());
                // `v.payment[0]` is `ValidatedPaymentMethod`, not `PaymentMethod`.
                let _: &ValidatedPaymentMethod = &v.payment[0];
            }
            ValidationOutcome::Incomplete { .. } => panic!("Expected Complete"),
        }
    }

    // ── Helpers ──

    fn make_sender() -> Sender {
        Sender {
            name: "Alice".into(),
            address: vec!["123 St".into()],
            email: "a@b.com".into(),
        }
    }

    fn make_recipient() -> Recipient {
        Recipient {
            key: None,
            name: "Bob Corp".into(),
            address: vec!["456 Ave".into()],
            company_id: None,
            vat_number: None,
        }
    }

    fn make_recipient_with_key() -> Recipient {
        Recipient {
            key: Some(RecipientKey::try_new("bob-corp").unwrap()),
            name: "Bob Corp".into(),
            address: vec!["456 Ave".into()],
            company_id: None,
            vat_number: None,
        }
    }

    fn make_payment() -> Vec<PaymentMethod> {
        vec![PaymentMethod {
            key: None,
            label: Some("SEPA".into()),
            iban: crate::domain::Iban::try_new("DE89370400440532013000").unwrap(),
            bic_swift: "BIC".into(),
        }]
    }

    fn make_payment_label_only(label: &str, iban: &str) -> PaymentMethod {
        PaymentMethod {
            key: None,
            label: Some(label.into()),
            iban: crate::domain::Iban::try_new(iban).unwrap(),
            bic_swift: "BIC".into(),
        }
    }

    fn make_payment_key_only(key: &str, iban: &str) -> PaymentMethod {
        PaymentMethod {
            key: Some(PaymentMethodKey::try_new(key).unwrap()),
            label: None,
            iban: crate::domain::Iban::try_new(iban).unwrap(),
            bic_swift: "BIC".into(),
        }
    }

    fn make_presets() -> Vec<Preset> {
        vec![Preset {
            key: crate::domain::PresetKey::try_new("dev").unwrap(),
            description: "Dev".into(),
            default_rate: 100.0,
            currency: None,
            tax_rate: None,
        }]
    }

    fn make_complete_config() -> Config {
        Config {
            sender: Some(make_sender()),
            recipient: None,
            recipients: Some(vec![make_recipient_with_key()]),
            default_recipient: Some(RecipientKey::try_new("bob-corp").unwrap()),
            payment: Some(make_payment()),
            presets: Some(make_presets()),
            defaults: Some(Defaults::default()),
            branding: None,
        }
    }

    // ── Sprint 10: ValidatedBranding integration tests ──

    #[test]
    fn test_validate_no_branding_uses_defaults() {
        // Arrange
        let config = make_complete_config(); // branding: None

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.branding.accent_color.as_str(), "#2c3e50");
                assert!(v.branding.font.is_none());
                assert!(v.branding.footer_text.is_none());
                assert!(v.branding.logo.is_none());
            }
            _ => panic!("Expected Complete"),
        }
    }

    #[test]
    fn test_validate_branding_accent_color_passthrough() {
        // Arrange
        let mut config = make_complete_config();
        config.branding = Some(crate::config::types::Branding {
            accent_color: Some(HexColor::try_new("#ff0000").unwrap()),
            ..Default::default()
        });

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.branding.accent_color.as_str(), "#ff0000");
            }
            _ => panic!("Expected Complete"),
        }
    }

    #[test]
    fn test_validate_branding_missing_accent_uses_default() {
        // Arrange — Branding present but accent_color None still falls back.
        let mut config = make_complete_config();
        config.branding = Some(crate::config::types::Branding {
            accent_color: None,
            ..Default::default()
        });

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.branding.accent_color.as_str(), "#2c3e50");
            }
            _ => panic!("Expected Complete"),
        }
    }

    #[test]
    fn test_validate_branding_font_passthrough() {
        // Arrange
        let mut config = make_complete_config();
        config.branding = Some(crate::config::types::Branding {
            font: Some("Fira Code".into()),
            ..Default::default()
        });

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.branding.font, Some("Fira Code".into()));
            }
            _ => panic!("Expected Complete"),
        }
    }

    #[test]
    fn test_validate_branding_footer_text_passthrough() {
        // Arrange
        let mut config = make_complete_config();
        config.branding = Some(crate::config::types::Branding {
            footer_text: Some("Thanks!".into()),
            ..Default::default()
        });

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.branding.footer_text, Some("Thanks!".into()));
            }
            _ => panic!("Expected Complete"),
        }
    }

    #[test]
    fn test_validate_branding_logo_passthrough() {
        // Arrange
        let mut config = make_complete_config();
        config.branding = Some(crate::config::types::Branding {
            logo: Some("logo.png".into()),
            ..Default::default()
        });

        // Act
        let result = config.validate().unwrap();

        // Assert
        match result {
            ValidationOutcome::Complete(v) => {
                assert_eq!(v.branding.logo, Some("logo.png".into()));
            }
            _ => panic!("Expected Complete"),
        }
    }
}
