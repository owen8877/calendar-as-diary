use calendar3::Event;
use chrono::{Duration, TimeZone, Utc};
use reqwest::header::HeaderMap;
use serde::Deserialize;
use serde_json::Number;

use crate::calendar::*;
use crate::common::*;

#[derive(Debug, Deserialize)]
struct BilibiliPage2 {
    cid: Number,
    page: Number,
    part: String,
    duration: Number,
}

#[derive(Debug, Deserialize)]
struct BilibiliHistoryItem {
    aid: Number,
    bvid: String,
    page: BilibiliPage2,
    progress: Number,
    redirect_link: String,
    title: String,
    view_at: Number,
}

impl HistoryItem for BilibiliHistoryItem {
    fn hash(self: &BilibiliHistoryItem) -> String {
        format!("bilibili|{}|{}|{}", self.bvid, self.page.page, self.view_at)
    }
}

#[derive(Debug, Deserialize)]
struct BilibiliResponse {
    code: Number,
    data: Vec<BilibiliHistoryItem>,
}

pub fn init_bilibili() -> RequestConfig {
    let mut config = RequestConfig {
        url: String::from(""),
        calendar_id: String::from(""),
        headers: HeaderMap::new(),
    };

    match read_json::<RequestConfigJson>("config/bilibili.json.default") {
        Ok(default_config) => {
            headers_modifier(&default_config.headers, &mut config.headers);
            config.url = default_config.url;
            config.calendar_id = default_config.calendar_id;
        },
        Err(e) => panic!("Default bilibili config not found! {}", e),
    }
    match read_json::<RequestConfigJson>("config/bilibili.json") {
        Ok(custom_config) => {
            headers_modifier(&custom_config.headers, &mut config.headers);
            config.url = custom_config.url;
            config.calendar_id = custom_config.calendar_id;
        }
        Err(e) => println!("Bilibili config file not found, falling back to default file. {}", e),
    }

    config
}

async fn get_bilibili(config: &RequestConfig) -> Result<BilibiliResponse, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let headers = config.headers.clone();
    let resp: BilibiliResponse = client.get(&config.url)
        .headers(headers)
        .send()
        .await?
        .json::<BilibiliResponse>()
        .await?;
    Ok(resp)
}

pub async fn post_bilibili_to_calendar(hub: &CalHub, config: &RequestConfig) -> Result<(), Box<dyn std::error::Error>> {
    let data = get_bilibili(&config).await?.data;
    for item in data.iter() {
        let view_duration = match item.progress.as_i64().unwrap() {
            -1 => &item.page.duration,
            _ => &item.progress,
        };
        let req: Event = SimpleEvent {
            summary: format!("[Bilibili] {}", item.title),
            description: format!("[link] {}\n[bvid] {}\n[hash] {}", item.redirect_link, item.bvid, item.hash()),
            start: Utc.ymd(1970, 1, 1).and_hms(0, 0, 0) + Duration::seconds(item.view_at.as_i64().unwrap()),
            end: Utc.ymd(1970, 1, 1).and_hms(0, 0, 0) + Duration::seconds(item.view_at.as_i64().unwrap() + view_duration.as_i64().unwrap()),
        }.into();
        calendar_post(&hub, &config, req);
    }

    Ok(())
}