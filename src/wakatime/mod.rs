use std::collections::HashSet;
use std::error::Error;

use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use serde_json::Number;

use crate::calendar::event::*;
use crate::common::*;

const IDENTIFIER: &str = "wakatime";

#[derive(Debug, Deserialize)]
struct WakatimeItem {
    #[serde(with = "utc_date_format")]
    created_at: DateTime<Utc>,
    duration: Number,
    id: String,
    machine_name_id: String,
    project: String,
    time: Number,
    user_id: String,
}

impl WakatimeItem {
    fn id(self: &WakatimeItem) -> String {
        format!("{}|{}", IDENTIFIER, self.id)
    }
}

#[derive(Debug, Deserialize)]
struct WakatimeResponse {
    branches: Vec<String>,
    data: Vec<WakatimeItem>,
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
    fn new(calendar_id: Option<String>) -> Wakatime {
        Wakatime {
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
        let yesterday = Utc::now() - Duration::days(1);
        self.request_config.url.replace("{date}", yesterday.format("%Y-%m-%d").to_string().as_str())
    }

    fn process_response_into_event_with_id(&self, response: String) -> Result<Vec<EventWithId>, Box<dyn Error>> {
        let items = match serde_json::from_str::<WakatimeResponse>(response.as_str()) {
            Ok(json) => json.data,
            Err(e) => panic!("Cannot parse {} response!, {:#?}", IDENTIFIER, e),
        };

        Ok(items.iter().map(|item| EventWithId::new(PartialDayEvent {
            summary: format!("[Wakatime] {}", item.project),
            description: format!("[link] https://wakatime.com/projects/{}", item.project),
            start: item.created_at,
            end: item.created_at + Duration::seconds(item.duration.as_f64().unwrap().floor() as i64),
        }.into(), item.id())).collect())
    }
}