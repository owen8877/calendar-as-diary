use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::{io, fs};
use std::path::Path;

use reqwest::header::*;
use serde::{de, Deserialize, Serialize};
use serde_json as json;

use crate::calendar::event::*;
use std::borrow::BorrowMut;

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
    pub fn new(source: &str, calendar_id: Option<String>) -> RequestConfig {
        let mut config = RequestConfig {
            url: String::from(""),
            calendar_id: String::from(""),
            headers: HeaderMap::new(),
        };

        match read_json::<RequestConfigJson>(format!("config/{}.json.default", source).as_str()) {
            Ok(default_config) => {
                debug!("Default {} config file loaded.", source);
                headers_modifier(&default_config.headers, &mut config.headers);
                config.url = default_config.url;
                config.calendar_id = default_config.calendar_id;
            },
            Err(e) => panic!("Default {} config not found! {}", source, e),
        }
        match read_json::<RequestConfigJson>(format!("config/{}.json", source).as_str()) {
            Ok(custom_config) => {
                debug!("Custom {} config file loaded.", source);
                headers_modifier(&custom_config.headers, &mut config.headers);
                config.url = custom_config.url;
                config.calendar_id = custom_config.calendar_id;
            }
            Err(e) => info!("Custom {} config file not found, falling back to default file. {}", source, e),
        }

        if let Some(calendar_id) = calendar_id {
            config.calendar_id = calendar_id;
        }

        config
    }
}

pub trait Module {
    fn new(calendar_id: Option<String>) -> Self where Self: Sized;
    fn dump(&self);
    fn get_config(&self) -> &RequestConfig;
    fn get_event_ids(&mut self) -> &mut HashSet<String>;
    fn process_response_into_event_with_id(&self, response: String) -> Vec<EventWithId>;
}

pub fn headers_modifier(headers: &HashMap<String, String>, header_map: &mut HeaderMap) {
    let header_dict: HashMap<&str, HeaderName> = get_header_dict();

    for key in headers.keys() {
        match header_dict.get(key.as_str()) {
            Some(header) => {
                debug!("Inserted header {} with value {}.", key, headers[key]);
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

pub fn read_dumped_event_id(identifier: &str) -> HashSet<String> {
    match read_json::<HashSet<String>>(format!("dump/{}.json", identifier).as_str()) {
        Ok(set) => set,
        Err(_) => HashSet::new(),
    }
}

pub fn path_exists(path: &str) -> bool {
    fs::metadata(path).is_ok()
}

pub fn ensure_directory(path: &str) {
    if !path_exists(path) {
        fs::create_dir(path);
    }
}

pub fn write_json<T: Serialize>(file_path: &str, object: &T) -> Result<(), io::Error> {
    match serde_json::to_string(object) {
        Ok(serialized) => fs::write(file_path, serialized),
        Err(e) => Err(e.into()),
    }
}

pub fn dump_event_id_wrapper(identifier: &str, ids: &HashSet<String>) {
    ensure_directory("dump");
    write_json::<HashSet<String>>(format!("dump/{}.json", identifier).as_str(), ids);
}