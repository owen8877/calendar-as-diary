extern crate core;
extern crate google_calendar3 as calendar3;
extern crate hyper;
extern crate hyper_rustls;
#[macro_use]
extern crate log;
extern crate regex;
extern crate reqwest;
extern crate serde_derive;
extern crate tokio;

use std::error::Error;
use std::time::SystemTime;

use chrono::{Duration, Utc};
use tokio::time;

use crate::bilibili::*;
use crate::calendar::*;
use crate::calendar::event::Duration::{StartEnd, WholeDay};
use crate::calendar::event::EventWithId;
use crate::common::*;
use crate::league_of_graphs::*;
use crate::league_of_legends::*;
use crate::netflix::*;
use crate::ut_oden_seminar::*;
use crate::wakatime::*;
use crate::youtube::*;

mod bilibili;
mod common;
mod calendar;
mod league_of_legends;
mod league_of_graphs;
mod netflix;
mod ut_oden_seminar;
mod youtube;
mod wakatime;

#[tokio::main]
#[allow(dead_code)]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let mut hub = init_hub().await;
    let mut modules: Vec<Box<dyn Module>> = filter_loaded_modules(vec![
        // Bilibili::new(None),
        LeagueOfGraphs::new(None),
        // LeagueOfLegends::new(None),
        // Netflix::new(None),
        // Wakatime::new(None),
        UTOdenSeminar::new(None),
        // Youtube::new(None),
    ]);
    let mut interval = time::interval(std::time::Duration::from_millis(60 * 60 * 1000));

    loop {
        interval.tick().await;
        info!("Timer picked up at {:#?}", SystemTime::now());
        for mut module in &mut modules {
            let response = fetch_data(&mut module).await?;
            let detail_response = make_detail(&mut module, response).await?;
            let events = filter_events_to_be_posted(&mut module, detail_response);
            match events {
                Ok(events) => {
                    for event in events {
                        calendar_post(&mut hub, module.get_config(), event.into()).await;
                    }
                    module.dump()
                }
                Err(e) => error!("{}", e),
            }
        }
        info!("Waiting for timer to pick up...")
    }

    Ok(())
}

async fn fetch_data(module: &mut Box<dyn Module>) -> Result<String, Box<dyn Error>> {
    let response = reqwest::Client::new()
        .get(&module.get_request_url())
        .headers(module.get_config().headers.clone())
        .send()
        .await?
        .text()
        .await?;

    Ok(response)
}

async fn make_detail(module: &mut Box<dyn Module>, response: String) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    if let Some(further_request_urls) = module.need_for_detail(&response) {
        let mut responses = vec![];
        for url in further_request_urls {
            let response = reqwest::Client::new()
                .get(&url)
                .headers(module.get_config().headers.clone())
                .send()
                .await?
                .text()
                .await?;
            responses.push(response);
        }
        Ok(responses)
    } else {
        Ok(vec![response])
    }
}

fn filter_events_to_be_posted(module: &mut Box<dyn Module>, responses: Vec<String>) -> Result<Vec<EventWithId>, Box<dyn Error>> {
    let fetched_events = module.process_response_into_event_with_id(responses)?;
    let filtered_events = filter_event(fetched_events);
    Ok(filtered_events.into_iter().filter(|event| {
        if module.get_event_ids().contains(event.id.as_str()) {
            debug!("Event with id \"{}\" already exists; skipped.", event.id);
            false
        } else {
            module.get_event_ids().insert(event.id.clone());
            debug!("Event with id \"{}\" shows for the first time; inserting.", event.id);
            true
        }
    }).collect())
}

fn filter_event(events: Vec<EventWithId>) -> Vec<EventWithId> {
    events.into_iter()
        .filter(|event| {
            if event.id.contains("ut_oden_seminar") {
                return true;
            }
            if match &event.duration {
                StartEnd(_, end) => *end < Utc::now() - Duration::hours(1),
                WholeDay(w) => *w <= Utc::today() - Duration::days(1),
            } {
                true
            } else {
                info!("Event {} is filtered since it seems to be ongoing.", event.summary);
                false
            }
        })
        .filter(|event| {
            if event.id.contains("bilibili") {
                return true;
            }
            if match &event.duration {
                StartEnd(start, end) => *end - *start > Duration::minutes(5),
                WholeDay(_) => true,
            } {
                true
            } else {
                info!("Event {} doesn't last long enough so it is ignored.", event.summary);
                false
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
