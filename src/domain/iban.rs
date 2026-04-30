//! IBAN (International Bank Account Number) newtype with mod-97 checksum
//! validation per ISO 13616-1.
//!
//! Whitespace is tolerated and stripped at construction time. The stored form
//! is compact uppercase (no spaces). [`Display`] renders the canonical
//! human-friendly grouped form with spaces every 4 characters.

use std::fmt;

use thiserror::Error;

/// Validation errors for [`Iban`].
#[derive(Debug, Error, PartialEq, Eq)]
pub enum IbanError {
    /// Length is outside the ISO 13616-1 range (15..=34 after stripping spaces).
    #[error("invalid IBAN length: {0} characters (must be 15-34)")]
    InvalidLength(usize),

    /// Found a non-alphanumeric character (e.g. punctuation, control char).
    #[error("invalid IBAN: non-alphanumeric character in \"{0}\"")]
    InvalidCharacters(String),

    /// First two characters must be ASCII letters (country code).
    #[error("invalid IBAN: country code must be two letters in \"{0}\"")]
    InvalidCountryCode(String),

    /// Characters 3-4 must be ASCII digits (check digits).
    #[error("invalid IBAN: check digits must be two digits in \"{0}\"")]
    InvalidCheckDigits(String),

    /// Checksum (mod-97) did not equal 1.
    #[error("invalid IBAN: checksum failed for \"{0}\"")]
    ChecksumFailed(String),
}

/// A validated IBAN, stored in compact uppercase form.
///
/// Construction validates the input via [`Iban::try_new`]; the value is
/// guaranteed to be 15-34 ASCII alphanumerics with a passing mod-97 checksum.
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(transparent)]
pub struct Iban(String);

impl Iban {
    /// Parse `s` as an IBAN. Whitespace is stripped, characters are uppercased,
    /// and the mod-97 checksum is verified per ISO 13616-1.
    pub fn try_new(s: impl AsRef<str>) -> Result<Self, IbanError> {
        let raw = s.as_ref();
        let compact: String = raw
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_ascii_uppercase();

        if !(15..=34).contains(&compact.len()) {
            return Err(IbanError::InvalidLength(compact.len()));
        }
        if !compact.chars().all(|c| c.is_ascii_alphanumeric()) {
            return Err(IbanError::InvalidCharacters(compact));
        }
        let bytes = compact.as_bytes();
        if !bytes[0].is_ascii_alphabetic() || !bytes[1].is_ascii_alphabetic() {
            return Err(IbanError::InvalidCountryCode(compact));
        }
        if !bytes[2].is_ascii_digit() || !bytes[3].is_ascii_digit() {
            return Err(IbanError::InvalidCheckDigits(compact));
        }

        if mod97(&compact) != 1 {
            return Err(IbanError::ChecksumFailed(compact));
        }

        Ok(Self(compact))
    }

    /// Borrow the compact uppercase IBAN string (no spaces).
    ///
    /// Use [`Display`] to render the canonical grouped form for human output.
    #[allow(dead_code)] // public newtype accessor — used in tests, exposed for API completeness
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Display in canonical grouped form: groups of 4 separated by single spaces.
///
/// Example: `GB82WEST12345698765432` → `GB82 WEST 1234 5698 7654 32`.
impl fmt::Display for Iban {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        for chunk in self.0.as_bytes().chunks(4) {
            if !first {
                f.write_str(" ")?;
            }
            // SAFETY: chunks of an ASCII-alphanumeric string are valid UTF-8.
            f.write_str(std::str::from_utf8(chunk).expect("ASCII alphanumeric"))?;
            first = false;
        }
        Ok(())
    }
}

impl AsRef<str> for Iban {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl<'de> serde::Deserialize<'de> for Iban {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::try_new(s).map_err(serde::de::Error::custom)
    }
}

/// Streaming mod-97 checksum.
///
/// Builds the rearranged numeric string conceptually (move the first 4 chars
/// to the end, replace each letter with its 2-digit code A=10..Z=35), but
/// processes it left-to-right keeping a running remainder so we never construct
/// the full integer.
fn mod97(iban: &str) -> u32 {
    debug_assert!(iban.len() >= 4, "caller validated length");
    let bytes = iban.as_bytes();
    let mut rem: u32 = 0;
    // Process [4..] then [..4] (i.e. the rearranged form).
    for &b in bytes[4..].iter().chain(bytes[..4].iter()) {
        if b.is_ascii_digit() {
            rem = (rem * 10 + (b - b'0') as u32) % 97;
        } else {
            // ASCII alphabetic, validated by caller.
            let n = (b - b'A') as u32 + 10;
            // Two-digit value: shift remainder by 100.
            rem = (rem * 100 + n) % 97;
        }
    }
    rem
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── mod97 algorithm direct tests ──

    #[test]
    fn test_mod97_canonical_gb_iban_returns_one() {
        // Arrange — the ISO spec example, known-valid
        let iban = "GB82WEST12345698765432";

        // Act
        let r = mod97(iban);

        // Assert
        assert_eq!(r, 1);
    }

    // ── try_new: valid IBANs ──

    #[test]
    fn test_try_new_accepts_canonical_gb_with_spaces() {
        // Arrange — ISO 13616-1 example
        let input = "GB82 WEST 1234 5698 7654 32";

        // Act
        let iban = Iban::try_new(input).unwrap();

        // Assert
        assert_eq!(iban.as_str(), "GB82WEST12345698765432");
    }

    #[test]
    fn test_try_new_accepts_de_iban() {
        // Arrange
        let input = "DE89 3704 0044 0532 0130 00";

        // Act
        let iban = Iban::try_new(input).unwrap();

        // Assert
        assert_eq!(iban.as_str(), "DE89370400440532013000");
    }

    #[test]
    fn test_try_new_accepts_ua_iban() {
        // Arrange — Ukrainian IBAN, since UAH is a supported currency
        let input = "UA21 3996 2200 0002 6007 2335 6600 1";

        // Act
        let iban = Iban::try_new(input).unwrap();

        // Assert
        assert_eq!(iban.as_str(), "UA213996220000026007233566001");
    }

    #[test]
    fn test_try_new_lowercase_input_normalizes_to_uppercase() {
        // Arrange
        let input = "gb82west12345698765432";

        // Act
        let iban = Iban::try_new(input).unwrap();

        // Assert
        assert_eq!(iban.as_str(), "GB82WEST12345698765432");
    }

    #[test]
    fn test_try_new_strips_whitespace() {
        // Arrange — tabs and multiple spaces
        let input = "GB82\tWEST 1234   5698 7654 32";

        // Act
        let iban = Iban::try_new(input).unwrap();

        // Assert
        assert_eq!(iban.as_str(), "GB82WEST12345698765432");
    }

    // ── try_new: rejection cases ──

    #[test]
    fn test_try_new_rejects_bad_check_digits() {
        // Arrange — same as canonical GB but check digits set to 00
        let input = "GB00WEST12345698765432";

        // Act
        let result = Iban::try_new(input);

        // Assert
        assert!(matches!(result, Err(IbanError::ChecksumFailed(_))));
    }

    #[test]
    fn test_try_new_rejects_too_short() {
        // Arrange — 14 chars
        let input = "GB82WEST123456";

        // Act
        let result = Iban::try_new(input);

        // Assert
        assert!(matches!(result, Err(IbanError::InvalidLength(_))));
    }

    #[test]
    fn test_try_new_rejects_too_long() {
        // Arrange — 35 chars
        let input = "GB82WEST123456987654321234567890ABCDE";

        // Act
        let result = Iban::try_new(input);

        // Assert
        assert!(matches!(result, Err(IbanError::InvalidLength(_))));
    }

    #[test]
    fn test_try_new_rejects_punctuation() {
        // Arrange
        let input = "GB82-WEST-1234-5698-7654-32";

        // Act
        let result = Iban::try_new(input);

        // Assert
        assert!(matches!(result, Err(IbanError::InvalidCharacters(_))));
    }

    #[test]
    fn test_try_new_rejects_country_code_with_digits() {
        // Arrange — "12" is not a valid country code (must be letters)
        let input = "1234567890123456";

        // Act
        let result = Iban::try_new(input);

        // Assert
        assert!(matches!(result, Err(IbanError::InvalidCountryCode(_))));
    }

    #[test]
    fn test_try_new_rejects_letter_check_digits() {
        // Arrange — chars 3-4 must be digits
        let input = "GBABWEST12345698765432";

        // Act
        let result = Iban::try_new(input);

        // Assert
        assert!(matches!(result, Err(IbanError::InvalidCheckDigits(_))));
    }

    #[test]
    fn test_try_new_rejects_empty_string() {
        // Arrange & Act
        let result = Iban::try_new("");

        // Assert
        assert!(matches!(result, Err(IbanError::InvalidLength(0))));
    }

    // ── Display: grouped form ──

    #[test]
    fn test_display_groups_in_fours() {
        // Arrange
        let iban = Iban::try_new("GB82WEST12345698765432").unwrap();

        // Act
        let s = format!("{iban}");

        // Assert
        assert_eq!(s, "GB82 WEST 1234 5698 7654 32");
    }

    #[test]
    fn test_display_handles_uneven_trailing_chunk() {
        // Arrange — DE IBAN has 22 chars (5 groups of 4 + 2)
        let iban = Iban::try_new("DE89370400440532013000").unwrap();

        // Act
        let s = format!("{iban}");

        // Assert
        assert_eq!(s, "DE89 3704 0044 0532 0130 00");
    }

    // ── serde round trips ──

    #[test]
    fn test_serde_yaml_round_trip() {
        // Arrange
        let iban = Iban::try_new("GB82WEST12345698765432").unwrap();

        // Act
        let yaml = serde_yaml::to_string(&iban).unwrap();
        let loaded: Iban = serde_yaml::from_str(&yaml).unwrap();

        // Assert
        assert_eq!(loaded, iban);
    }

    #[test]
    fn test_serde_yaml_accepts_grouped_input() {
        // Arrange — config files are likely to contain spaces.
        let yaml = "GB82 WEST 1234 5698 7654 32\n";

        // Act
        let iban: Iban = serde_yaml::from_str(yaml).unwrap();

        // Assert — stored compact
        assert_eq!(iban.as_str(), "GB82WEST12345698765432");
    }

    #[test]
    fn test_serde_yaml_rejects_invalid_checksum() {
        // Arrange
        let yaml = "GB00WEST12345698765432\n";

        // Act
        let result: Result<Iban, _> = serde_yaml::from_str(yaml);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_serde_json_round_trip() {
        // Arrange
        let iban = Iban::try_new("DE89370400440532013000").unwrap();

        // Act
        let json = serde_json::to_string(&iban).unwrap();
        let loaded: Iban = serde_json::from_str(&json).unwrap();

        // Assert
        assert_eq!(loaded, iban);
        assert_eq!(json, "\"DE89370400440532013000\"");
    }
}
