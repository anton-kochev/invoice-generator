//! Domain newtypes that enforce invariants at construction time.
//!
//! Instead of letting raw `String`s flow through the codebase and discovering
//! invalid data at the point of use, the types here parse-don't-validate at
//! the boundary (typically deserialization or interactive setup), so the rest
//! of the program can rely on the invariant.
pub mod billing_unit;
pub mod currency;
pub mod hex_color;
pub mod iban;
pub mod non_empty;
pub mod payment_method_key;
pub mod preset_key;
pub mod recipient_key;
pub mod sender_key;
pub mod tax_rate;

pub use billing_unit::BillingUnit;
pub use currency::Currency;
pub use hex_color::HexColor;
pub use iban::Iban;
pub use non_empty::NonEmpty;
pub use payment_method_key::PaymentMethodKey;
pub use preset_key::PresetKey;
pub use recipient_key::RecipientKey;
pub use sender_key::SenderKey;
pub use tax_rate::is_valid_tax_rate;
