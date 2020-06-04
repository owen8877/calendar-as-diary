use std::fs::File;
use std::path::Path;

use calendar3::{CalendarHub, Error, Event};
use hyper::Client;
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;
use serde_json as json;
use yup_oauth2::{Authenticator, ConsoleApplicationSecret, DefaultAuthenticatorDelegate, DiskTokenStorage, FlowType};

use crate::common::RequestConfig;

pub type CalHub = CalendarHub<Client, Authenticator<DefaultAuthenticatorDelegate, DiskTokenStorage, Client>>;

pub fn calendar_post(hub: &CalHub, config: &RequestConfig, req: Event) {
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
        Ok((_res, events)) => println!("Success: {:#?}", events),
    }
}

pub fn init_hub() -> CalHub {
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