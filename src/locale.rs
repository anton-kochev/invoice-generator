use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

/// Supported locale codes for invoice localization.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Locale {
    #[serde(rename = "en-US")]
    #[default]
    EnUs,
    #[serde(rename = "en-GB")]
    EnGb,
    #[serde(rename = "de-DE")]
    DeDe,
    #[serde(rename = "fr-FR")]
    FrFr,
    #[serde(rename = "cs-CZ")]
    CsCz,
    #[serde(rename = "uk-UA")]
    UkUa,
}

impl Locale {
    /// All available locale variants.
    pub const ALL: [Locale; 6] = [
        Locale::EnUs,
        Locale::EnGb,
        Locale::DeDe,
        Locale::FrFr,
        Locale::CsCz,
        Locale::UkUa,
    ];

    /// Returns the nominative form of the month name for this locale.
    pub fn month_name(&self, month: time::Month) -> &'static str {
        use time::Month::*;
        match self {
            Self::EnUs | Self::EnGb => match month {
                January => "January",
                February => "February",
                March => "March",
                April => "April",
                May => "May",
                June => "June",
                July => "July",
                August => "August",
                September => "September",
                October => "October",
                November => "November",
                December => "December",
            },
            Self::DeDe => match month {
                January => "Januar",
                February => "Februar",
                March => "März",
                April => "April",
                May => "Mai",
                June => "Juni",
                July => "Juli",
                August => "August",
                September => "September",
                October => "Oktober",
                November => "November",
                December => "Dezember",
            },
            Self::FrFr => match month {
                January => "janvier",
                February => "février",
                March => "mars",
                April => "avril",
                May => "mai",
                June => "juin",
                July => "juillet",
                August => "août",
                September => "septembre",
                October => "octobre",
                November => "novembre",
                December => "décembre",
            },
            Self::CsCz => match month {
                January => "leden",
                February => "únor",
                March => "březen",
                April => "duben",
                May => "květen",
                June => "červen",
                July => "červenec",
                August => "srpen",
                September => "září",
                October => "říjen",
                November => "listopad",
                December => "prosinec",
            },
            Self::UkUa => match month {
                January => "січень",
                February => "лютий",
                March => "березень",
                April => "квітень",
                May => "травень",
                June => "червень",
                July => "липень",
                August => "серпень",
                September => "вересень",
                October => "жовтень",
                November => "листопад",
                December => "грудень",
            },
        }
    }

    /// Returns the genitive form of the month name for this locale.
    ///
    /// For en-US, en-GB, de-DE, fr-FR the genitive is identical to nominative.
    /// Only cs-CZ and uk-UA have distinct genitive forms.
    pub fn month_name_genitive(&self, month: time::Month) -> &'static str {
        use time::Month::*;
        match self {
            Self::EnUs | Self::EnGb | Self::DeDe | Self::FrFr => self.month_name(month),
            Self::CsCz => match month {
                January => "ledna",
                February => "února",
                March => "března",
                April => "dubna",
                May => "května",
                June => "června",
                July => "července",
                August => "srpna",
                September => "září",
                October => "října",
                November => "listopadu",
                December => "prosince",
            },
            Self::UkUa => match month {
                January => "січня",
                February => "лютого",
                March => "березня",
                April => "квітня",
                May => "травня",
                June => "червня",
                July => "липня",
                August => "серпня",
                September => "вересня",
                October => "жовтня",
                November => "листопада",
                December => "грудня",
            },
        }
    }

    /// Format a date for invoice display according to this locale.
    ///
    /// Uses nominative month names for locales without distinct genitive forms
    /// (en-US, en-GB, de-DE, fr-FR) and genitive forms for cs-CZ and uk-UA.
    pub fn format_date(&self, date: time::Date) -> String {
        let day = date.day();
        let month = date.month();
        let year = date.year();

        match self {
            Self::EnUs => format!("{} {day}, {year}", self.month_name(month)),
            Self::EnGb => format!("{day} {} {year}", self.month_name(month)),
            Self::DeDe => format!("{day}. {} {year}", self.month_name(month)),
            Self::FrFr => format!("{day} {} {year}", self.month_name(month)),
            Self::CsCz => format!("{day}. {} {year}", self.month_name_genitive(month)),
            Self::UkUa => format!("{day} {} {year}", self.month_name_genitive(month)),
        }
    }

    /// Format a period (month + year) for display using nominative month name.
    pub fn format_period(&self, month: time::Month, year: i32) -> String {
        format!("{} {year}", self.month_name(month))
    }

    /// Format a number with locale-appropriate decimal and thousands separators.
    pub fn format_number(&self, value: f64, decimals: u32) -> String {
        debug_assert!(!value.is_nan(), "format_number called with NaN");
        debug_assert!(value.is_finite(), "format_number called with Infinity");

        // Extract sign and work with the absolute value to avoid the minus
        // sign being treated as a digit position during thousands-separator
        // insertion.
        let (negative, value) = if value.is_sign_negative() {
            (true, -value)
        } else {
            (false, value)
        };

        let (decimal_sep, thousands_sep) = match self {
            Self::EnUs | Self::EnGb => ('.', ','),
            Self::DeDe => (',', '.'),
            Self::FrFr | Self::CsCz | Self::UkUa => (',', '\u{00A0}'),
        };

        let factor = 10f64.powi(decimals as i32);
        let scaled = (value * factor).round() as u64;

        let int_part = scaled / factor as u64;
        let frac_part = scaled % factor as u64;

        // Format integer part with thousands separators
        let int_str = int_part.to_string();
        let mut with_sep = String::new();
        for (i, ch) in int_str.chars().enumerate() {
            if i > 0 && (int_str.len() - i).is_multiple_of(3) {
                with_sep.push(thousands_sep);
            }
            with_sep.push(ch);
        }

        // Suppress minus sign for negative zero
        let prefix = if negative && int_part == 0 && frac_part == 0 {
            ""
        } else if negative {
            "-"
        } else {
            ""
        };

        if decimals == 0 {
            format!("{prefix}{with_sep}")
        } else {
            format!(
                "{prefix}{with_sep}{decimal_sep}{:0>width$}",
                frac_part,
                width = decimals as usize
            )
        }
    }
}

impl fmt::Display for Locale {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::EnUs => "en-US",
            Self::EnGb => "en-GB",
            Self::DeDe => "de-DE",
            Self::FrFr => "fr-FR",
            Self::CsCz => "cs-CZ",
            Self::UkUa => "uk-UA",
        };
        write!(f, "{s}")
    }
}

impl FromStr for Locale {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "en-us" => Ok(Self::EnUs),
            "en-gb" => Ok(Self::EnGb),
            "de-de" => Ok(Self::DeDe),
            "fr-fr" => Ok(Self::FrFr),
            "cs-cz" => Ok(Self::CsCz),
            "uk-ua" => Ok(Self::UkUa),
            _ => Err(AppError::InvalidLocale {
                key: s.to_string(),
                available: Self::ALL.iter().map(|l| l.to_string()).collect(),
            }),
        }
    }
}

/// Lenient deserializer for the `Defaults.locale` field.
///
/// Attempts to parse the string as a [`Locale`]. If the value is unknown,
/// prints a warning to stderr and falls back to [`Locale::EnUs`].
pub fn deserialize_locale_lenient<'de, D>(deserializer: D) -> Result<Locale, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    match Locale::from_str(&s) {
        Ok(locale) => Ok(locale),
        Err(_) => {
            let available: Vec<String> = Locale::ALL.iter().map(|l| l.to_string()).collect();
            eprintln!(
                "Warning: unknown locale \"{s}\", falling back to en-US. Available: {}",
                available.join(", ")
            );
            Ok(Locale::EnUs)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_en_us() {
        // Arrange & Act
        let locale = Locale::default();

        // Assert
        assert_eq!(locale, Locale::EnUs);
    }

    #[test]
    fn test_all_has_six_variants() {
        // Arrange & Act & Assert
        assert_eq!(Locale::ALL.len(), 6);
    }

    #[test]
    fn test_display_all_variants() {
        // Arrange
        let expected = ["en-US", "en-GB", "de-DE", "fr-FR", "cs-CZ", "uk-UA"];

        // Act & Assert
        for (locale, display) in Locale::ALL.iter().zip(expected.iter()) {
            assert_eq!(format!("{locale}"), *display);
        }
    }

    #[test]
    fn test_from_str_valid_codes() {
        // Arrange
        let codes = ["en-US", "en-GB", "de-DE", "fr-FR", "cs-CZ", "uk-UA"];

        // Act & Assert
        for code in codes {
            let result: Result<Locale, _> = code.parse();
            assert!(result.is_ok(), "Should parse '{code}' as a valid Locale");
        }
    }

    #[test]
    fn test_from_str_case_insensitive() {
        // Arrange & Act & Assert
        assert_eq!("en-us".parse::<Locale>().unwrap(), Locale::EnUs);
        assert_eq!("EN-US".parse::<Locale>().unwrap(), Locale::EnUs);
        assert_eq!("En-Us".parse::<Locale>().unwrap(), Locale::EnUs);
    }

    #[test]
    fn test_from_str_invalid_returns_error() {
        // Arrange & Act
        let result_unknown: Result<Locale, _> = "xx-YY".parse();
        let result_empty: Result<Locale, _> = "".parse();

        // Assert
        assert!(result_unknown.is_err());
        assert!(result_empty.is_err());
        let msg = result_unknown.unwrap_err().to_string();
        assert!(msg.contains("xx-YY"), "Error should contain the invalid key");
        assert!(msg.contains("en-US"), "Error should list available locales");
    }

    #[test]
    fn test_serde_round_trip() {
        // Arrange & Act & Assert
        for locale in Locale::ALL {
            let yaml = serde_yaml::to_string(&locale).unwrap();
            let loaded: Locale = serde_yaml::from_str(&yaml).unwrap();
            assert_eq!(loaded, locale, "Round trip failed for {locale:?}");
        }
    }

    #[test]
    fn test_lenient_deserializer_unknown_value() {
        // Arrange — a Defaults-like struct using the lenient deserializer
        #[derive(Deserialize)]
        struct TestStruct {
            #[serde(deserialize_with = "deserialize_locale_lenient")]
            locale: Locale,
        }

        let yaml = "locale: xx-YY\n";

        // Act
        let result: TestStruct = serde_yaml::from_str(yaml).unwrap();

        // Assert — falls back to en-US
        assert_eq!(result.locale, Locale::EnUs);
    }

    // ── Story 13.2: month_name tests ──

    #[test]
    fn test_month_name_en_us_all_months() {
        // Arrange
        use time::Month::*;
        let locale = Locale::EnUs;
        let expected = [
            (January, "January"), (February, "February"), (March, "March"),
            (April, "April"), (May, "May"), (June, "June"),
            (July, "July"), (August, "August"), (September, "September"),
            (October, "October"), (November, "November"), (December, "December"),
        ];

        // Act & Assert
        for (month, name) in expected {
            assert_eq!(locale.month_name(month), name, "en-US {month:?}");
        }
    }

    #[test]
    fn test_month_name_de_de_all_months() {
        // Arrange
        use time::Month::*;
        let locale = Locale::DeDe;
        let expected = [
            (January, "Januar"), (February, "Februar"), (March, "März"),
            (April, "April"), (May, "Mai"), (June, "Juni"),
            (July, "Juli"), (August, "August"), (September, "September"),
            (October, "Oktober"), (November, "November"), (December, "Dezember"),
        ];

        // Act & Assert
        for (month, name) in expected {
            assert_eq!(locale.month_name(month), name, "de-DE {month:?}");
        }
    }

    #[test]
    fn test_month_name_fr_fr_all_months() {
        // Arrange
        use time::Month::*;
        let locale = Locale::FrFr;
        let expected = [
            (January, "janvier"), (February, "février"), (March, "mars"),
            (April, "avril"), (May, "mai"), (June, "juin"),
            (July, "juillet"), (August, "août"), (September, "septembre"),
            (October, "octobre"), (November, "novembre"), (December, "décembre"),
        ];

        // Act & Assert
        for (month, name) in expected {
            assert_eq!(locale.month_name(month), name, "fr-FR {month:?}");
        }
    }

    #[test]
    fn test_month_name_cs_cz_all_months() {
        // Arrange
        use time::Month::*;
        let locale = Locale::CsCz;
        let expected = [
            (January, "leden"), (February, "únor"), (March, "březen"),
            (April, "duben"), (May, "květen"), (June, "červen"),
            (July, "červenec"), (August, "srpen"), (September, "září"),
            (October, "říjen"), (November, "listopad"), (December, "prosinec"),
        ];

        // Act & Assert
        for (month, name) in expected {
            assert_eq!(locale.month_name(month), name, "cs-CZ {month:?}");
        }
    }

    #[test]
    fn test_month_name_uk_ua_all_months() {
        // Arrange
        use time::Month::*;
        let locale = Locale::UkUa;
        let expected = [
            (January, "січень"), (February, "лютий"), (March, "березень"),
            (April, "квітень"), (May, "травень"), (June, "червень"),
            (July, "липень"), (August, "серпень"), (September, "вересень"),
            (October, "жовтень"), (November, "листопад"), (December, "грудень"),
        ];

        // Act & Assert
        for (month, name) in expected {
            assert_eq!(locale.month_name(month), name, "uk-UA {month:?}");
        }
    }

    #[test]
    fn test_month_name_en_gb_matches_en_us() {
        // Arrange
        use time::Month::*;
        let months = [
            January, February, March, April, May, June,
            July, August, September, October, November, December,
        ];

        // Act & Assert
        for month in months {
            assert_eq!(
                Locale::EnGb.month_name(month),
                Locale::EnUs.month_name(month),
                "en-GB should match en-US for {month:?}"
            );
        }
    }

    // ── Story 13.2: genitive tests ──

    #[test]
    fn test_month_name_genitive_cs_cz_differs_from_nominative() {
        // Arrange
        use time::Month::*;
        let locale = Locale::CsCz;

        // Act & Assert
        assert_ne!(locale.month_name_genitive(January), locale.month_name(January));
        assert_ne!(locale.month_name_genitive(March), locale.month_name(March));
        assert_ne!(locale.month_name_genitive(May), locale.month_name(May));
        assert_eq!(locale.month_name_genitive(January), "ledna");
        assert_eq!(locale.month_name_genitive(March), "března");
        assert_eq!(locale.month_name_genitive(May), "května");
    }

    #[test]
    fn test_month_name_genitive_uk_ua_differs_from_nominative() {
        // Arrange
        use time::Month::*;
        let locale = Locale::UkUa;

        // Act & Assert
        assert_ne!(locale.month_name_genitive(January), locale.month_name(January));
        assert_ne!(locale.month_name_genitive(March), locale.month_name(March));
        assert_ne!(locale.month_name_genitive(May), locale.month_name(May));
        assert_eq!(locale.month_name_genitive(January), "січня");
        assert_eq!(locale.month_name_genitive(March), "березня");
        assert_eq!(locale.month_name_genitive(May), "травня");
    }

    #[test]
    fn test_month_name_genitive_en_us_same_as_nominative() {
        // Arrange
        use time::Month::*;
        let locale = Locale::EnUs;
        let months = [January, March, May, September, December];

        // Act & Assert
        for month in months {
            assert_eq!(locale.month_name_genitive(month), locale.month_name(month));
        }
    }

    #[test]
    fn test_month_name_genitive_de_de_same_as_nominative() {
        // Arrange
        use time::Month::*;
        let locale = Locale::DeDe;
        let months = [January, March, May, September, December];

        // Act & Assert
        for month in months {
            assert_eq!(locale.month_name_genitive(month), locale.month_name(month));
        }
    }

    #[test]
    fn test_month_name_genitive_fr_fr_same_as_nominative() {
        // Arrange
        use time::Month::*;
        let locale = Locale::FrFr;
        let months = [January, March, May, September, December];

        // Act & Assert
        for month in months {
            assert_eq!(locale.month_name_genitive(month), locale.month_name(month));
        }
    }

    // ── Story 13.2: format_period tests ──

    #[test]
    fn test_format_period_en_us() {
        // Arrange
        let locale = Locale::EnUs;

        // Act
        let result = locale.format_period(time::Month::March, 2026);

        // Assert
        assert_eq!(result, "March 2026");
    }

    #[test]
    fn test_format_period_de_de() {
        // Arrange
        let locale = Locale::DeDe;

        // Act
        let result = locale.format_period(time::Month::March, 2026);

        // Assert
        assert_eq!(result, "März 2026");
    }

    #[test]
    fn test_format_period_cs_cz() {
        // Arrange
        let locale = Locale::CsCz;

        // Act
        let result = locale.format_period(time::Month::March, 2026);

        // Assert
        assert_eq!(result, "březen 2026");
    }

    #[test]
    fn test_format_period_uk_ua() {
        // Arrange
        let locale = Locale::UkUa;

        // Act
        let result = locale.format_period(time::Month::March, 2026);

        // Assert
        assert_eq!(result, "березень 2026");
    }

    // ── Story 13.2: format_date tests ──

    #[test]
    fn test_format_date_en_us() {
        // Arrange
        let date = time::Date::from_calendar_date(2026, time::Month::March, 9).unwrap();

        // Act
        let result = Locale::EnUs.format_date(date);

        // Assert
        assert_eq!(result, "March 9, 2026");
    }

    #[test]
    fn test_format_date_en_gb() {
        // Arrange
        let date = time::Date::from_calendar_date(2026, time::Month::March, 9).unwrap();

        // Act
        let result = Locale::EnGb.format_date(date);

        // Assert
        assert_eq!(result, "9 March 2026");
    }

    #[test]
    fn test_format_date_de_de() {
        // Arrange
        let date = time::Date::from_calendar_date(2026, time::Month::March, 9).unwrap();

        // Act
        let result = Locale::DeDe.format_date(date);

        // Assert
        assert_eq!(result, "9. März 2026");
    }

    #[test]
    fn test_format_date_fr_fr() {
        // Arrange
        let date = time::Date::from_calendar_date(2026, time::Month::March, 9).unwrap();

        // Act
        let result = Locale::FrFr.format_date(date);

        // Assert
        assert_eq!(result, "9 mars 2026");
    }

    #[test]
    fn test_format_date_cs_cz() {
        // Arrange
        let date = time::Date::from_calendar_date(2026, time::Month::March, 9).unwrap();

        // Act
        let result = Locale::CsCz.format_date(date);

        // Assert
        assert_eq!(result, "9. března 2026");
    }

    #[test]
    fn test_format_date_uk_ua() {
        // Arrange
        let date = time::Date::from_calendar_date(2026, time::Month::March, 9).unwrap();

        // Act
        let result = Locale::UkUa.format_date(date);

        // Assert
        assert_eq!(result, "9 березня 2026");
    }

    // ── Story 13.2: format_number tests ──

    #[test]
    fn test_format_number_en_us_simple() {
        // Arrange & Act
        let result = Locale::EnUs.format_number(800.0, 2);

        // Assert
        assert_eq!(result, "800.00");
    }

    #[test]
    fn test_format_number_en_us_thousands() {
        // Arrange & Act
        let result = Locale::EnUs.format_number(13000.0, 2);

        // Assert
        assert_eq!(result, "13,000.00");
    }

    #[test]
    fn test_format_number_en_us_large() {
        // Arrange & Act
        let result = Locale::EnUs.format_number(1234567.89, 2);

        // Assert
        assert_eq!(result, "1,234,567.89");
    }

    #[test]
    fn test_format_number_de_de_decimal() {
        // Arrange & Act
        let result = Locale::DeDe.format_number(800.0, 2);

        // Assert
        assert_eq!(result, "800,00");
    }

    #[test]
    fn test_format_number_de_de_thousands() {
        // Arrange & Act
        let result = Locale::DeDe.format_number(13000.0, 2);

        // Assert
        assert_eq!(result, "13.000,00");
    }

    #[test]
    fn test_format_number_uk_ua_thousands() {
        // Arrange & Act
        let result = Locale::UkUa.format_number(13000.0, 2);

        // Assert
        assert_eq!(result, "13\u{00A0}000,00");
    }

    #[test]
    fn test_format_number_zero() {
        // Arrange & Act & Assert
        assert_eq!(Locale::EnUs.format_number(0.0, 2), "0.00");
        assert_eq!(Locale::DeDe.format_number(0.0, 2), "0,00");
    }

    #[test]
    fn test_format_number_exactly_1000() {
        // Arrange & Act & Assert
        assert_eq!(Locale::EnUs.format_number(1000.0, 2), "1,000.00");
        assert_eq!(Locale::DeDe.format_number(1000.0, 2), "1.000,00");
    }

    #[test]
    fn test_format_number_no_decimals() {
        // Arrange & Act & Assert
        assert_eq!(Locale::EnUs.format_number(1234.0, 0), "1,234");
        assert_eq!(Locale::DeDe.format_number(1234.0, 0), "1.234");
    }

    #[test]
    fn test_format_number_one_decimal() {
        // Arrange & Act & Assert
        assert_eq!(Locale::EnUs.format_number(21.0, 1), "21.0");
        assert_eq!(Locale::DeDe.format_number(21.0, 1), "21,0");
    }

    // ── Fix 1: negative value handling ──

    #[test]
    fn test_format_number_negative_value() {
        // Arrange
        let value = -1234.56;

        // Act
        let en_us = Locale::EnUs.format_number(value, 2);
        let de_de = Locale::DeDe.format_number(value, 2);

        // Assert
        assert_eq!(en_us, "-1,234.56");
        assert_eq!(de_de, "-1.234,56");
    }

    #[test]
    fn test_format_number_zero_negative() {
        // Arrange
        let value = -0.0;

        // Act
        let result = Locale::EnUs.format_number(value, 2);

        // Assert — no minus sign for negative zero
        assert_eq!(result, "0.00");
    }

    // ── Fix 5: edge case tests ──

    #[test]
    fn test_format_number_large_value() {
        // Arrange & Act
        let result = Locale::EnUs.format_number(999_999_999.99, 2);

        // Assert
        assert_eq!(result, "999,999,999.99");
    }

    #[test]
    fn test_format_number_very_small_fractional() {
        // Arrange & Act
        let result = Locale::DeDe.format_number(0.01, 2);

        // Assert
        assert_eq!(result, "0,01");
    }
}
