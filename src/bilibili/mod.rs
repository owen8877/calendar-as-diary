use chrono::{Duration, TimeZone, Utc};
use serde::Deserialize;
use serde_json::Number;

use crate::common::*;
use crate::calendar::event::*;
use std::collections::HashSet;
use std::collections::hash_map::RandomState;
use std::io::Error;

const IDENTIFIER: &str = "bilibili";

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
    event_ids: HashSet<String>,
}

impl Module for Bilibili {
    fn new(calendar_id: Option<String>) -> Bilibili {
        Bilibili {
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

    fn process_response_into_event_with_id(&self, response: String) -> Vec<EventWithId> {
        let items = match serde_json::from_str::<BilibiliResponse>(response.as_str()) {
            Ok(json) => json.data,
            Err(e) => panic!("Cannot parse bilibili response!, {:#?}", e),
        };

        items.iter().map(|item| {
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
        }).collect()
    }
}