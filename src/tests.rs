use calendar3::{Calendar, CalendarListEntry};

use calendar::CalHub;

use super::*;

#[test]
fn list_all_calendar() {
    env_logger::init();
    let hub = init_hub();

    match get_all_calendars(&hub) {
        None => println!("No calendars."),
        Some(vec) => {
            for entry in vec {
                println!("Name: {}, id: {}.", entry.summary.unwrap(), entry.id.unwrap());
            }
        }
    }
}

fn get_all_calendars(hub: &CalHub) -> Option<Vec<CalendarListEntry>> {
    match hub.calendar_list().list().doit() {
        Ok((_resp, calendar_list)) => calendar_list.items,
        Err(e) => panic!("{:#?}", e),
    }
}

#[tokio::test]
async fn test_integration() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let hub = init_hub();

    let test_calendar_name = "Test purpose only";

    // First clear any calendar of the same name
    match get_all_calendars(&hub) {
        None => println!("No calendars."),
        Some(vec) => {
            for entry in vec {
                if entry.summary.unwrap() == test_calendar_name {
                    match hub.calendars().delete(entry.id.unwrap().as_str()).doit() {
                        Ok(_) => println!("Previous test calendar deleted."),
                        Err(e) => panic!("{:#?}", e),
                    }
                }
            }
        }
    }

    // Then add our new test calendar
    let req = Calendar {
        summary: Some(test_calendar_name.to_string()),
        ..Calendar::default()
    };
    let result = hub.calendars().insert(req).doit();
    match result {
        Ok((_resp, calendar)) => {
            let calendar_id = calendar.id.unwrap();
            println!("The test calendar is {}. Please visit https://calendar.google.com/calendar/r.", &calendar_id);

            let modules: Vec<Box<dyn Module>> = filter_loaded_modules(vec![
                Bilibili::new(Some(calendar_id.clone())),
                LeagueOfLegends::new(Some(calendar_id.clone())),
                Netflix::new(Some(calendar_id.clone())),
                Wakatime::new(Some(calendar_id.clone())),
                Youtube::new(Some(calendar_id.clone())),
            ]);

            for mut module in modules {
                let response = fetch_data(&mut module).await?;
                let events = filter_events_to_be_posted(&mut module, response)?;
                for event in events {
                    calendar_post(&hub, module.get_config(), event.into());
                }
            }
        }
        Err(e) => panic!("Error occurred when insert the test calendar! {}", e),
    }

    Ok(())
}

#[tokio::test]
async fn test_fetch() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let modules: Vec<Box<dyn Module>> = filter_loaded_modules(vec![
        // Bilibili::new(None),
        // LeagueOfLegends::new(None),
        // Netflix::new(None),
        Wakatime::new(None),
        Youtube::new(None),
    ]);

    for mut module in modules {
        let response = fetch_data(&mut module).await?;
        let events = filter_events_to_be_posted(&mut module, response)?;
        println!("{:#?}", events);
    }

    Ok(())
}

#[tokio::test]
async fn test_dump() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let modules: Vec<Box<dyn Module>> = filter_loaded_modules(vec![
        Bilibili::new(None),
        LeagueOfLegends::new(None),
        Netflix::new(None),
        Wakatime::new(None),
        Youtube::new(None),
    ]);

    for mut module in modules {
        let response = fetch_data(&mut module).await?;
        filter_events_to_be_posted(&mut module, response)?;
        // We skip the posting-to-calendar step
        module.dump()
    }

    Ok(())
}

#[tokio::test]
async fn test_interval() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let mut modules: Vec<Box<dyn Module>> = filter_loaded_modules(vec![
        Bilibili::new(None),
        LeagueOfLegends::new(None),
        Netflix::new(None),
        Wakatime::new(None),
        Youtube::new(None),
    ]);

    let mut interval = time::interval(std::time::Duration::from_millis(2 * 1000));

    for _ in 0..3 {
        interval.tick().await;
        info!("Timer picked up at {:#?}", SystemTime::now());
        for mut module in &mut modules {
            let response = fetch_data(&mut module).await?;
            let events = filter_events_to_be_posted(&mut module, response)?;
            println!("{}", events.len());
            // We skip the posting-to-calendar step and the dumping step
        }
        info!("Waiting for timer to pick up...");
    }

    Ok(())
}

#[test]
fn test_filter_event() {
    let events: Vec<EventWithId> = vec![
        EventWithId {
            summary: "1".to_string(),
            description: "".to_string(),
            duration: StartEnd((Utc::now() - Duration::hours(2), Utc::now() - Duration::hours(1) - Duration::minutes(30))),
            id: "".to_string(),
        },
        EventWithId {
            summary: "2".to_string(),
            description: "".to_string(),
            duration: StartEnd((Utc::now() - Duration::hours(3), Utc::now() - Duration::minutes(30))),
            id: "".to_string(),
        },
        EventWithId {
            summary: "3".to_string(),
            description: "".to_string(),
            duration: StartEnd((Utc::now() - Duration::minutes(40), Utc::now() + Duration::minutes(30))),
            id: "".to_string(),
        },
        EventWithId {
            summary: "4".to_string(),
            description: "".to_string(),
            duration: WholeDay(Utc::today()),
            id: "".to_string(),
        },
        EventWithId {
            summary: "5".to_string(),
            description: "".to_string(),
            duration: WholeDay(Utc::today() - Duration::days(1)),
            id: "".to_string(),
        },
        EventWithId {
            summary: "6".to_string(),
            description: "".to_string(),
            duration: WholeDay(Utc::today() + Duration::days(1)),
            id: "".to_string(),
        },
    ];
    let filtered_events = filter_event(events);
    let filtered_ids = filtered_events.iter().map(|e| e.summary.parse::<i32>().unwrap()).collect::<Vec<i32>>();
    assert_eq!(filtered_ids, vec![1, 5])
}