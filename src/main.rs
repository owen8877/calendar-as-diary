extern crate google_calendar3 as calendar3;
extern crate hyper;
extern crate hyper_rustls;
extern crate serde_derive;

use std::io;
use std::default::Default;
use std::fs::File;
use std::path::Path;

use calendar3::{Error, Event, EventDateTime};
use calendar3::CalendarHub;
use chrono::{DateTime, Duration, TimeZone, Utc};
use hyper::{Client, net::HttpsConnector};
use hyper_native_tls::NativeTlsClient;
use reqwest::header::{ACCEPT, COOKIE, DNT, HeaderMap, REFERER, USER_AGENT};
use serde::{Deserialize, de};
use serde_json as json;
use serde_json::Number;
use yup_oauth2::{Authenticator, ConsoleApplicationSecret, DefaultAuthenticatorDelegate, DiskTokenStorage, FlowType};

type CalHub = CalendarHub<Client, Authenticator<DefaultAuthenticatorDelegate, DiskTokenStorage, Client>>;

struct SimpleEvent {
    summary: String,
    description: String,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
}

impl From<SimpleEvent> for Event {
    fn from(item: SimpleEvent) -> Self {
        Event {
            summary: Some(item.summary),
            description: Some(item.description),
            start: Some(EventDateTime {
                date_time: Some(item.start.to_rfc3339()),
                ..EventDateTime::default()
            }),
            end: Some(EventDateTime {
                date_time: Some(item.end.to_rfc3339()),
                ..EventDateTime::default()
            }),
            ..Event::default()
        }
    }
}

fn calendar_post(hub: &CalHub, config: &BilibiliConfig, req: Event) {
    let result = hub.events().insert(req, config.calendar_id.as_str()).doit();

    match result {
        Err(e) => match e {
            Error::HttpError(_)
            | Error::MissingAPIKey
            | Error::MissingToken(_)
            | Error::Cancelled
            | Error::UploadSizeLimitExceeded(_, _)
            | Error::Failure(_)
            | Error::BadRequest(_)
            | Error::FieldClash(_)
            | Error::JsonDecodeError(_, _) => println!("{}", e),
        },
        Ok((res, events)) => println!("Success: {:#?}", events),
    }
}

fn init_hub() -> CalHub {
    let json_file_path = Path::new("config/clientsecret.json");
    let json_file = File::open(json_file_path).expect("file not found");
    let secret = json::from_reader::<File, ConsoleApplicationSecret>(json_file)
        .expect("client secret not found").installed.unwrap();
    let token_location: String = String::from("config/tokenstorage.json");
    let token_storage = DiskTokenStorage::new(&token_location).expect("init failed");
    let auth = Authenticator::new(&secret, DefaultAuthenticatorDelegate,
                                  Client::with_connector(HttpsConnector::new(NativeTlsClient::new().unwrap())),
                                  token_storage, Some(FlowType::InstalledInteractive));
    let hub = CalendarHub::new(hyper::Client::with_connector(hyper::net::HttpsConnector::new(hyper_rustls::TlsClient::new())), auth);
    hub
}

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

#[derive(Debug, Deserialize)]
struct BilibiliResponse {
    code: Number,
    data: Vec<BilibiliHistoryItem>,
}

#[derive(Debug, Deserialize)]
struct Headers {
    user_agent: String,
    accept: String,
    dnt: String,
    referer: String,
    cookie: String,
}

#[derive(Debug, Deserialize)]
struct BilibiliConfigJson {
    url: String,
    calendar_id: String,
    headers: Headers,
}

struct BilibiliConfig {
    url: String,
    calendar_id: String,
    headers: HeaderMap,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hub = init_hub();
    let bilibili_config = init_bilibili();
    let data = get_bilibili(&bilibili_config).await?.data;

    for item in data.iter() {
        let view_duration = match item.progress.as_i64().unwrap() {
            -1 => &item.page.duration,
            _ => &item.progress,
        };
        let req: Event = SimpleEvent {
            summary: format!("[Bilibili] {}", item.title),
            description: format!("[link] {}\n[bvid] {}\n[hash] {}", item.redirect_link, item.bvid, 0),
            start: Utc.ymd(1970, 1, 1).and_hms(0, 0, 0) + Duration::seconds(item.view_at.as_i64().unwrap()),
            end: Utc.ymd(1970, 1, 1).and_hms(0, 0, 0) + Duration::seconds(item.view_at.as_i64().unwrap() + view_duration.as_i64().unwrap()),
        }.into();
        calendar_post(&hub, &bilibili_config, req);
    }

    Ok(())
}

fn read_json<T: de::DeserializeOwned>(file_path: &str) -> Result<T, io::Error> {
    match File::open(Path::new(file_path)) {
        Ok(file) => {
            match json::from_reader::<File, T>(file) {
                Ok(result) => Ok(result),
                Err(e) => Err(e.into()),
            }
        },
        Err(e) => Err(e),
    }
}

fn init_bilibili() -> BilibiliConfig {
    let mut config = BilibiliConfig {
        url: String::from(""),
        calendar_id: String::from(""),
        headers: HeaderMap::new(),
    };

    match read_json::<BilibiliConfigJson>("config/bilibili.json.default") {
        Ok(default_config) => {
            config.headers.insert(USER_AGENT, default_config.headers.user_agent.parse().unwrap());
            config.headers.insert(ACCEPT, default_config.headers.cookie.parse().unwrap());
            config.headers.insert(DNT, default_config.headers.dnt.parse().unwrap());
            config.headers.insert(REFERER, default_config.headers.referer.parse().unwrap());
            config.headers.insert(COOKIE, default_config.headers.cookie.parse().unwrap());
            config.url = default_config.url;
            config.calendar_id = default_config.calendar_id;
        },
        Err(e) => panic!("Default bilibili config not found! {}", e),
    }
    match read_json::<BilibiliConfigJson>("config/bilibili.json") {
        Ok(custom_config) => {
            config.headers.insert(USER_AGENT, custom_config.headers.user_agent.parse().unwrap());
            config.headers.insert(ACCEPT, custom_config.headers.cookie.parse().unwrap());
            config.headers.insert(DNT, custom_config.headers.dnt.parse().unwrap());
            config.headers.insert(REFERER, custom_config.headers.referer.parse().unwrap());
            config.headers.insert(COOKIE, custom_config.headers.cookie.parse().unwrap());
            config.url = custom_config.url;
            config.calendar_id = custom_config.calendar_id;
        }
        Err(e) => println!("Bilibili config file not found, falling back to default file. {}", e),
    }

    config
}

async fn get_bilibili(config: &BilibiliConfig) -> Result<BilibiliResponse, Box<dyn std::error::Error>> {
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