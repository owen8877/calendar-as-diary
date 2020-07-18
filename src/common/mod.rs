use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::{io, fs};
use std::path::Path;

use reqwest::header::*;
use serde::{de, Deserialize, Serialize};
use serde_json as json;

use crate::calendar::event::*;
use std::error::Error;
use chrono::{Date, Local, Datelike};

pub mod utc_date_format;

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
    pub fn new(source: &str, calendar_id: Option<String>) -> Result<RequestConfig, Box<dyn Error>> {
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
            Err(e) => {
                warn!("Default {} config not found! {}", source, e);
                return Err(Box::new(e))
            },
        }
        match read_json::<RequestConfigJson>(format!("config/{}.json", source).as_str()) {
            Ok(custom_config) => {
                debug!("Custom {} config file loaded.", source);
                headers_modifier(&custom_config.headers, &mut config.headers);
                config.url = custom_config.url;
                config.calendar_id = custom_config.calendar_id;
            }
            Err(e) => {
                warn!("Custom {} config file not found! {}", source, e);
                return Err(Box::new(e))
            },
        }

        if let Some(calendar_id) = calendar_id {
            config.calendar_id = calendar_id;
        }

        Ok(config)
    }
}

pub trait Module {
    fn new(calendar_id: Option<String>) -> Result<Box<dyn Module>, Box<dyn Error>> where Self: Sized;
    fn dump(&self);
    fn get_config(&self) -> &RequestConfig;
    fn get_event_ids(&mut self) -> &mut HashSet<String>;
    fn get_identifier(&self) -> &str;
    fn get_request_url(&self) -> String;
    fn process_response_into_event_with_id(&self, response: String) -> Result<Vec<EventWithId>, Box<dyn Error>>;
}

pub fn filter_loaded_modules(modules: Vec<Result<Box<dyn Module>, Box<dyn Error>>>) -> Vec<Box<dyn Module>> {
    modules.into_iter().filter_map(|module_res| {
        module_res
            .map(|module| {
                info!("Loaded module {}.", module.get_identifier());
                module
            })
            .map_err(|e| {
                info!("Error raised in loading module: {}.", e);
                e
            }).ok()
    }).collect()
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
    dict.insert("authorization",             AUTHORIZATION);
    dict.insert("cache-control",             CACHE_CONTROL);
    dict.insert("cookie",                    COOKIE);
    dict.insert("dnt",                       DNT);
    dict.insert("origin",                    ORIGIN);
    dict.insert("referer",                   REFERER);
    dict.insert("upgrade-insecure-requests", UPGRADE_INSECURE_REQUESTS);
    dict.insert("user-agent",                USER_AGENT);

    dict
}

#[derive(Debug, Deserialize)]
pub struct DaylightSavingConfigWrapper {
    pub daylight_saving: DaylightSavingConfig,
}

#[derive(Debug, Deserialize)]
pub struct DaylightSavingConfig {
    pub start: (u32, u32), // date when daylight saving is effective, normally in spring
    pub end: (u32, u32), // date when daylight saving is no longer effective, normally in fall
    pub effective: i32, // daylight saving timezone
    pub standard: i32,
    pub local: i32, // timezone on the server machine
}

impl DaylightSavingConfig {
    pub fn get_offset_on(&self, date: &Date<Local>) -> i32 {
        let month = date.month();
        let day = date.day();
        let date = (month, day);
        if self.start <= date && date <= self.end {
            self.effective - self.local
        } else {
            self.standard - self.local
        }
    }
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

pub fn read_dumped_event_id(identifier: &str) -> Result<HashSet<String>, Box<dyn Error>> {
    match read_json::<HashSet<String>>(format!("dump/{}.json", identifier).as_str()) {
        Ok(set) => Ok(set),
        Err(e) => Err(Box::new(e)),
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

#[cfg(test)]
mod tests;