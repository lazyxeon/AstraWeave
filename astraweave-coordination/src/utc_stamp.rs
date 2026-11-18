//! UTC text with the same digits the previous clock wrote.
//!
//! A whole second has no fraction. Milliseconds use three digits, microseconds
//! six, and anything finer nine. `zulu` selects the `Z` suffix; otherwise the
//! suffix is `+00:00`.
use time::{OffsetDateTime, UtcOffset};

pub(crate) fn format_utc(dt: OffsetDateTime, zulu: bool) -> String {
    let utc = dt.to_offset(UtcOffset::UTC);
    let mut out = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}",
        utc.year(),
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

pub(crate) fn parse_utc_rfc3339(text: &str) -> Result<OffsetDateTime, time::error::Parse> {
    OffsetDateTime::parse(text, &time::format_description::well_known::Rfc3339)
        .map(|dt| dt.to_offset(UtcOffset::UTC))
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

pub(crate) fn saturating_std(
    duration: time::Duration,
    fallback: std::time::Duration,
) -> std::time::Duration {
    std::time::Duration::try_from(duration).unwrap_or(fallback)
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
    fn serde_json_matches_pinned_zulu_string() {
        #[derive(serde::Serialize, serde::Deserialize)]
        struct Row {
            #[serde(with = "super::serde_rfc3339")]
            timestamp: OffsetDateTime,
        }
        #[derive(serde::Serialize, serde::Deserialize)]
        struct Opt {
            #[serde(with = "super::serde_rfc3339::option")]
            timestamp: Option<OffsetDateTime>,
        }
        let row = Row {
            timestamp: at(100_000_000),
        };
        assert_eq!(
            serde_json::to_string(&row).unwrap(),
            "{\"timestamp\":\"2020-01-02T03:04:05.100Z\"}"
        );
        let back: Row =
            serde_json::from_str("{\"timestamp\":\"2020-01-02T08:04:05+05:00\"}").unwrap();
        assert_eq!(format_utc(back.timestamp, true), "2020-01-02T03:04:05Z");
        assert_eq!(
            serde_json::to_string(&Opt { timestamp: None }).unwrap(),
            "{\"timestamp\":null}"
        );
        assert_eq!(
            serde_json::to_string(&Opt {
                timestamp: Some(at(0))
            })
            .unwrap(),
            "{\"timestamp\":\"2020-01-02T03:04:05Z\"}"
        );
    }
}
