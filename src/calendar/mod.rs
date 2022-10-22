use calendar3::{CalendarHub, Error, oauth2};
use calendar3::api::Event;
use hyper::client::HttpConnector;
use hyper_rustls::HttpsConnector;

use crate::common::RequestConfig;

pub mod event;

pub type CalHub = CalendarHub<HttpsConnector<HttpConnector>>;

pub async fn calendar_post(hub: &mut CalHub, config: &RequestConfig, req: Event) {
    let result = hub.events().insert(req, config.calendar_id.as_str()).doit().await;

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
            | Error::JsonDecodeError(_, _) => error!("Error occurred in posting an event: {}.", e),
            _ => {}
        },
        Ok((_res, event)) => {
            info!("Success in posting an event \"{}\" which starts at {}.", match &event.summary {
                Some(str) => str.clone(),
                None => "[No summary]".to_string(),
            }, match &event.start {
                Some(start) => {
                    if let Some(date) = &start.date {
                        date.clone()
                    } else if let Some(datetime) = &start.date_time {
                        datetime.clone()
                    } else {
                        "[No start time]".to_string()
                    }
                }
                None => "[No start time]".to_string(),
            });
            debug!("Detail info about this event: {:?}.", &event);
        }
    }
}

pub async fn init_hub() -> CalHub {
    let secret: oauth2::ApplicationSecret = yup_oauth2::read_application_secret("config/clientsecret.json")
        .await.expect("client secret not found!");
    let auth = oauth2::InstalledFlowAuthenticator::builder(secret, oauth2::InstalledFlowReturnMethod::Interactive)
        .persist_tokens_to_disk("config/tokenstorage.json").build().await.unwrap();
    let hub = CalendarHub::new(hyper::Client::builder().build(hyper_rustls::HttpsConnectorBuilder::new().with_native_roots().https_or_http().enable_http1().enable_http2().build()), auth);
    hub
}

#[cfg(test)]
mod tests {
    use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod};

    #[tokio::test]
    async fn test_yup_oauth2() {
        // From the official test example.
        env_logger::init();
        let secret = yup_oauth2::read_application_secret("config/clientsecret.json").await.expect("clientsecret.json");
        let auth = InstalledFlowAuthenticator::builder(secret, InstalledFlowReturnMethod::Interactive)
            .persist_tokens_to_disk("config/tokencache.json").build().await.unwrap();
        let scopes = &[
            "https://www.googleapis.com/auth/calendar",
            "https://www.googleapis.com/auth/calendar.events",
        ];
        match auth.token(scopes).await {
            Ok(token) => println!("The token is {:?}", token),
            Err(e) => println!("error: {:?}", e),
        }
    }
}