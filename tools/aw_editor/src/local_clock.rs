//! Wall-clock stamps for autosave names and the session lock file.
//!
//! The numbers are the local civil time. If the platform cannot report a local
//! offset, the stamp uses UTC.

use time::OffsetDateTime;

pub fn now() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
}

pub fn compact(dt: OffsetDateTime) -> String {
    format!(
        "{:04}{:02}{:02}_{:02}{:02}{:02}",
        dt.year(),
        u8::from(dt.month()),
        dt.day(),
        dt.hour(),
        dt.minute(),
        dt.second()
    )
}

pub fn readable(dt: OffsetDateTime) -> String {
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        dt.year(),
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
}
