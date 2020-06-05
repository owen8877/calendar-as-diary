use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::Path;

use calendar3::{Event, EventDateTime};
use chrono::{Date, DateTime, Utc};
use reqwest::header::*;
use reqwest::Response;
use serde::{de, Deserialize};
use serde_json as json;

use async_trait::async_trait;

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

impl RequestConfig {
    pub(crate) fn new(source: &str) -> RequestConfig {
        let mut config = RequestConfig {
            url: String::from(""),
            calendar_id: String::from(""),
            headers: HeaderMap::new(),
        };

        match read_json::<RequestConfigJson>(format!("config/{}.json.default", source).as_str()) {
            Ok(default_config) => {
                headers_modifier(&default_config.headers, &mut config.headers);
                config.url = default_config.url;
                config.calendar_id = default_config.calendar_id;
            },
            Err(e) => panic!("Default {} config not found! {}", source, e),
        }
        match read_json::<RequestConfigJson>(format!("config/{}.json", source).as_str()) {
            Ok(custom_config) => {
                headers_modifier(&custom_config.headers, &mut config.headers);
                config.url = custom_config.url;
                config.calendar_id = custom_config.calendar_id;
            }
            Err(e) => println!("{} config file not found, falling back to default file. {}", source, e),
        }

        config
    }
}

#[async_trait]
pub trait Module {
    fn new() -> Self where Self: Sized;
    fn get_config(&self) -> &RequestConfig;
    async fn process_response_into_event_with_id(&self, response: Response) -> Result<Vec<EventWithId>, Box<dyn std::error::Error>>;
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

pub struct PartialDayEvent {
    pub summary: String,
    pub description: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

pub struct WholeDayEvent {
    pub summary: String,
    pub description: String,
    pub date: Date<Utc>,
}

impl From<PartialDayEvent> for Event {
    fn from(item: PartialDayEvent) -> Self {
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

impl From<WholeDayEvent> for Event {
    fn from(item: WholeDayEvent) -> Self {
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

pub struct EventWithId {
    pub event: Event,
    pub id: String,
}

impl EventWithId {
    pub fn new(event: Event, id: String) -> EventWithId {
        EventWithId {
            event,
            id,
        }
    }
}