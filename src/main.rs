extern crate google_calendar3 as calendar3;
extern crate hyper;
extern crate hyper_rustls;
extern crate reqwest;
extern crate serde_derive;

use reqwest::Response;

use crate::bilibili::*;
use crate::calendar::*;
use crate::common::*;
use crate::netflix::*;

mod bilibili;
mod common;
mod calendar;
mod netflix;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hub = init_hub();

    let modules: Vec<Box<dyn Module>> = vec![Box::new(Bilibili::new()), Box::new(Netflix::new())];

    for module in modules {
        let config = module.get_config();
        let client = reqwest::Client::new();
        let headers = config.headers.clone();
        let response: Response = client.get(&config.url)
            .headers(headers)
            .send()
            .await?;
        let events = module.process_response_into_event_with_id(response).await?;
        for event in events.iter().take(5) {
            calendar_post(&hub, &config, event.event.clone());
        }
    }

    Ok(())
}
