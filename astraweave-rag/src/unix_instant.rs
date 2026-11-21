//! Unix seconds for 2020-01-02 03:04:05 UTC. Subsecond digits do not change the second.

#[test]
fn unix_timestamp_matches_pinned_instant() {
    let whole = time::OffsetDateTime::from_unix_timestamp(1_577_934_245).unwrap();
    assert_eq!(whole.unix_timestamp(), 1_577_934_245);
    let subsec =
        time::OffsetDateTime::from_unix_timestamp_nanos(1_577_934_245_100_000_000).unwrap();
    assert_eq!(subsec.unix_timestamp(), 1_577_934_245);
}
