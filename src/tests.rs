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
        },
    }
}

fn get_all_calendars(hub: &CalHub) -> Option<Vec<CalendarListEntry>> {
    match hub.calendar_list().list().doit() {
        Ok((resp, calendar_list)) => calendar_list.items,
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
        },
    }

    // Then add our new test calendar
    let mut req = Calendar {
        summary: Some(test_calendar_name.to_string()),
        ..Calendar::default()
    };
    let result = hub.calendars().insert(req).doit();
    match result {
        Ok((resp, calendar)) => {
            let calendar_id = calendar.id.unwrap();
            println!("The test calendar is {}. Please visit https://calendar.google.com/calendar/r/settings/addcalendar to add that.", &calendar_id);

            let modules: Vec<Box<dyn Module>> = vec![
                Box::new(Bilibili::new(Some(calendar_id.clone()))),
                Box::new(Netflix::new(Some(calendar_id.clone()))),
            ];

            for mut module in modules {
                let response = fetch_data(&mut module, &hub).await?;
                let events = filter_events_to_be_posted(&mut module, response);
                for event in events {
                    calendar_post(&hub, module.get_config(), event.event.clone());
                }
            }
        },
        Err(e) => panic!("Error occurred when insert the test calendar! {}", e),
    }

    Ok(())
}

#[tokio::test]
async fn test_dump() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    let hub = init_hub();

    let modules: Vec<Box<dyn Module>> = vec![
        Box::new(Bilibili::new(None)),
        Box::new(Netflix::new(None)),
    ];

    for mut module in modules {
        let response = fetch_data(&mut module, &hub).await?;
        let events = filter_events_to_be_posted(&mut module, response);
        // We skip the posting-to-calendar step
        module.dump()
    }

    Ok(())
}
