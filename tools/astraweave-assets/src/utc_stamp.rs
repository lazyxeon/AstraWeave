//! UTC text with the same digits the previous clock wrote.
//!
//! A whole second has no fraction. Milliseconds use three digits, microseconds
//! six, and anything finer nine. `zulu` selects the `Z` suffix; otherwise the
//! suffix is `+00:00`.
use time::{OffsetDateTime, UtcOffset};

pub(crate) fn format_utc_rfc3339(dt: OffsetDateTime) -> String {
    format_utc(dt, false)
}

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
        assert_eq!(
            format_utc_rfc3339(at(100_000_000)),
            "2020-01-02T03:04:05.100+00:00"
        );
    }
}
