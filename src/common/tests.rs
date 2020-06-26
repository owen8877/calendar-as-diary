use super::*;
use chrono::TimeZone;

#[test]
fn test_get_offset_on() {
    let config = DaylightSavingConfig {
        start: (3, 11),
        end: (11, 4),
        effective: -5,
        standard: -6,
        local: 0
    };

    assert_eq!(config.get_offset_on(&Local.ymd(2020, 1, 2)), -6);
    assert_eq!(config.get_offset_on(&Local.ymd(2020, 3, 1)), -6);
    assert_eq!(config.get_offset_on(&Local.ymd(2020, 3, 15)), -5);
    assert_eq!(config.get_offset_on(&Local.ymd(2020, 11, 1)), -5);
    assert_eq!(config.get_offset_on(&Local.ymd(2020, 11, 12)), -6);
    assert_eq!(config.get_offset_on(&Local.ymd(2020, 12, 2)), -6);
}
