//! UTC text with the same digits the previous clock wrote.
//!
//! A whole second has no fraction. Milliseconds use three digits, microseconds
//! six, and anything finer nine. `zulu` selects the `Z` suffix; otherwise the
//! suffix is `+00:00`. Years in `0..=9999` use four digits. Every other year
//! keeps a sign and at least four digits of magnitude.
use time::{OffsetDateTime, UtcOffset};

/// Four digits for years `0..=9999`. Any other year keeps a sign and at least
/// four digits of magnitude.
fn year_text(year: i32) -> String {
    if !(0..=9999).contains(&year) {
        format!("{year:+05}")
    } else {
        format!("{year:04}")
    }
}

pub fn format_utc_seconds(dt: OffsetDateTime) -> String {
    let utc = dt.to_offset(UtcOffset::UTC);
    format!(
        "{}-{:02}-{:02} {:02}:{:02}:{:02}",
        year_text(utc.year()),
        u8::from(utc.month()),
        utc.day(),
        utc.hour(),
        utc.minute(),
        utc.second()
    )
}

pub(crate) fn format_utc_display(dt: OffsetDateTime) -> String {
    let utc = dt.to_offset(UtcOffset::UTC);
    let mut out = format_utc_seconds(utc);
    push_fraction(&mut out, utc.nanosecond());
    out.push_str(" UTC");
    out
}

pub(crate) fn format_utc(dt: OffsetDateTime, zulu: bool) -> String {
    let utc = dt.to_offset(UtcOffset::UTC);
    let mut out = format!(
        "{}-{:02}-{:02}T{:02}:{:02}:{:02}",
        year_text(utc.year()),
        u8::from(utc.month()),
        utc.day(),
        utc.hour(),
        utc.minute(),
        utc.second()
    );
    push_fraction(&mut out, utc.nanosecond());
    if zulu {
        out.push('Z');
    } else {
        out.push_str("+00:00");
    }
    out
}

fn push_fraction(out: &mut String, nano: u32) {
    if nano == 0 {
        return;
    }
    if nano % 1_000_000 == 0 {
        out.push_str(&format!(".{:03}", nano / 1_000_000));
    } else if nano % 1_000 == 0 {
        out.push_str(&format!(".{:06}", nano / 1_000));
    } else {
        out.push_str(&format!(".{nano:09}"));
    }
}

fn invalid_year() -> time::error::Parse {
    time::error::Parse::ParseFromDescription(time::error::ParseFromDescription::InvalidComponent(
        "year",
    ))
}

pub(crate) fn parse_utc_rfc3339(text: &str) -> Result<OffsetDateTime, time::error::Parse> {
    let rfc3339 = &time::format_description::well_known::Rfc3339;
    if text.starts_with('+') || text.starts_with('-') {
        parse_signed_year(text, rfc3339)
    } else {
        OffsetDateTime::parse(text, rfc3339).map(|dt| dt.to_offset(UtcOffset::UTC))
    }
}

/// A leading sign is outside `time`'s RFC 3339 year. The magnitude is at least
/// four digits. The remainder keeps its offset, then the result is stored as UTC.
fn parse_signed_year(
    text: &str,
    rfc3339: &time::format_description::well_known::Rfc3339,
) -> Result<OffsetDateTime, time::error::Parse> {
    let negative = text.starts_with('-');
    let rest = &text[1..];
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits < 4 {
        return Err(invalid_year());
    }
    let magnitude: i32 = rest[..digits].parse().map_err(|_| invalid_year())?;
    let year = if negative {
        magnitude.checked_neg().ok_or_else(invalid_year)?
    } else {
        magnitude
    };
    let parsed = OffsetDateTime::parse(&format!("1970{}", &rest[digits..]), rfc3339)?;
    let date = time::Date::from_calendar_date(year, parsed.month(), parsed.day())
        .map_err(|_| invalid_year())?;
    OffsetDateTime::new_in_offset(date, parsed.time(), parsed.offset())
        .checked_to_offset(UtcOffset::UTC)
        .ok_or_else(invalid_year)
}

pub(crate) mod serde_rfc3339 {
    use super::{format_utc, parse_utc_rfc3339};
    use serde::{Deserialize, Deserializer, Serializer};
    use time::OffsetDateTime;

    pub fn serialize<S: Serializer>(dt: &OffsetDateTime, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format_utc(*dt, true))
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<OffsetDateTime, D::Error> {
        let text = String::deserialize(deserializer)?;
        parse_utc_rfc3339(&text).map_err(serde::de::Error::custom)
    }

    #[allow(dead_code)]
    pub mod option {
        use super::super::{format_utc, parse_utc_rfc3339};
        use serde::{Deserialize, Deserializer, Serializer};
        use time::OffsetDateTime;

        pub fn serialize<S: Serializer>(
            dt: &Option<OffsetDateTime>,
            serializer: S,
        ) -> Result<S::Ok, S::Error> {
            match dt {
                Some(value) => serializer.serialize_some(&format_utc(*value, true)),
                None => serializer.serialize_none(),
            }
        }

        pub fn deserialize<'de, D: Deserializer<'de>>(
            deserializer: D,
        ) -> Result<Option<OffsetDateTime>, D::Error> {
            let text = Option::<String>::deserialize(deserializer)?;
            text.map(|value| parse_utc_rfc3339(&value).map_err(serde::de::Error::custom))
                .transpose()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(nanos: i128) -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp_nanos(1_577_934_245_000_000_000 + nanos).unwrap()
    }

    #[test]
    fn utc_text_matches_pinned_strings() {
        assert_eq!(format_utc(at(0), false), "2020-01-02T03:04:05+00:00");
        assert_eq!(format_utc(at(0), true), "2020-01-02T03:04:05Z");
        assert_eq!(
            format_utc(at(100_000_000), false),
            "2020-01-02T03:04:05.100+00:00"
        );
        assert_eq!(
            format_utc(at(100_000_000), true),
            "2020-01-02T03:04:05.100Z"
        );
        assert_eq!(
            format_utc(at(100_000), false),
            "2020-01-02T03:04:05.000100+00:00"
        );
        assert_eq!(format_utc(at(100_000), true), "2020-01-02T03:04:05.000100Z");
        assert_eq!(
            format_utc(at(1_000_000), false),
            "2020-01-02T03:04:05.001+00:00"
        );
        assert_eq!(format_utc(at(1_000_000), true), "2020-01-02T03:04:05.001Z");
        assert_eq!(
            format_utc(at(1), false),
            "2020-01-02T03:04:05.000000001+00:00"
        );
        assert_eq!(format_utc(at(1), true), "2020-01-02T03:04:05.000000001Z");
        assert_eq!(
            format_utc(at(123_456_789), false),
            "2020-01-02T03:04:05.123456789+00:00"
        );
        assert_eq!(
            format_utc(at(123_456_789), true),
            "2020-01-02T03:04:05.123456789Z"
        );
        assert_eq!(
            format_utc(at(123_400_000), false),
            "2020-01-02T03:04:05.123400+00:00"
        );
        assert_eq!(
            format_utc(at(123_400_000), true),
            "2020-01-02T03:04:05.123400Z"
        );
        assert_eq!(format_utc_seconds(at(123_456_789)), "2020-01-02 03:04:05");
        assert_eq!(format_utc_display(at(0)), "2020-01-02 03:04:05 UTC");
        assert_eq!(
            format_utc_display(at(100_000_000)),
            "2020-01-02 03:04:05.100 UTC"
        );
    }

    #[test]
    fn parsed_offsets_become_utc() {
        assert_eq!(
            format_utc(parse_utc_rfc3339("2020-01-02T03:04:05Z").unwrap(), true),
            "2020-01-02T03:04:05Z"
        );
        assert_eq!(
            format_utc(
                parse_utc_rfc3339("2020-01-02T03:04:05+00:00").unwrap(),
                true
            ),
            "2020-01-02T03:04:05Z"
        );
        assert_eq!(
            format_utc(
                parse_utc_rfc3339("2020-01-02T08:04:05+05:00").unwrap(),
                true
            ),
            "2020-01-02T03:04:05Z"
        );
        assert_eq!(
            format_utc(parse_utc_rfc3339("2020-01-02T03:04:05.100Z").unwrap(), true),
            "2020-01-02T03:04:05.100Z"
        );
        assert_eq!(
            format_utc(
                parse_utc_rfc3339("2020-01-02T03:04:05.000100+00:00").unwrap(),
                true
            ),
            "2020-01-02T03:04:05.000100Z"
        );
        assert_eq!(
            format_utc(
                parse_utc_rfc3339("2020-01-02T03:04:05.000000001Z").unwrap(),
                true
            ),
            "2020-01-02T03:04:05.000000001Z"
        );
        assert_eq!(
            format_utc(
                parse_utc_rfc3339("2020-01-02T03:04:05.123400Z").unwrap(),
                true
            ),
            "2020-01-02T03:04:05.123400Z"
        );
        assert_eq!(
            format_utc(
                parse_utc_rfc3339("2020-01-02T03:04:05.123456789+00:00").unwrap(),
                true
            ),
            "2020-01-02T03:04:05.123456789Z"
        );
    }

    #[test]
    fn toml_and_postcard_match_pinned_zulu_string() {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct Row {
            #[serde(with = "super::serde_rfc3339")]
            timestamp: OffsetDateTime,
        }
        let row = Row {
            timestamp: at(100_000_000),
        };
        assert_eq!(
            toml::to_string(&row).unwrap(),
            "timestamp = \"2020-01-02T03:04:05.100Z\"\n"
        );
        assert_eq!(
            postcard::to_allocvec(&row).unwrap(),
            b"\x182020-01-02T03:04:05.100Z".to_vec()
        );
        let whole = Row { timestamp: at(0) };
        assert_eq!(
            postcard::to_allocvec(&whole).unwrap(),
            b"\x142020-01-02T03:04:05Z".to_vec()
        );
        let back: Row = toml::from_str("timestamp = \"2020-01-02T08:04:05+05:00\"\n").unwrap();
        assert_eq!(format_utc(back.timestamp, true), "2020-01-02T03:04:05Z");
    }

    fn civil(year: i32) -> OffsetDateTime {
        let date = time::Date::from_calendar_date(year, time::Month::January, 2).unwrap();
        let clock = time::Time::from_hms(3, 4, 5).unwrap();
        time::PrimitiveDateTime::new(date, clock).assume_utc()
    }

    #[test]
    fn year_width_keeps_a_sign_and_four_digit_magnitude() {
        assert_eq!(format_utc(civil(-1), true), "-0001-01-02T03:04:05Z");
        assert_eq!(format_utc(civil(-1), false), "-0001-01-02T03:04:05+00:00");
        assert_eq!(format_utc(civil(0), true), "0000-01-02T03:04:05Z");
        assert_eq!(format_utc(civil(9999), true), "9999-01-02T03:04:05Z");
        assert_eq!(format_utc(civil(-9999), true), "-9999-01-02T03:04:05Z");
        assert_eq!(year_text(10_000), "+10000");
        assert_eq!(year_text(-10_000), "-10000");
        assert_eq!(year_text(-1), "-0001");
        assert_eq!(year_text(2020), "2020");
        assert_eq!(format_utc_seconds(civil(-1)), "-0001-01-02 03:04:05");
        assert_eq!(format_utc_display(civil(-1)), "-0001-01-02 03:04:05 UTC");
    }

    #[test]
    fn signed_years_parse_back_to_utc() {
        assert_eq!(
            format_utc(parse_utc_rfc3339("-0001-01-02T03:04:05Z").unwrap(), true),
            "-0001-01-02T03:04:05Z"
        );
        assert_eq!(
            format_utc(
                parse_utc_rfc3339("-0001-01-01T01:00:00+05:00").unwrap(),
                true
            ),
            "-0002-12-31T20:00:00Z"
        );
        assert_eq!(
            format_utc(parse_utc_rfc3339("+2020-01-02T03:04:05Z").unwrap(), true),
            "2020-01-02T03:04:05Z"
        );
        assert_eq!(
            format_utc(parse_utc_rfc3339("+0000-01-02T03:04:05Z").unwrap(), true),
            "0000-01-02T03:04:05Z"
        );
        assert!(parse_utc_rfc3339("-001-01-02T03:04:05Z").is_err());
        assert!(parse_utc_rfc3339("+10000-01-02T03:04:05Z").is_err());
        assert!(parse_utc_rfc3339("-10000-01-02T03:04:05Z").is_err());
    }

    #[test]
    fn option_timestamp_reads_a_present_string_null_and_omission() {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct Opt {
            #[serde(default, with = "super::serde_rfc3339::option")]
            timestamp: Option<OffsetDateTime>,
        }
        let present: Opt =
            serde_json::from_str("{\"timestamp\":\"2020-01-02T03:04:05Z\"}").unwrap();
        assert_eq!(present.timestamp, Some(at(0)));
        let null: Opt = serde_json::from_str("{\"timestamp\":null}").unwrap();
        assert!(null.timestamp.is_none());
        let omitted: Opt = serde_json::from_str("{}").unwrap();
        assert!(omitted.timestamp.is_none());
    }
}
