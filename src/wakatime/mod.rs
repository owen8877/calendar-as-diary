use std::collections::HashSet;
use std::error::Error;

use chrono::{DateTime, Duration, TimeZone, Utc};
use serde::Deserialize;
use serde_json::Number;

use crate::calendar::event::*;
use crate::calendar::event::Duration::StartEnd;
use crate::common::*;

const IDENTIFIER: &str = "wakatime";

#[derive(Debug, Deserialize)]
struct Item {
    duration: Number,
    project: String,
    time: Number,
}

impl Item {
    fn id(self: &Item) -> String {
        format!("{}|{}", IDENTIFIER, self.time)
    }
}

#[derive(Debug, Deserialize)]
struct Response {
    branches: Vec<String>,
    data: Vec<Item>,
    #[serde(with = "utc_date_format")]
    end: DateTime<Utc>,
    #[serde(with = "utc_date_format")]
    start: DateTime<Utc>,
    timezone: String,
}

pub struct Wakatime {
    request_config: RequestConfig,
    event_ids: HashSet<String>,
}

impl Module for Wakatime {
    fn new(calendar_id: Option<String>) -> Result<Box<dyn Module>, Box<dyn Error>> {
        let request_config = RequestConfig::new(IDENTIFIER, calendar_id)?;
        let event_ids = read_dumped_event_id(IDENTIFIER).unwrap_or(HashSet::new());
        Ok(Box::new(Wakatime {
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
        self.request_config.url.replace("{date}", Utc::now().format("%Y-%m-%d").to_string().as_str())
    }

    fn process_response_into_event_with_id(&self, response: String) -> Result<Vec<EventWithId>, Box<dyn Error>> {
        let items = match serde_json::from_str::<Response>(response.as_str()) {
            Ok(json) => json.data,
            Err(e) => panic!("Cannot parse {} response!, {:#?}. The original response reads:\n{}", IDENTIFIER, e, response),
        };

        Ok(items.iter().map(|item| {
            let created_at = Utc.timestamp(item.time.as_f64().unwrap().floor() as i64, 0);
            EventWithId {
                summary: format!("[Wakatime] {}", item.project),
                description: format!("[link] https://wakatime.com/projects/{}", item.project),
                duration: StartEnd((created_at, created_at + Duration::seconds(item.duration.as_f64().unwrap().floor() as i64))),
                id: item.id(),
            }
        }).collect())
    }
}