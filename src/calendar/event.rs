use calendar3::{Event, EventDateTime};
use chrono::{Date, DateTime, Utc};

#[derive(Debug)]
pub struct EventWithId {
    pub summary: String,
    pub description: String,
    pub duration: Duration,
    pub id: String,
}

#[derive(Debug)]
pub enum Duration {
    StartEnd((DateTime<Utc>, DateTime<Utc>)),
    WholeDay(Date<Utc>),
}

impl From<EventWithId> for Event {
    fn from(e: EventWithId) -> Self {
        let ((start_time, start_date), (end_time, end_date)) = match e.duration {
            Duration::StartEnd((start, end)) => (
                (Some(start.to_rfc3339()), None),
                (Some(end.to_rfc3339()), None),
            ),
            Duration::WholeDay(day) => (
                (None, Some(day.format("%Y-%m-%d").to_string())),
                (None, Some(day.format("%Y-%m-%d").to_string())),
            ),
        };
        Event {
            summary: Some(e.summary),
            description: Some(e.description),
            start: Some(EventDateTime {
                date_time: start_time,
                date: start_date,
                ..EventDateTime::default()
            }),
            end: Some(EventDateTime {
                date_time: end_time,
                date: end_date,
                ..EventDateTime::default()
            }),
            ..Event::default()
        }
    }
}