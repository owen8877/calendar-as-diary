use std::error::Error;

use chrono::{Duration, TimeZone, Utc};
use reqwest::Response;
use serde::Deserialize;
use serde_json::Number;

use async_trait::async_trait;

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

impl BilibiliHistoryItem {
    fn id(self: &BilibiliHistoryItem) -> String {
        format!("bilibili|{}|{}|{}", self.bvid, self.page.page, self.view_at)
    }
}

#[derive(Debug, Deserialize)]
struct BilibiliResponse {
    code: Number,
    data: Vec<BilibiliHistoryItem>,
}

pub struct Bilibili {
    request_config: RequestConfig,
}

#[async_trait]
impl Module for Bilibili {
    fn new() -> Bilibili {
        Bilibili {
            request_config: RequestConfig::new("bilibili"),
        }
    }

    fn get_config(&self) -> &RequestConfig {
        &(self.request_config)
    }

    async fn process_response_into_event_with_id(&self, response: Response) -> Result<Vec<EventWithId>, Box<dyn Error>> {
        let items: Vec<BilibiliHistoryItem> = response.json::<BilibiliResponse>().await?.data;

        Ok(items.iter().map(|item| {
            let view_duration = match item.progress.as_i64().unwrap() {
                -1 => &item.page.duration,
                _ => &item.progress,
            };
            EventWithId::new(PartialDayEvent {
                summary: format!("[Bilibili] {}", item.title),
                description: format!("[link] {}\n[bvid] {}\n[hash] {}", item.redirect_link, item.bvid, item.id()),
                start: Utc.ymd(1970, 1, 1).and_hms(0, 0, 0) + Duration::seconds(item.view_at.as_i64().unwrap()),
                end: Utc.ymd(1970, 1, 1).and_hms(0, 0, 0) + Duration::seconds(item.view_at.as_i64().unwrap() + view_duration.as_i64().unwrap()),
            }.into(), item.id())
        }).collect::<Vec<EventWithId>>())
    }
}