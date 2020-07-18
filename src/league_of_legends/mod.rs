use std::collections::HashSet;
use std::error::Error;

use chrono::{TimeZone, Utc, Duration};
use serde::Deserialize;

use crate::calendar::event::*;
use crate::common::*;
use serde_json::Number;

const IDENTIFIER: &str = "league_of_legends";

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LeagueOfLegendsResponse {
    platform_id: String,
    account_id: Number,
    games: GamesObject,
}

#[derive(Debug, Deserialize)]
struct GamesObject {
    games: Vec<GameObject>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GameObject {
    game_id: Number,
    platform_id: String,
    game_creation: Number,
    game_duration: Number,
    queue_id: Number,
    game_mode: String,
    game_type: String,
    participant_identities: Vec<ParticipantIdentity>,
}

#[derive(Debug, Deserialize)]
struct ParticipantIdentity {
    player: Player,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Player {
    summoner_name: String,
    match_history_uri: String,
    account_id: Number,
}

impl GameObject {
    fn id(self: &GameObject) -> String {
        format!("{}|{}|{}|{}", IDENTIFIER, self.platform_id, self.game_id, self.participant_identities[0].player.account_id)
    }
}

pub struct LeagueOfLegends {
    request_config: RequestConfig,
    event_ids: HashSet<String>,
}

impl Module for LeagueOfLegends {
    fn new(calendar_id: Option<String>) -> Result<Box<dyn Module>, Box<dyn Error>> {
        let request_config = RequestConfig::new(IDENTIFIER, calendar_id)?;
        let event_ids = read_dumped_event_id(IDENTIFIER).unwrap_or(HashSet::new());
        Ok(Box::new(LeagueOfLegends {
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

    fn process_response_into_event_with_id(&self, response: String) -> Result<Vec<EventWithId>, Box<dyn Error>> {
        let (account_id, items) = match serde_json::from_str::<LeagueOfLegendsResponse>(response.as_str()) {
            Ok(json) => (json.account_id, json.games.games),
            Err(e) => panic!("Cannot parse {} response!, {:#?}", IDENTIFIER, e),
        };

        Ok(items.iter().map(|item| {
            let game_link = format!("https://matchhistory.na.leagueoflegends.com/en/#match-details/NA1/{}/{}", item.game_id, account_id);
            EventWithId::new(PartialDayEvent {
                summary: format!("[League of Legends] {}", item.game_mode),
                description: format!("[link] {}\n[mode] {} {}\n[hash] {}", game_link, item.game_mode, item.game_type, item.id()),
                start: Utc.ymd(1970, 1, 1).and_hms(0, 0, 0) + Duration::seconds(item.game_creation.as_i64().unwrap() / 1000),
                end: Utc.ymd(1970, 1, 1).and_hms(0, 0, 0) + Duration::seconds(item.game_creation.as_i64().unwrap() / 1000 + item.game_duration.as_i64().unwrap()),
            }.into(), item.id())
        }).collect())
    }
}