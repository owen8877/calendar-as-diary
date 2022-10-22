use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use chrono::{DateTime, FixedOffset, TimeZone};
use lazy_static::lazy_static;
use regex::Regex;
use scraper::{Html, Selector};

use crate::calendar::event::*;
use crate::calendar::event::Duration::StartEnd;
use crate::common::*;
use crate::ut_oden_seminar::ParseError::*;

const IDENTIFIER: &str = "ut_oden_seminar";

#[derive(Debug)]
struct Item {
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
    ComingSoon,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            UnwrapNone(t) => write!(f, "Try to unwrap None at: {}", t),
            UnknownMonth(t) => write!(f, "Unknown month: {}", t),
            ParseFails(t) => write!(f, "Parse fails: {}", t),
            CaptureFails(t) => write!(f, "Capture fails: {}", t),
            ComingSoon => write!(f, "Event comes soon, ignored for now.")
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
    if hour == 12 {
        if time_str.contains("AM") {
            hour = 0;
        }
    } else {
        hour += 12;
        if time_str.contains("AM") {
            hour -= 12;
        }
    }
    Ok((hour, minute))
}

fn parse_selector(selection: &str) -> Result<Selector, ParseError> {
    Selector::parse(selection).map_err(|_e| ParseFails(format!("selector {}", selection)))
}

fn parse_seminar(response: &str) -> Result<Item, Box<dyn Error>> {
    let document = Html::parse_document(response);
    let div_selector = parse_selector("div.cell.large-8")?;
    let cell_div = document.select(&div_selector).next().ok_or(UnwrapNone("div.cell.large-8".to_string()))?;

    let title_h1_selector = parse_selector("h1.event__title")?;
    let speaker_span_selector = parse_selector("span.event__speaker")?;
    let affiliation_span_selector = parse_selector("span.event__speaker-affiliation")?;

    let title = cell_div.select(&title_h1_selector).next().ok_or(UnwrapNone("title".to_string()))?.inner_html();
    if title.starts_with("Coming soon") {
        return Err(Box::new(ComingSoon))
    }
    let speaker = cell_div.select(&speaker_span_selector).next().ok_or(UnwrapNone("speaker".to_string()))?.inner_html();
    let affiliation = cell_div.select(&affiliation_span_selector).next().ok_or(UnwrapNone("affiliation".to_string()))?.inner_html();

    let logistics_selector = parse_selector("div.event__logistics")?;
    let p_selector = parse_selector("p")?;
    let a_selector = parse_selector("a")?;
    let span_selector = parse_selector("span")?;

    let logistics_div = cell_div.select(&logistics_selector).next().ok_or(UnwrapNone("div.event__logistics".to_string()))?;
    let info_paragraph = logistics_div.select(&p_selector).next().ok_or(UnwrapNone("first (info) paragraph".to_string()))?;
    let info_str = info_paragraph.inner_html().chars().filter(|c| c != &'\t').filter(|c| c != &'\n').collect::<String>();
    let location_paragraph = logistics_div.select(&p_selector).skip(1).next().ok_or(UnwrapNone("second (location) paragraph".to_string()))?;
    let location = match location_paragraph.select(&a_selector).next() {
        Some(zoom_element) => {
            let href = zoom_element.value().attr("href").unwrap();
            let place = zoom_element.inner_html();
            format!("{}: {}", place, href)
        }
        None => {
            location_paragraph.select(&span_selector).next().ok_or(UnwrapNone("span or a element for location".to_string()))?.inner_html()
        }
    };
    let abs = cell_div.select(&p_selector).skip(5).next().ok_or(UnwrapNone("abstract paragraph".to_string()))?.inner_html();
    let description = format!("{}, {}\n{}\n{}", speaker, affiliation, abs, location.as_str());

    let info_split: Vec<&str> = info_str.split("<br>").collect();
    let date_str = info_split[1].trim();
    let time_str = info_split[0].trim().to_string();

    let date_captures = Regex::new(r"(\w+) (\w+) (\d+), (\d+)")?
        .captures_iter(date_str).next().ok_or(CaptureFails("date capture".to_string()))?;
    let day = date_captures.get(3).ok_or(CaptureFails("day".to_string()))?
        .as_str().parse::<i32>()?;
    let month_str = date_captures.get(2).ok_or(CaptureFails("month".to_string()))?
        .as_str();
    let month_strs = vec!["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let (month, _) = month_strs.iter().enumerate().find(|(_i, m)| *m == &month_str).ok_or(UnknownMonth(month_str.to_string()))?;
    let year = date_captures.get(4).ok_or(CaptureFails("year".to_string()))?
        .as_str().parse::<i32>().unwrap();

    let time_split: Vec<&str> = time_str.split(" – ").collect();
    let (start_hour, start_minute) = parse_time(time_split[0])?;
    let (end_hour, end_minute) = parse_time(time_split[1])?;

    // let last_breadcrumb_selector = parse_selector("span.breadcrumb__last")?;
    // let seminar_str = document.select(&last_breadcrumb_selector).next().ok_or("span.breadcrumb__last".to_string())?.inner_html();
    // let seminar_id = (seminar_str.as_str().trim().split(' ').next().unwrap()).to_string().parse::<u32>()?;

    let seminar_captures = Regex::new(r"news-and-events/events/(\d*)")?
        .captures_iter(response).next().ok_or(CaptureFails("seminar id".to_string()))?;
    let seminar_id = seminar_captures.get(1).ok_or(CaptureFails("seminar id 1".to_string()))?.as_str().parse::<u32>()?;


    let time_zone: FixedOffset = FixedOffset::west(5 * 60 * 60); // Daylight saving mode
    Ok(Item {
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
        lazy_static! {
            static ref RE: Regex = Regex::new(r"/news-and-events/events/\d+").unwrap();
        }
        let base_url = Regex::new(r"https://[^/]*").unwrap().find(self.request_config.url.as_str())?.as_str();
        Some(RE.find_iter(response).map(|mat| base_url.to_string() + mat.as_str()).collect())
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
            .map(|r: Item| {
                EventWithId {
                    id: r.id(),
                    summary: r.title,
                    description: r.description,
                    duration: StartEnd(DateTime::from(r.start), DateTime::from(r.end)),
                }
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use lazy_static::lazy_static;
    use regex::Regex;

    use crate::ut_oden_seminar::{parse_seminar, parse_time};

    #[test]
    fn test_regex() {
        let response = "
<link type=\"text/css\" rel=\"stylesheet\" href=\"/static/news_events/events/events.css\" />
<h3 class=\"event__title\"><a href=\"/news-and-events/events/1727---R\">Stochastic </a></h3>
<p class=\"oden--event-card-location\">
<h3 class=\"event__title\"><a href=\"/news-and-events/events/1708---A\">Combining estimation</a></h3>
    ";
        lazy_static! {
            static ref RE: Regex = Regex::new(r"/news-and-events/events/\d+").unwrap();
        }
        for mat in RE.find_iter(response) {
            println!("Found: {}", mat.as_str());
        }
    }

    #[test]
    fn test_regex_date() {
        let response = "Tuesday Oct 11, 2022";
        lazy_static! {
            static ref RE: Regex = Regex::new(r"(\w+) (\w+) (\d+), (\d+)").unwrap();
        }
        for mat in RE.captures_iter(response) {
            println!("0: {}, 1: {}, 2: {}, 3: {}", &mat[0], &mat[1], &mat[2], &mat[3]);
        }
    }

    #[test]
    fn test_parse_seminar() {
        let response = "
<link rel=\"canonical\" href=\"https://oden.utexas.edu/news-and-events/events/1708/\" />
<div class=\"cell small-12 medium-12 large-8 \">
    <p class=\"event__eyebrow\">
        Upcoming Event:
        <span class=\"event__sponsor oden institute seminar\"> Oden Institute Seminar</span>
    </p>

    <h1 class=\"event__title\">Combining collections of high fidelity and reduced order for large-scale system state estimation</h1>

    <p class=\"\">
        <span class=\"event__speaker\">Andrey Popov</span>, <span class=\"event__speaker-affiliation\">ASE/EM Dept., UT Austin</span>
    </p>

    <div class=\"event__logistics\">
        <p>
            3:30 – 5PM <br>
            Tuesday Oct 11, 2022
        </p>
        <p>
            <a href=\"https://utexas.zoom.us/j/965\" target=\"_blank\">POB 6.304 &amp; Zoom</a>
        </p>
    </div>

    <h2>Abstract</h2>
    <p>** This seminar will be presented live in POB 6.304 and via Zoom.**</p>
    <p>Physics-based high-fidelity models that predict large scale natural processes such as the weather require immense computational resources for a result that is often not a good representation of the truth, as dangerously few realizations of these models are used to make predictions. There once was a thought that cheap data-driven models would replace their physics-based counterparts.&nbsp; This reality has not come to pass, as models purely based on data do not provide reliable physically consistent predictions. By leveraging and extending the multilevel Monte Carlo approach, this talk proposes a framework, model forest data assimilation, in which state prediction and correction (the data assimilation problem) can be performed given a large collection of physics-based and data-driven models. Thus, this approach aims to combine the speed of many modern data-driven models, specifically types of reduced order models, with the accuracy of the much slower physics-based high-fidelity models in a statistically consistent manner. Applying this approach to the ensemble Kalman filter shows great promise in significantly reducing the reliance on expensive high-fidelity models.</p>

    <h2>Biography</h2>
    <p>Andrey obtained his&nbsp;Ph.D. in Computer Science from Virginia Tech (VT), and his B.S. in Mathematics from Rensselaer Polytechnic Institute (RPI).&nbsp;&nbsp;During the course of his Ph.D., Andrey has worked on ensemble filtering techniques including work with multifidelity data assimilation and with covariance shrinkage.&nbsp;He has also worked on extending and applying non-linear dimensionality reduction techniques to constructing efficient reduced order models for use in scientific applications.&nbsp;Andrey's other interests include data-driven science, knowledge-guided machine learning, and time integration</p>
</div>";
        match parse_seminar(response) {
            Ok(item) => println!("{:#?}", item),
            Err(e) => println!("{:#?}", e),
        }
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
}