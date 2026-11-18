//! Wall-clock stamps for autosave names and the session lock file.
//!
//! The numbers are the local civil time. If the platform cannot report a local
//! offset, the stamp uses UTC.

use time::OffsetDateTime;

/// Four digits for years `0..=9999`. Any other year keeps a sign and at least
/// four digits of magnitude.
fn year_text(year: i32) -> String {
    if !(0..=9999).contains(&year) {
        format!("{year:+05}")
    } else {
        format!("{year:04}")
    }
}

pub fn now() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
}

pub fn compact(dt: OffsetDateTime) -> String {
    format!(
        "{}{:02}{:02}_{:02}{:02}{:02}",
        year_text(dt.year()),
        u8::from(dt.month()),
        dt.day(),
        dt.hour(),
        dt.minute(),
        dt.second()
    )
}

pub fn readable(dt: OffsetDateTime) -> String {
    format!(
        "{}-{:02}-{:02} {:02}:{:02}:{:02}",
        year_text(dt.year()),
        u8::from(dt.month()),
        dt.day(),
        dt.hour(),
        dt.minute(),
        dt.second()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::{Date, Month, OffsetDateTime, PrimitiveDateTime, Time, UtcOffset};

    fn wall(offset_hours: i8) -> OffsetDateTime {
        let date = Date::from_calendar_date(2020, Month::January, 2).unwrap();
        let time_of_day = Time::from_hms(3, 4, 5).unwrap();
        PrimitiveDateTime::new(date, time_of_day)
            .assume_offset(UtcOffset::from_hms(offset_hours, 0, 0).unwrap())
    }

    #[test]
    fn wall_clock_stamps_keep_the_civil_time() {
        assert_eq!(compact(wall(-5)), "20200102_030405");
        assert_eq!(readable(wall(-5)), "2020-01-02 03:04:05");
        assert_eq!(compact(wall(0)), "20200102_030405");
        assert_eq!(readable(wall(0)), "2020-01-02 03:04:05");
    }

    fn civil(year: i32) -> OffsetDateTime {
        let date = Date::from_calendar_date(year, Month::January, 2).unwrap();
        let clock = Time::from_hms(3, 4, 5).unwrap();
        PrimitiveDateTime::new(date, clock).assume_utc()
    }

    #[test]
    fn year_width_keeps_a_sign_and_four_digit_magnitude() {
        assert_eq!(compact(civil(-1)), "-00010102_030405");
        assert_eq!(readable(civil(-1)), "-0001-01-02 03:04:05");
        assert_eq!(compact(civil(0)), "00000102_030405");
        assert_eq!(readable(civil(9999)), "9999-01-02 03:04:05");
        assert_eq!(year_text(10_000), "+10000");
        assert_eq!(year_text(-10_000), "-10000");
    }
}
