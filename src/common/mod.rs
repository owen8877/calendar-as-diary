use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::Path;

use calendar3::{Event, EventDateTime};
use chrono::{Date, DateTime, Utc};
use reqwest::header::*;
use serde::{de, Deserialize};
use serde_json as json;

pub trait HistoryItem {
    fn hash(&self) -> String;
}

#[derive(Debug, Deserialize)]
pub struct RequestConfigJson {
    pub url: String,
    pub calendar_id: String,
    pub headers: HashMap<String, String>,
}

pub struct RequestConfig {
    pub url: String,
    pub calendar_id: String,
    pub headers: HeaderMap,
}

pub fn headers_modifier(headers: &HashMap<String, String>, header_map: &mut HeaderMap) {
    let header_dict: HashMap<&str, HeaderName> = get_header_dict();

    for key in headers.keys() {
        match header_dict.get(key.as_str()) {
            Some(header) => {
                header_map.insert(header, headers[key].parse().unwrap());
            },
            None => panic!("Unknown header {}.", key),
        }
    }
}

fn get_header_dict() -> HashMap<&'static str, HeaderName> {
    let mut dict = HashMap::<&str, HeaderName>::new();
    dict.insert("accept",                    ACCEPT);
    dict.insert("accept-language",           ACCEPT_LANGUAGE);
    dict.insert("cache-control",             CACHE_CONTROL);
    dict.insert("cookie",                    COOKIE);
    dict.insert("dnt",                       DNT);
    dict.insert("referer",                   REFERER);
    dict.insert("upgrade-insecure-requests", UPGRADE_INSECURE_REQUESTS);
    dict.insert("user-agent",                USER_AGENT);

    dict
}

pub fn read_json<T: de::DeserializeOwned>(file_path: &str) -> Result<T, io::Error> {
    match File::open(Path::new(file_path)) {
        Ok(file) => {
            match json::from_reader::<File, T>(file) {
                Ok(result) => Ok(result),
                Err(e) => Err(e.into()),
            }
        },
        Err(e) => Err(e),
    }
}

pub struct SimpleEvent {
    pub summary: String,
    pub description: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

pub struct SimpleWholeDayEvent {
    pub summary: String,
    pub description: String,
    pub date: Date<Utc>,
}

impl From<SimpleEvent> for Event {
    fn from(item: SimpleEvent) -> Self {
        Event {
            summary: Some(item.summary),
            description: Some(item.description),
            start: Some(EventDateTime {
                date_time: Some(item.start.to_rfc3339()),
                ..EventDateTime::default()
            }),
            end: Some(EventDateTime {
                date_time: Some(item.end.to_rfc3339()),
                ..EventDateTime::default()
            }),
            ..Event::default()
        }
    }
}

impl From<SimpleWholeDayEvent> for Event {
    fn from(item: SimpleWholeDayEvent) -> Self {
        Event {
            summary: Some(item.summary),
            description: Some(item.description),
            start: Some(EventDateTime {
                date: Some(item.date.format("%Y-%m-%d").to_string()),
                ..EventDateTime::default()
            }),
            end: Some(EventDateTime {
                date: Some(item.date.format("%Y-%m-%d").to_string()),
                ..EventDateTime::default()
            }),
            ..Event::default()
        }
    }
}