use chrono::{DateTime, TimeZone, Utc};
use serde::{self, Deserialize, Deserializer, Serializer};
use serde::de;

const FORMAT: &'static str = "%Y-%m-%dT%H:%M:%SZ";

#[allow(dead_code)]
pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
    let s = format!("{}", date.format(FORMAT));
    serializer.serialize_str(&s)
}

pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error> where D: Deserializer<'de> {
    let s = String::deserialize(deserializer)?;
    Utc.datetime_from_str(&s, FORMAT).map_err(de::Error::custom)
}
