extern crate google_calendar3 as calendar3;
extern crate hyper;
extern crate hyper_rustls;
#[macro_use] extern crate log;
extern crate regex;
extern crate reqwest;
extern crate serde_derive;
extern crate tokio;

use std::error::Error;
use std::time::{Duration, SystemTime};

use tokio::time;

use crate::bilibili::*;
use crate::calendar::*;
use crate::calendar::event::EventWithId;
use crate::common::*;
use crate::netflix::*;
use crate::wakatime::*;
use crate::youtube::*;

mod bilibili;
mod common;
mod calendar;
mod netflix;
mod youtube;
mod wakatime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let hub = init_hub();
    let mut modules: Vec<Box<dyn Module>> = vec![
        Box::new(Bilibili::new(None)),
        Box::new(Netflix::new(None)),
        Box::new(Wakatime::new(None)),
        Box::new(Youtube::new(None)),
    ];
    let mut interval = time::interval(Duration::from_millis(60*60*1000));

    loop {
        interval.tick().await;
        info!("Timer picked up at {:#?}", SystemTime::now());
        for mut module in &mut modules {
            let response = fetch_data(&mut module).await?;
            let events = filter_events_to_be_posted(&mut module, response);
            match events {
                Ok(events) => {
                    for event in events {
                        calendar_post(&hub, module.get_config(), event.event.clone());
                    }
                    module.dump()
                },
                Err(e) => error!("{}", e),
            }

        }
        info!("Waiting for timer to pick up...")
    }

    Ok(())
}

async fn fetch_data(module: &mut Box<dyn Module>) -> Result<String, Box<dyn std::error::Error>> {
    let response = reqwest::Client::new()
        .get(&module.get_request_url())
        .headers(module.get_config().headers.clone())
        .send()
        .await?
        .text()
        .await?;

    Ok(response)
}

fn filter_events_to_be_posted(module: &mut Box<dyn Module>, response: String) -> Result<Vec<EventWithId>, Box<dyn Error>> {
    let fetched_events = module.process_response_into_event_with_id(response)?;
    Ok(fetched_events.into_iter().filter(|event| {
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

#[cfg(test)]
mod tests;
