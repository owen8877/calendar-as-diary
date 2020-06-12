use calendar3::{Event, EventDateTime};
use chrono::{Date, DateTime, Utc};

pub struct PartialDayEvent {
    pub summary: String,
    pub description: String,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

pub struct WholeDayEvent {
    pub summary: String,
    pub description: String,
    pub date: Date<Utc>,
}

impl From<PartialDayEvent> for Event {
    fn from(item: PartialDayEvent) -> Self {
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

impl From<WholeDayEvent> for Event {
    fn from(item: WholeDayEvent) -> Self {
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

#[derive(std::fmt::Debug)]
pub struct EventWithId {
    pub event: Event,
    pub id: String,
}

impl EventWithId {
    pub fn new(event: Event, id: String) -> EventWithId {
        EventWithId {
            event,
            id,
        }
    }
}