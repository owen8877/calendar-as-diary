extern crate google_calendar3 as calendar3;
extern crate hyper;
extern crate hyper_rustls;
extern crate serde_derive;

use crate::bilibili::*;
use crate::calendar::*;
use crate::netflix::*;

mod bilibili;
mod common;
mod calendar;
mod netflix;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hub = init_hub();
    let bilibili_config = init_bilibili();
    let netflix_config = init_netflix();

    post_bilibili_to_calendar(&hub, &bilibili_config).await;
    post_netflix_to_calendar(&hub, &netflix_config).await;

    Ok(())
}
