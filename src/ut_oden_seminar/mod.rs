use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use chrono::{DateTime, FixedOffset, Local, TimeZone};
use regex::Regex;
use scraper::{Html, Selector};

use crate::calendar::event::*;
use crate::calendar::event::Duration::StartEnd;
use crate::common::*;
use crate::ut_oden_seminar::ParseError::*;

const IDENTIFIER: &str = "ut_oden_seminar";

#[derive(Debug)]
struct Item {
    link: String,
    title: String,
    description: String,
    seminar_id: u32,
    start: DateTime<FixedOffset>,
    end: DateTime<FixedOffset>,
}

impl Item {
    fn id(self: &Item) -> String {
        let id = self.seminar_id;
        format!("{}|{}|{}", IDENTIFIER, id, self.start.format("%Y-%m-%d %H:%M").to_string())
    }
}

#[derive(Debug, Clone)]
enum ParseError {
    UnwrapNone(String),
    UnknownMonth(String),
    ParseFails(String),
    CaptureFails(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UnwrapNone(t) => write!(f, "Try to unwrap None at: {}", t),
            UnknownMonth(t) => write!(f, "Unknown month: {}", t),
            ParseFails(t) => write!(f, "Parse fails: {}", t),
            CaptureFails(t) => write!(f, "Capture fails: {}", t),
        }
    }
}

impl Error for ParseError {}

pub struct UTOdenSeminar {
    request_config: RequestConfig,
    event_ids: HashSet<String>,
}

fn parse_time(time_str: &str) -> Result<(i32, i32), Box<dyn Error>> {
    let hour_minute_str = time_str.trim_end_matches("AM").trim_end_matches("PM");
    let (mut hour, minute) = if hour_minute_str.contains(':') {
        let split: Vec<&str> = hour_minute_str.split(':').collect();
        (split[0].parse::<i32>()?,
         split[1].parse::<i32>()?)
    } else {
        (hour_minute_str.parse::<i32>()?, 0)
    };
    if time_str.contains("PM") {
        hour += 12;
    }
    Ok((hour, minute))
}

fn parse_seminar(response: &str) -> Result<Item, Box<dyn Error>> {
    let document = Html::parse_document(response);
    let div_selector = Selector::parse("div#page-body").map_err(|_e| ParseFails("selector div#page-body".to_string()))?;
    let p_selector = Selector::parse("p").map_err(|_e| ParseFails("selector p".to_string()))?;
    let page_div = document
        .select(&div_selector).next()
        .ok_or(UnwrapNone("div#page-body".to_string()))?;
    let info_paragraph = page_div
        .select(&p_selector).next()
        .ok_or(UnwrapNone("first (info) paragraph".to_string()))?;
    let desc_paragraph = page_div
        .select(&p_selector).skip(1).next()
        .ok_or(UnwrapNone("second (description) paragraph".to_string()))?;
    let info_str = info_paragraph.inner_html().chars().filter(|c| c != &'\t').filter(|c| c != &'\n').collect::<String>();

    let info_split: Vec<&str> = info_str.split("<br>").collect();
    let title = info_split[0].to_string();
    let date_str = info_split[1].to_string();
    let time_str = info_split[2].to_string();

    let date_captures = Regex::new(r"(\w+), (\w+) (\d+), (\d+)")?
        .captures(date_str.as_str()).ok_or(CaptureFails("date capture".to_string()))?;
    let day = date_captures.get(3).ok_or(CaptureFails("day".to_string()))?
        .as_str().parse::<i32>()?;
    let month_str = date_captures.get(2).ok_or(CaptureFails("month".to_string()))?
        .as_str().to_string();
    let month_strs = vec!["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
    let (month, _) = month_strs.iter().enumerate().find(|(_i, m)| *m == &month_str.as_str()).ok_or(UnknownMonth(month_str))?;
    let year = date_captures.get(4).ok_or(CaptureFails("year".to_string()))?
        .as_str().parse::<i32>().unwrap();

    let time_split: Vec<&str> = time_str.split(" – ").collect();
    let (start_hour, start_minute) = parse_time(time_split[0])?;
    let (end_hour, end_minute) = parse_time(time_split[1])?;

    let description = desc_paragraph.inner_html();

    let seminar_id = Regex::new(r"Oden Institute Event:(\d+)").unwrap()
        .captures(page_div.inner_html().as_str()).ok_or(CaptureFails("seminar id".to_string()))?
        .get(1).ok_or(UnwrapNone("seminar id capture".to_string()))?
        .as_str().parse::<u32>()?;
    let link = Regex::new(r"https://utexas.zoom.us/j/\d+").unwrap()
        .find(page_div.inner_html().as_str()).ok_or(UnwrapNone("zoom link match".to_string()))?
        .as_str().to_string();

    let time_zone: FixedOffset = FixedOffset::west(5 * 60 * 60); // Daylight saving mode
    Ok(Item {
        link,
        title,
        description,
        seminar_id,
        start: time_zone.ymd(year, month as u32 + 1, day as u32).and_hms(start_hour as u32, start_minute as u32, 0),
        end: time_zone.ymd(year, month as u32 + 1, day as u32).and_hms(end_hour as u32, end_minute as u32, 0),
    })
}

impl Module for UTOdenSeminar {
    fn new(calendar_id: Option<String>) -> Result<Box<dyn Module>, Box<dyn Error>> {
        let request_config = RequestConfig::new(IDENTIFIER, calendar_id)?;
        let event_ids = read_dumped_event_id(IDENTIFIER).unwrap_or(HashSet::new());
        Ok(Box::new(Self {
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
        self.request_config.url.to_string()
    }

    fn need_for_detail(&self, response: &String) -> Option<Vec<String>> {
        let regex = Regex::new(r"/about/events/\d*").unwrap();
        let base_url = Regex::new(r"https://[^/]*").unwrap().find(self.request_config.url.as_str())?.as_str();
        Some(regex.find_iter(response).map(|mat| base_url.to_string() + mat.as_str()).collect())
    }

    fn process_response_into_event_with_id(&self, responses: Vec<String>) -> Result<Vec<EventWithId>, Box<dyn Error>> {
        Ok(responses.into_iter()
            .map(|r| parse_seminar(r.as_str()))
            .filter_map(|r| match r {
                Ok(t) => Some(t),
                Err(e) => {
                    info!("{}", e);
                    None
                }
            })
            .map(|r| {
                EventWithId {
                    id: r.id(),
                    summary: r.title,
                    description: format!("Zoom link: {}\n{}", r.link, r.description),
                    duration: StartEnd((DateTime::from(r.start), DateTime::from(r.end))),
                }
            })
            .collect())
    }
}

#[test]
fn test_regex() {
    let reponse = "\
    <h4 class=\"oden--event-card-title\"><a href=\"/about/events/1539\">Physics Discovery</a></h4>
    <p class=\"oden--event-card-location\">
    <h4 class=\"oden--event-card-title\"><a href=\"/about/events/1551\">Statistical Estimation</a></h4>
    ";
    let regex = Regex::new(r"/about/events/\d*").unwrap();
    for mat in regex.find_iter(reponse) {
        println!("Found: {}", mat.as_str());
    }
}

#[test]
fn test_parse_seminar() {
    let reponse = "
    <div id=\"page-body\">
          <div class=\"small-12 medium-12 large-12 cell\">
				<!-- Display ONE detailed seminar. -->
				<div style=\"text-align:center\"><img src=\"/media/uploaded-images/1021.jpg\"></div>
				<h3>Seminar:</h3>
				<p style=\"font-weight: bold; font-size: 1.2em; text-align: center;\">
					Physics Discovery<br/>
					Tuesday, January 19, 2021<br/>
					3:30PM &ndash; 5PM<br />
					Zoom Meeting
				</p>
				<h3>K. G.</h3>
				<p>In this talk</p>
				<p>For questions, please contact: <a href=\"mailto:a@example.edu?subject=Question Regarding - Oden Institute Event:1001\">a@example.edu</a></p>
				&nbsp;Event Stream Link: <a href=\"https://utexas.zoom.us/j/973\" target=\"_blank\">Click Here to Watch</a>
        </div>
    </div>";
    let item = parse_seminar(reponse);
    println!("{:#?}", item);
}

#[test]
fn test_parse_time() {
    fn h(a: Result<(i32, i32), Box<dyn Error>>, b: (i32, i32)) {
        match a {
            Ok(a) => {
                assert_eq!(a, b);
            }
            Err(e) => {
                println!("{:?}", e);
            }
        }
    }
    h(parse_time("10AM"), (10, 0));
    h(parse_time("11AM"), (11, 0));
    h(parse_time("11:30AM"), (11, 30));
    h(parse_time("3PM"), (15, 0));
    h(parse_time("5PM"), (17, 0));
    h(parse_time("5:30PM"), (17, 30));
}