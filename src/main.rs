extern crate google_calendar3 as calendar3;
extern crate hyper;
extern crate hyper_rustls;
extern crate serde_derive;

use std::collections::HashMap;
use std::default::Default;
use std::fs::File;
use std::io;
use std::path::Path;

use calendar3::{Error, Event, EventDateTime};
use calendar3::CalendarHub;
use chrono::{Date, DateTime, Duration, TimeZone, Utc};
use hyper::{Client, net::HttpsConnector};
use hyper_native_tls::NativeTlsClient;
use reqwest::header::{ACCEPT, ACCEPT_LANGUAGE, CACHE_CONTROL, COOKIE, DNT, HeaderMap, HeaderName, REFERER, UPGRADE_INSECURE_REQUESTS, USER_AGENT};
use scraper::{ElementRef, Html, Selector};
use serde::{de, Deserialize};
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

struct SimpleWholeDayEvent {
    summary: String,
    description: String,
    date: Date<Utc>,
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

impl From<SimpleWholeDayEvent> for Event {
    fn from(item: SimpleWholeDayEvent) -> Self {
        Event {
            summary: Some(item.summary),
            description: Some(item.description),
            start: Some(EventDateTime {
                date: Some(item.date.format("%Y-%m-%d").to_string()),
                ..EventDateTime::default()
            }),
            end: Some(EventDateTime {
                date: Some(item.date.format("%Y-%m-%d").to_string()),
                ..EventDateTime::default()
            }),
            ..Event::default()
        }
    }
}

fn calendar_post(hub: &CalHub, config: &RequestConfig, req: Event) {
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

trait HistoryItem {
    fn hash(&self) -> String;
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

#[derive(Debug, Deserialize)]
struct NetflixHistoryItem {
    link: String,
    title: String,
    date: String,
}

impl HistoryItem for NetflixHistoryItem {
    fn hash(self: &NetflixHistoryItem) -> String {
        let paths = self.link.split("/").collect::<Vec<&str>>();
        let id = paths[2].parse::<u32>().unwrap();
        format!("netflix|{}|{}", id, self.date)
    }
}

#[derive(Debug, Deserialize)]
struct NetflixResponse {
    data: Vec<NetflixHistoryItem>,
}

#[derive(Debug, Deserialize)]
struct RequestConfigJson {
    url: String,
    calendar_id: String,
    headers: HashMap<String, String>,
}

struct RequestConfig {
    url: String,
    calendar_id: String,
    headers: HeaderMap,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let hub = init_hub();
    let bilibili_config = init_bilibili();
    let netflix_config = init_netflix();

    post_bilibili_to_calendar(&hub, &bilibili_config).await;
    post_netflix_to_calendar(&hub, &netflix_config).await;

    Ok(())
}

async fn post_bilibili_to_calendar(hub: &CalHub, config: &RequestConfig) -> Result<(), Box<dyn std::error::Error>> {
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

async fn post_netflix_to_calendar(hub: &CalHub, config: &RequestConfig) -> Result<(), Box<dyn std::error::Error>> {
    let data = get_netflix(&config).await?.data;
    for item in data.iter().take(5) {
        let date_info: Vec<u32> = item.date.split("/").collect::<Vec<&str>>().iter().map(|s: &&str| s.parse::<u32>().unwrap()).collect();
        let req: Event = SimpleWholeDayEvent {
            summary: format!("[Netflix] {}", item.title),
            description: format!("[link] https://www.netflix.com{}\n[hash] {}", item.link, item.hash()),
            date: Utc.ymd((2000 + date_info[2]) as i32, date_info[0], date_info[1]),
        }.into();
        calendar_post(&hub, &config, req);
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

fn init_bilibili() -> RequestConfig {
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

fn init_netflix() -> RequestConfig {
    let mut config = RequestConfig {
        url: String::from(""),
        calendar_id: String::from(""),
        headers: HeaderMap::new(),
    };

    match read_json::<RequestConfigJson>("config/netflix.json.default") {
        Ok(default_config) => {
            headers_modifier(&default_config.headers, &mut config.headers);
            config.url = default_config.url;
            config.calendar_id = default_config.calendar_id;
        },
        Err(e) => panic!("Default netflix config not found! {}", e),
    }
    match read_json::<RequestConfigJson>("config/netflix.json") {
        Ok(custom_config) => {
            headers_modifier(&custom_config.headers, &mut config.headers);
            config.url = custom_config.url;
            config.calendar_id = custom_config.calendar_id;
        }
        Err(e) => println!("Netflix config file not found, falling back to default file. {}", e),
    }

    config
}

async fn get_netflix(config: &RequestConfig) -> Result<NetflixResponse, Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();
    let headers = config.headers.clone();
    let resp: String = client.get(&config.url)
        .headers(headers)
        .send()
        .await?
        .text()
        .await?;

    let document = Html::parse_document(resp.as_str());
    let selector = Selector::parse("li.retableRow").unwrap();
    let title_selector = Selector::parse("div.title").unwrap();
    let date_selector = Selector::parse("div.date").unwrap();
    let link_selector = Selector::parse("a").unwrap();

    Ok(NetflixResponse {
        data: document.select(&selector).map(|e: ElementRef| {
            let link_element = e.select(&title_selector).next().unwrap().select(&link_selector).next().unwrap();
            NetflixHistoryItem {
                link: String::from(link_element.value().attr("href").unwrap()),
                title: link_element.inner_html(),
                date: e.select(&date_selector).next().unwrap().inner_html(),
            }
        }).collect()
    })
}

fn headers_modifier(headers: &HashMap<String, String>, header_map: &mut HeaderMap) {
    let header_dict: HashMap<&str, HeaderName> = get_header_dict();

    for key in headers.keys() {
        match header_dict.get(key.as_str()) {
            Some(header) => {
                header_map.insert(header, headers[key].parse().unwrap());
            },
            None => panic!("Unknown header {}.", key),
        }
    }
}

fn get_header_dict() -> HashMap<&'static str, HeaderName> {
    let mut dict = HashMap::<&str, HeaderName>::new();
    dict.insert("accept",                    ACCEPT);
    dict.insert("accept-language",           ACCEPT_LANGUAGE);
    dict.insert("cache-control",             CACHE_CONTROL);
    dict.insert("cookie",                    COOKIE);
    dict.insert("dnt",                       DNT);
    dict.insert("referer",                   REFERER);
    dict.insert("upgrade-insecure-requests", UPGRADE_INSECURE_REQUESTS);
    dict.insert("user-agent",                USER_AGENT);

    dict
}