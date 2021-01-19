use std::collections::HashSet;
use std::error::Error;

use chrono::{TimeZone, Utc};
use scraper::{ElementRef, Html, Selector};
use serde::Deserialize;

use crate::calendar::event::*;
use crate::calendar::event::Duration::WholeDay;
use crate::common::*;

const IDENTIFIER: &str = "netflix";

#[derive(Debug, Deserialize)]
struct Item {
    link: String,
    title: String,
    date: String,
}

impl Item {
    fn id(self: &Item) -> String {
        let paths = self.link.split("/").collect::<Vec<&str>>();
        let id = paths[2].parse::<u32>().unwrap();
        format!("{}|{}|{}", IDENTIFIER, id, self.date)
    }
}

pub struct Netflix {
    request_config: RequestConfig,
    event_ids: HashSet<String>,
}

impl Module for Netflix {
    fn new(calendar_id: Option<String>) -> Result<Box<dyn Module>, Box<dyn Error>> {
        let request_config = RequestConfig::new(IDENTIFIER, calendar_id)?;
        let event_ids = read_dumped_event_id(IDENTIFIER).unwrap_or(HashSet::new());
        Ok(Box::new(Netflix {
            request_config,
            event_ids,
        }))
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

    fn need_for_detail(&self, _response: &String) -> Option<Vec<String>> {
        None
    }

    fn process_response_into_event_with_id(&self, responses: Vec<String>) -> Result<Vec<EventWithId>, Box<dyn Error>> {
        let response = responses[0].clone();
        let document = Html::parse_document(response.as_str());
        let selector = Selector::parse("li.retableRow").unwrap();
        let title_selector = Selector::parse("div.title").unwrap();
        let date_selector = Selector::parse("div.date").unwrap();
        let link_selector = Selector::parse("a").unwrap();

        let items: Vec<Item> = document.select(&selector).map(|e: ElementRef| {
            let link_element = e.select(&title_selector).next().unwrap().select(&link_selector).next().unwrap();
            Item {
                link: String::from(link_element.value().attr("href").unwrap()),
                title: link_element.inner_html(),
                date: e.select(&date_selector).next().unwrap().inner_html(),
            }
        }).collect();

        Ok(items.iter().map(|item| {
            let date_info: Vec<u32> = item.date.split("/").collect::<Vec<&str>>().iter().map(|s: &&str| s.parse::<u32>().unwrap()).collect();
            EventWithId {
                summary: format!("[Netflix] {}", item.title),
                description: format!("[link] https://www.netflix.com{}\n[hash] {}", item.link, item.id()),
                duration: WholeDay(Utc.ymd((2000 + date_info[2]) as i32, date_info[0], date_info[1])),
                id: item.id(),
            }
        }).collect())
    }
}