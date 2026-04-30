//! Domain newtypes that enforce invariants at construction time.
//!
//! Instead of letting raw `String`s flow through the codebase and discovering
//! invalid data at the point of use, the types here parse-don't-validate at
//! the boundary (typically deserialization or interactive setup), so the rest
//! of the program can rely on the invariant.
pub mod hex_color;

pub use hex_color::HexColor;
