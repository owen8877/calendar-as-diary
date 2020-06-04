use calendar3::Event;
use chrono::{TimeZone, Utc};
use reqwest::header::HeaderMap;
use scraper::{ElementRef, Html, Selector};
use serde::Deserialize;

use crate::calendar::*;
use crate::common::*;

#[derive(Debug, Deserialize)]
struct NetflixHistoryItem {
    link: String,
    title: String,
    date: String,
}

impl HistoryItem for NetflixHistoryItem {
    fn hash(self: &NetflixHistoryItem) -> String {
        let paths = self.link.split("/").collect::<Vec<&str>>();
        let id = paths[2].parse::<u32>().unwrap();
        format!("netflix|{}|{}", id, self.date)
    }
}

#[derive(Debug, Deserialize)]
struct NetflixResponse {
    data: Vec<NetflixHistoryItem>,
}

pub fn init_netflix() -> RequestConfig {
    let mut config = RequestConfig {
        url: String::from(""),
        calendar_id: String::from(""),
        headers: HeaderMap::new(),
    };

    match read_json::<RequestConfigJson>("config/netflix.json.default") {
        Ok(default_config) => {
            headers_modifier(&default_config.headers, &mut config.headers);
            config.url = default_config.url;
            config.calendar_id = default_config.calendar_id;
        },
        Err(e) => panic!("Default netflix config not found! {}", e),
    }
    match read_json::<RequestConfigJson>("config/netflix.json") {
        Ok(custom_config) => {
            headers_modifier(&custom_config.headers, &mut config.headers);
            config.url = custom_config.url;
            config.calendar_id = custom_config.calendar_id;
        }
        Err(e) => println!("Netflix config file not found, falling back to default file. {}", e),
    }

    config
}

async fn get_netflix(config: &RequestConfig) -> Result<NetflixResponse, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let headers = config.headers.clone();
    let resp: String = client.get(&config.url)
        .headers(headers)
        .send()
        .await?
        .text()
        .await?;

    let document = Html::parse_document(resp.as_str());
    let selector = Selector::parse("li.retableRow").unwrap();
    let title_selector = Selector::parse("div.title").unwrap();
    let date_selector = Selector::parse("div.date").unwrap();
    let link_selector = Selector::parse("a").unwrap();

    Ok(NetflixResponse {
        data: document.select(&selector).map(|e: ElementRef| {
            let link_element = e.select(&title_selector).next().unwrap().select(&link_selector).next().unwrap();
            NetflixHistoryItem {
                link: String::from(link_element.value().attr("href").unwrap()),
                title: link_element.inner_html(),
                date: e.select(&date_selector).next().unwrap().inner_html(),
            }
        }).collect()
    })
}

pub async fn post_netflix_to_calendar(hub: &CalHub, config: &RequestConfig) -> Result<(), Box<dyn std::error::Error>> {
    let data = get_netflix(&config).await?.data;
    for item in data.iter().take(5) {
        let date_info: Vec<u32> = item.date.split("/").collect::<Vec<&str>>().iter().map(|s: &&str| s.parse::<u32>().unwrap()).collect();
        let req: Event = SimpleWholeDayEvent {
            summary: format!("[Netflix] {}", item.title),
            description: format!("[link] https://www.netflix.com{}\n[hash] {}", item.link, item.hash()),
            date: Utc.ymd((2000 + date_info[2]) as i32, date_info[0], date_info[1]),
        }.into();
        calendar_post(&hub, &config, req);
    }

    Ok(())
}