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

pub(crate) fn format_utc_rfc3339(dt: OffsetDateTime) -> String {
    format_utc(dt, false)
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
        assert_eq!(format_utc_rfc3339(civil(-1)), "-0001-01-02T03:04:05+00:00");
    }
}
