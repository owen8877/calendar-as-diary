use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use chrono::{DateTime, Duration, TimeZone, Utc};
use lazy_static::lazy_static;
use regex::Regex;
use scraper::{ElementRef, Html, Selector};

use crate::calendar::event::*;
use crate::calendar::event::Duration::StartEnd;
use crate::common::*;
use crate::league_of_graphs::ParseError::*;

const IDENTIFIER: &str = "league_of_graphs";

#[derive(Debug)]
struct GameObject {
    id: u64,
    creation: DateTime<Utc>,
    duration: i64,
    mode: String,
}

impl GameObject {
    fn id(self: &GameObject) -> String {
        format!("{}|{}", IDENTIFIER, self.id)
    }
}


#[derive(Debug, Clone)]
enum ParseError {
    DurationError(String),
    ParseFails(String),
    IdError(String),
    DateError(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DurationError(t) => write!(f, "Duration error: {}", t),
            ParseFails(t) => write!(f, "Parse fails for {}", t),
            IdError(t) => write!(f, "Id error: {}", t),
            DateError(t) => write!(f, "Date error: {}", t),
        }
    }
}

impl Error for ParseError {}

fn parse_selector(selection: &str) -> Result<Selector, ParseError> {
    Selector::parse(selection).map_err(|_e| ParseFails(format!("selector {}", selection)))
}

fn select_helper<'a, 'b>(element: &ElementRef<'a>, selector: &'b Selector) -> Result<ElementRef<'a>, ParseError> {
    element.select(selector).next().ok_or_else(|| ParseFails(format!("fails to select {:?}", selector)))
}

fn parse_duration(s: &str) -> Result<i64, Box<dyn Error>> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"(\d+)min (\d+)s").unwrap();
    }
    let mat = RE.captures_iter(s).next().ok_or(Box::new(DurationError(s.to_string())))?;

    let minute = mat[1].parse::<i64>()?;
    let second = mat[2].parse::<i64>()?;
    return Ok(minute * 60 + second);
}

fn parse_individual_game(script: &str, mode: &str, duration: &str) -> Result<GameObject, Box<dyn Error>> {
    lazy_static! {
        static ref ID_RE: Regex = Regex::new(r"match-(\d+)").unwrap();
        static ref DATE_RE: Regex = Regex::new(r"new Date\((\d+)").unwrap();
    }

    let game_id = ID_RE.captures_iter(script).next().ok_or(Box::new(IdError(script.to_string())))?;
    let game_date = DATE_RE.captures_iter(script).next().ok_or(Box::new(DateError(script.to_string())))?;

    Ok(GameObject {
        id: game_id[1].parse::<u64>()?,
        creation: Utc.timestamp_millis(game_date[1].parse::<i64>()?),
        duration: parse_duration(duration.trim())?,
        mode: mode.trim().to_string(),
    })
}

fn parse_games(response: &str) -> Result<Vec<GameObject>, Box<dyn Error>> {
    let document = Html::parse_document(response);
    let tr_selector = parse_selector("tr[class=\"\"]")?;
    let script_selector = parse_selector("script")?;
    let div_game_mode_selector = parse_selector("div.gameMode")?;
    let div_game_duration_selector = parse_selector("div.gameDuration")?;

    Ok(document.select(&tr_selector).into_iter().map(|row| {
        // println!("{}", row.inner_html());
        let script_content = select_helper(&row, &script_selector)?.inner_html();
        let game_mode = select_helper(&row, &div_game_mode_selector)?.inner_html();
        let game_duration = select_helper(&row, &div_game_duration_selector)?.inner_html();
        parse_individual_game(&script_content, &game_mode, &game_duration)
    }).filter_map(|r| match r {
        Ok(r) => Some(r),
        Err(e) => {
            println!("{:?}", e);
            None
        }
    }).collect())
}

pub struct LeagueOfGraphs {
    request_config: RequestConfig,
    event_ids: HashSet<String>,
}

impl Module for LeagueOfGraphs {
    fn new(calendar_id: Option<String>) -> Result<Box<dyn Module>, Box<dyn Error>> {
        let request_config = RequestConfig::new(IDENTIFIER, calendar_id)?;
        let event_ids = read_dumped_event_id(IDENTIFIER).unwrap_or(HashSet::new());
        Ok(Box::new(LeagueOfGraphs {
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

    fn need_for_detail(&self, _response: &String) -> Option<Vec<String>> {
        None
    }

    fn process_response_into_event_with_id(&self, responses: Vec<String>) -> Result<Vec<EventWithId>, Box<dyn Error>> {
        match parse_games(responses[0].as_str()) {
            Ok(v) => Ok(v.into_iter().map(|r| {
                EventWithId {
                    summary: format!("[League of Legends] {}", r.mode),
                    description: format!("[link] https://www.leagueofgraphs.com/match/na/{}\n[mode] {}\n[hash] {}", r.id, r.mode, r.id()),
                    duration: StartEnd(r.creation, r.creation + Duration::seconds(r.duration)),
                    id: r.id(),
                }
            }).collect()),
            Err(e) => {
                info!("{}", e);
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use lazy_static::lazy_static;
    use regex::Regex;
    use scraper::Html;

    use crate::league_of_graphs::{parse_duration, parse_games, parse_selector, select_helper};

    #[test]
    fn test_regex() {
        let reponse = "
<a href=\"/match/na/4471269577#participant9\">
<a class=\"full-cell\" href=\"/match/na/4471295235#participant1\">
";
        lazy_static! {
            static ref RE: Regex = Regex::new(r"/match/na/\d+").unwrap();
        }
        for mat in RE.find_iter(reponse) {
            println!("Found: {}", mat.as_str());
        }
    }

    #[test]
    fn test_parse_helper() {
        fn worker() -> Result<String, Box<dyn Error>> {
            let response = "<div><span></span></div>";
            let document = Html::parse_document(response);
            let div_selector = parse_selector("div")?;
            let div_element = select_helper(&document.root_element(), &div_selector)?;
            let a_selector = parse_selector("a")?;
            let a_element = select_helper(&div_element, &a_selector)?;
            Ok(a_element.inner_html())
        }

        match worker() {
            Ok(r) => { println!("{:?}", r) }
            Err(e) => { println!("{}", e) }
        }
    }


    #[test]
    fn test_parse_duration() {
        match parse_duration("10min 30s") {
            Ok(r) => assert_eq!(r, 630),
            Err(e) => {
                println!("{:?}", e);
                panic!()
            }
        }
    }

    #[test]
    fn test_parse_game() {
        let response = "
<table class=\"data_table relative recentGamesTable inverted_rows_color\">
    <tbody>
    	<tr class=\"recentGamesTableHeader hide-for-dark\"></tr>
        <tr class=\"recentGamesTableHeader filtersBlock\"></tr>
        <tr class=\"\">
			<td class=\"championCellLight\">
			    <a href=\"/match/na/4471269577#participant9\">
			        <div></div>
			        <div class=\"spells\"></div>
			    </a>
			</td>

			<td class=\"championCellDark\">
			    <div class=\"winIndicator victory\"></div>
	            <a href=\"/match/na/4471269577#participant9\">
	                <div class=\"championContainer\">
			            <div></div>
			            <div class=\"spells\"></div>
                    </div>
                </a>
		    </td>
		    <script type=\"text/javascript\">
			    var newTooltipData = {\"match-4471269577\": (new Date(1666411915909).toLocaleDateString() + \" \" + new Date(1666411915909).toLocaleTimeString()) + \" - 10min 20s\"};
			    if (window.tooltipData) {
			        window.tooltipData = Object.assign(window.tooltipData, newTooltipData);
			    } else {
			        window.tooltipData = newTooltipData;
			    }
			</script>

			<td class=\"resultCellLight text-center\">
			    <a class=\"display-block\" href=\"/match/na/4471269577#participant9\">
			        <div class=\"victoryDefeatText victory\">Victory</div>        <div class=\"gameMode requireTooltip\" tooltip-vertical-offset=\"0\" tooltip=\"ARAM\">ARAM        </div>
			        <div class=\"gameDate requireTooltip\" tooltip-vertical-offset=\"0\" tooltip-var=\"match-4471269577\">16 hours ago        </div>
			        <div class=\"gameDuration\">10min 20s        </div>
			        <div class=\"lpChange\"></div>
			    </a>
			</td>

			<td class=\"resultCellDark nopadding\"></td>
			<td class=\"text-center nopadding kdaColumn \"></td>
    		<td class=\"itemsColumnLight\"></td>
			<td class=\"itemsColumnDark\"></td>
			<td class=\"summonersTdLight\"></td>
			<td class=\"summonersTdDark\"></td>
		</tr>

		<tr class=\"\">
			<td class=\"championCellLight\">
			    <a href=\"/match/na/4471295235#participant1\">
			        <div></div>
			        <div class=\"spells\"></div>
			    </a>
			</td>

			<td class=\"championCellDark\">
			    <div class=\"winIndicator defeat\"></div>
			    <a href=\"/match/na/4471295235#participant1\"></a>
			</td>
			<script type=\"text/javascript\">
			    var newTooltipData = {\"match-4471295235\": (new Date(1666410585008).toLocaleDateString() + \" \" + new Date(1666410585008).toLocaleTimeString()) + \" - 18min 31s\"};
			    if (window.tooltipData) {
			        window.tooltipData = Object.assign(window.tooltipData, newTooltipData);
			    } else {
			        window.tooltipData = newTooltipData;
			    }
			</script>

			<td class=\"resultCellLight text-center\">
			    <a class=\"display-block\" href=\"/match/na/4471295235#participant1\">
			        <div class=\"victoryDefeatText defeat\">Defeat</div>        <div class=\"gameMode requireTooltip\" tooltip-vertical-offset=\"0\" tooltip=\"ARAM\">ARAM        </div>
			        <div class=\"gameDate requireTooltip\" tooltip-vertical-offset=\"0\" tooltip-var=\"match-4471295235\">16 hours ago        </div>
			        <div class=\"gameDuration\">18min 31s        </div>
			        <div class=\"lpChange\"></div>
			    </a>
			</td>

			<td class=\"resultCellDark nopadding\"></td>
			<td class=\"text-center nopadding kdaColumn \"></td>
		    <td class=\"itemsColumnLight\"></td>
			<td class=\"itemsColumnDark\"></td>
		    <td class=\"summonersTdLight\"></td>
			<td class=\"summonersTdDark\"></td>
		</tr>
    </tbody>
</table>
";
        match parse_games(response) {
            Ok(item) => println!("{:#?}", item),
            Err(e) => println!("{:#?}", e),
        }
    }
}
