//! Wave 2 mutation-resistant remediation tests for hdri_catalog.rs DayPeriod
//!
//! Targets:
//!   - DayPeriod::from_game_hours boundary values (5.0, 10.0, 17.0, 21.0)
//!   - DayPeriod::from_str_loose all aliases
//!   - DayPeriod::as_str golden strings
//!   - DayPeriod::all() count and contents

#[cfg(target_os = "linux")]
#[path = "support/linux_test_threads.rs"]
mod linux_test_threads;

use astraweave_render::hdri_catalog::DayPeriod;

// ===========================================================================
// DayPeriod::from_game_hours — exact boundary tests
// ===========================================================================

#[test]
fn hours_4_99_is_night() {
    assert_eq!(DayPeriod::from_game_hours(4.99), DayPeriod::Night);
}

#[test]
fn hours_5_00_is_morning() {
    assert_eq!(DayPeriod::from_game_hours(5.0), DayPeriod::Morning);
}

#[test]
fn hours_7_00_is_morning() {
    assert_eq!(DayPeriod::from_game_hours(7.0), DayPeriod::Morning);
}

#[test]
fn hours_9_99_is_morning() {
    assert_eq!(DayPeriod::from_game_hours(9.99), DayPeriod::Morning);
}

#[test]
fn hours_10_00_is_day() {
    assert_eq!(DayPeriod::from_game_hours(10.0), DayPeriod::Day);
}

#[test]
fn hours_12_00_is_day() {
    assert_eq!(DayPeriod::from_game_hours(12.0), DayPeriod::Day);
}

#[test]
fn hours_16_99_is_day() {
    assert_eq!(DayPeriod::from_game_hours(16.99), DayPeriod::Day);
}

#[test]
fn hours_17_00_is_evening() {
    assert_eq!(DayPeriod::from_game_hours(17.0), DayPeriod::Evening);
}

#[test]
fn hours_18_00_is_evening() {
    assert_eq!(DayPeriod::from_game_hours(18.0), DayPeriod::Evening);
}

#[test]
fn hours_20_99_is_evening() {
    assert_eq!(DayPeriod::from_game_hours(20.99), DayPeriod::Evening);
}

#[test]
fn hours_21_00_is_night() {
    assert_eq!(DayPeriod::from_game_hours(21.0), DayPeriod::Night);
}

#[test]
fn hours_23_00_is_night() {
    assert_eq!(DayPeriod::from_game_hours(23.0), DayPeriod::Night);
}

#[test]
fn hours_0_00_is_night() {
    assert_eq!(DayPeriod::from_game_hours(0.0), DayPeriod::Night);
}

#[test]
fn hours_2_00_is_night() {
    assert_eq!(DayPeriod::from_game_hours(2.0), DayPeriod::Night);
}

// ===========================================================================
// DayPeriod::from_game_hours — wrapping (>24)
// ===========================================================================

#[test]
fn hours_36_is_noon() {
    // 36 mod 24 = 12 → Day
    assert_eq!(DayPeriod::from_game_hours(36.0), DayPeriod::Day);
}

#[test]
fn hours_negative_wraps() {
    // -1 rem_euclid 24 = 23 → Night
    assert_eq!(DayPeriod::from_game_hours(-1.0), DayPeriod::Night);
}

#[test]
fn hours_negative_morning() {
    // -17 rem_euclid 24 = 7 → Morning
    assert_eq!(DayPeriod::from_game_hours(-17.0), DayPeriod::Morning);
}

// ===========================================================================
// DayPeriod::from_str_loose — all known aliases
// ===========================================================================

#[test]
fn parse_day() {
    assert_eq!(DayPeriod::from_str_loose("day"), Some(DayPeriod::Day));
}

#[test]
fn parse_day_uppercase() {
    assert_eq!(DayPeriod::from_str_loose("Day"), Some(DayPeriod::Day));
    assert_eq!(DayPeriod::from_str_loose("DAY"), Some(DayPeriod::Day));
}

#[test]
fn parse_morning_aliases() {
    assert_eq!(
        DayPeriod::from_str_loose("morning"),
        Some(DayPeriod::Morning)
    );
    assert_eq!(
        DayPeriod::from_str_loose("sunrise"),
        Some(DayPeriod::Morning)
    );
    assert_eq!(DayPeriod::from_str_loose("dawn"), Some(DayPeriod::Morning));
}

#[test]
fn parse_evening_aliases() {
    assert_eq!(
        DayPeriod::from_str_loose("evening"),
        Some(DayPeriod::Evening)
    );
    assert_eq!(
        DayPeriod::from_str_loose("sunset"),
        Some(DayPeriod::Evening)
    );
    assert_eq!(DayPeriod::from_str_loose("dusk"), Some(DayPeriod::Evening));
}

#[test]
fn parse_night_aliases() {
    assert_eq!(DayPeriod::from_str_loose("night"), Some(DayPeriod::Night));
    assert_eq!(
        DayPeriod::from_str_loose("midnight"),
        Some(DayPeriod::Night)
    );
}

#[test]
fn parse_whitespace_trimmed() {
    assert_eq!(DayPeriod::from_str_loose("  day  "), Some(DayPeriod::Day));
    assert_eq!(
        DayPeriod::from_str_loose("\tnight\n"),
        Some(DayPeriod::Night)
    );
}

#[test]
fn parse_invalid_returns_none() {
    assert_eq!(DayPeriod::from_str_loose("invalid"), None);
    assert_eq!(DayPeriod::from_str_loose("noon"), None);
    assert_eq!(DayPeriod::from_str_loose(""), None);
}

// ===========================================================================
// DayPeriod::as_str — golden strings
// ===========================================================================

#[test]
fn as_str_day() {
    assert_eq!(DayPeriod::Day.as_str(), "day");
}

#[test]
fn as_str_morning() {
    assert_eq!(DayPeriod::Morning.as_str(), "morning");
}

#[test]
fn as_str_evening() {
    assert_eq!(DayPeriod::Evening.as_str(), "evening");
}

#[test]
fn as_str_night() {
    assert_eq!(DayPeriod::Night.as_str(), "night");
}

// ===========================================================================
// DayPeriod::all()
// ===========================================================================

#[test]
fn all_has_four_periods() {
    assert_eq!(DayPeriod::all().len(), 4);
}

#[test]
fn all_contains_all_variants() {
    let all = DayPeriod::all();
    assert!(all.contains(&DayPeriod::Day));
    assert!(all.contains(&DayPeriod::Morning));
    assert!(all.contains(&DayPeriod::Evening));
    assert!(all.contains(&DayPeriod::Night));
}

// ===========================================================================
// Roundtrip: from_str_loose -> as_str
// ===========================================================================

#[test]
fn roundtrip_parse_display() {
    for period in DayPeriod::all() {
        let s = period.as_str();
        let parsed = DayPeriod::from_str_loose(s);
        assert_eq!(parsed, Some(*period), "roundtrip failed for {:?}", period);
    }
}
