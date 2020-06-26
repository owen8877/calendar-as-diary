use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use chrono::{TimeZone, Utc, DateTime, Duration, Local, Datelike};
use scraper::{ElementRef, Html, Selector};
use serde::Deserialize;

use crate::calendar::event::*;
use crate::common::*;
use crate::youtube::ParseError::*;
use regex::{Regex, Captures};

const IDENTIFIER: &str = "youtube";

#[derive(Debug)]
struct YoutubeHistoryItem {
    link: String,
    title: String,
    author: String,
    length: u32,
    start: DateTime<Local>,
}

impl YoutubeHistoryItem {
    fn id(self: &YoutubeHistoryItem) -> String {
        let paths = self.link.split("=").collect::<Vec<&str>>();
        let id = paths[1];
        format!("{}|{}|{}", IDENTIFIER, id, self.start.format("%Y-%m-%d %H:%M").to_string())
    }
}

pub struct Youtube {
    request_config: RequestConfig,
    event_ids: HashSet<String>,
}

#[derive(Debug, Clone)]
enum ParseError {
    UnknownElement(String),
    WrongStartTimeInput(String),
    WrongTotalLengthInput(String),
    WrongPercentStyleInput(String),
    WrongViewDateInput(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UnknownElement(t) => write!(f, "Unknown element type: {}", t),
            WrongStartTimeInput(t) => write!(f, "Wrong start time input: {}", t),
            WrongTotalLengthInput(t) => write!(f, "Wrong total length input: {}", t),
            WrongPercentStyleInput(t) => write!(f, "Wrong percent style input: {}", t),
            WrongViewDateInput(t) => write!(f, "Wrong view date input: {}", t)
        }
    }
}

impl Error for ParseError {

}

impl Module for Youtube {
    fn new(calendar_id: Option<String>) -> Youtube {
        Youtube {
            request_config: RequestConfig::new(IDENTIFIER, calendar_id),
            event_ids: read_dumped_event_id(IDENTIFIER),
        }
    }

    fn dump(&self) {
        dump_event_id_wrapper(IDENTIFIER, &self.event_ids);
    }

    fn get_config(&self) -> &RequestConfig {
        &(self.request_config)
    }

    fn get_event_ids(&mut self) -> &mut HashSet<String> {
        &mut self.event_ids
    }

    fn get_identifier(&self) -> &str {
        IDENTIFIER
    }

    fn get_request_url(&self) -> String {
        self.request_config.url.to_string()
    }

    fn process_response_into_event_with_id(&self, response: String) -> Result<Vec<EventWithId>, Box<dyn Error>> {
        let document = Html::parse_document(response.as_str());
        let selector = Selector::parse("c-wiz[data-token]").unwrap();
        let div_selector = Selector::parse("div").unwrap();
        let a_selector = Selector::parse("a").unwrap();
        let h2_selector = Selector::parse("h2").unwrap();

        let first_cwiz_element = match document.select(&selector).next() {
            None => return Ok(vec![]),
            Some(e) => e,
        };

        let list_div = first_cwiz_element.parent().and_then(ElementRef::wrap).unwrap();

        let mut items = vec![];
        let mut date = Local::today();
        for e in list_div.children() {
            let e = ElementRef::wrap(e).unwrap();
            match e.value().name() {
                "div" => {
                    let date_h2 = e.select(&h2_selector).next().unwrap();
                    let date_text = date_h2.inner_html();
                    if date_text == "今天" {
                        date = Local::today();
                    } else if date_text == "昨天" {
                        date = Local::today().pred();
                    } else {
                        let ymd_pattern = Regex::new(r"(\d+)年(\d+)月(\d+)日")?;
                        if let Some(cap) = ymd_pattern.captures(date_text.as_str()) {
                            let year = cap[1].parse::<i32>().unwrap();
                            let month = cap[2].parse::<u32>().unwrap();
                            let day = cap[3].parse::<u32>().unwrap();
                            date = Local.ymd(year, month, day);
                        } else {
                            let md_pattern = Regex::new(r"(\d+)月(\d+)日")?;
                            match md_pattern.captures(date_text.as_str()) {
                                None => return Err(Box::new(WrongViewDateInput(date_text))),
                                Some(cap) => {
                                    let year = Local::today().year();
                                    let month = cap[1].parse::<u32>().unwrap();
                                    let day = cap[2].parse::<u32>().unwrap();
                                    date = Local.ymd(year, month, day);
                                }
                            }
                        }
                    }
                },
                "c-wiz" => {
                    let card_root = e
                        .select(&div_selector).next().unwrap()
                            .select(&div_selector).next().unwrap()
                                .select(&div_selector).next().unwrap()
                                .next_sibling().and_then(ElementRef::wrap).unwrap();
                    let left_panel = card_root.select(&div_selector).next().unwrap();
                    let title_element = left_panel
                        .select(&div_selector).next().unwrap()
                            .select(&a_selector).next().unwrap();
                    let author_element = left_panel
                        .select(&div_selector).next().unwrap()
                        .next_sibling().and_then(ElementRef::wrap).unwrap()
                            .select(&a_selector).next().unwrap();
                    let start_time_element = left_panel
                        .select(&div_selector).next().unwrap()
                        .next_sibling().and_then(ElementRef::wrap).unwrap()
                        .next_sibling().and_then(ElementRef::wrap).unwrap()
                            .select(&div_selector).next().unwrap();
                    let total_length_element = card_root
                        .select(&div_selector).next().unwrap()
                        .next_sibling().and_then(ElementRef::wrap).unwrap()
                            .select(&a_selector).next().unwrap()
                                .select(&div_selector).next().unwrap()
                                    .select(&div_selector).next().unwrap()
                                    .next_sibling().and_then(ElementRef::wrap).unwrap();
                    let percent_element = {
                        match total_length_element.next_sibling().and_then(ElementRef::wrap) {
                            None => None,
                            Some(e) => e.next_sibling().and_then(ElementRef::wrap),
                        }
                    };

                    let start_hour_minute = {
                        let full_text = start_time_element.inner_html();
                        let am_const = "上午";
                        let pattern = Regex::new(r"(上午|下午)(\d+):(\d+)")?;
                        match pattern.captures(full_text.as_str()) {
                            None => return Err(Box::new(WrongStartTimeInput(full_text))),
                            Some(cap) => {
                                let hour = *&cap[2].parse::<u32>().unwrap();
                                let minute = *&cap[3].parse::<u32>().unwrap();
                                if &cap[1] == am_const {
                                    (hour % 12, minute)
                                } else {
                                    (hour % 12 + 12, minute)
                                }
                            },
                        }
                    };

                    let total_length = {
                        let full_text = total_length_element.inner_html();
                        let one_hour_more_pattern = Regex::new(r"(\d+):(\d+):(\d+)")?;
                        match one_hour_more_pattern.captures(full_text.as_str()) {
                            None => {
                                let pattern = Regex::new(r"(\d+):(\d+)")?;
                                match pattern.captures(full_text.as_str()) {
                                    None => return Err(Box::new(WrongTotalLengthInput(full_text))),
                                    Some(cap) =>
                                        *&cap[1].parse::<u32>().unwrap() * 60 + *&cap[2].parse::<u32>().unwrap()
                                }
                            },
                            Some(cap) =>
                                *&cap[1].parse::<u32>().unwrap() * 3600
                                    + *&cap[2].parse::<u32>().unwrap() * 60
                                    + *&cap[3].parse::<u32>().unwrap()
                        }
                    };

                    let watched_length = match percent_element {
                        None => total_length,
                        Some(percent_element) => {
                            let style_text = percent_element.value().attr("style");
                            match style_text {
                                None => return Err(Box::new(WrongPercentStyleInput(percent_element.html()))),
                                Some(t) => {
                                    let pattern = Regex::new(r"width:(\d+)%")?;
                                    let percent = match pattern.captures(t) {
                                        None => return Err(Box::new(WrongPercentStyleInput(percent_element.html()))),
                                        Some(cap) => *&cap[1].parse::<u32>().unwrap()
                                    };
                                    total_length * percent / 100
                                }
                            }
                        },
                    };

                    items.push(YoutubeHistoryItem {
                        link: title_element.value().attr("href").unwrap().to_string(),
                        title: title_element.inner_html(),
                        author: author_element.inner_html(),
                        length: watched_length,
                        start: date.and_hms(start_hour_minute.0, start_hour_minute.1, 0)
                    })
                },
                t => return Err(Box::new(UnknownElement(t.to_string()))),
            }
        }

        Ok(items.iter().map(|item| {
            EventWithId::new(PartialDayEvent {
                summary: format!("[Youtube] {}", item.title),
                description: format!("[link] {}\n[author] {}\n[hash] {}", item.link, item.author, item.id()),
                start: item.start.with_timezone(&Utc),
                end: (item.start + Duration::seconds(item.length as i64)).with_timezone(&Utc),
            }.into(), item.id())
        }).collect())
    }
}