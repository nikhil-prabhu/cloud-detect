//! Akamai Cloud

use std::sync::mpsc::SyncSender;
use std::time::Duration;

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument};

use crate::blocking::Provider;
use crate::ProviderId;

const METADATA_URI: &str = "http://169.254.169.254";
const METADATA_PATH: &str = "/v1/instance";
const METADATA_TOKEN_PATH: &str = "/v1/token";
pub(crate) const IDENTIFIER: ProviderId = ProviderId::Akamai;

#[derive(Serialize, Deserialize)]
struct MetadataResponse {
    id: isize,
    host_uuid: String,
}

pub(crate) struct Akamai;

impl Provider for Akamai {
    fn identifier(&self) -> ProviderId {
        IDENTIFIER
    }

    /// Tries to identify Akamai using all the implemented options.
    fn identify(&self, tx: SyncSender<ProviderId>, timeout: Duration) {
        info!("Checking Akamai Cloud");
        if self.check_metadata_server(METADATA_URI, timeout) {
            info!("Identified Akamai Cloud");
            let res = tx.send(IDENTIFIER);

            if let Err(err) = res {
                error!("Error sending message: {:?}", err);
            }
        }
    }
}

impl Akamai {
    #[instrument(skip_all)]
    fn check_metadata_server(&self, metadata_uri: &str, timeout: Duration) -> bool {
        let token_url = format!("{}{}", metadata_uri, METADATA_TOKEN_PATH);
        debug!("Retrieving {} token from: {}", IDENTIFIER, token_url);

        let client = if let Ok(client) = Client::builder().timeout(timeout).build() {
            client
        } else {
            error!("Error creating client");
            return false;
        };

        let token = match client
            .get(token_url)
            .header("Metadata-Token-Expiry-Seconds", "60")
            .send()
        {
            Ok(resp) => resp.text().unwrap_or_else(|err| {
                error!("Error reading token: {:?}", err);
                String::new()
            }),
            Err(err) => {
                error!("Error making request: {:?}", err);
                return false;
            }
        };

        if token.is_empty() {
            error!("Token is empty");
            return false;
        }

        // Request to use token to get metadata
        let metadata_url = format!("{}{}", metadata_uri, METADATA_PATH);
        debug!(
            "Checking {} metadata using url: {}",
            IDENTIFIER, metadata_url,
        );

        let resp = match client
            .get(metadata_url)
            .header("Metadata-Token", token)
            .send()
        {
            Ok(resp) => resp.json::<MetadataResponse>(),
            Err(err) => {
                error!("Error making request: {:?}", err);
                return false;
            }
        };

        match resp {
            Ok(metadata) => metadata.id > 0 && !metadata.host_uuid.is_empty(),
            Err(err) => {
                error!("Error reading response: {:?}", err);
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use mockito::Server;

    use super::*;

    #[test]
    fn test_check_metadata_server_success() {
        let mut server = Server::new();
        let url = server.url();

        let token_mock = server
            .mock("GET", METADATA_TOKEN_PATH)
            .with_status(200)
            .with_body("123abc")
            .create();

        let metadata_mock = server
            .mock("GET", METADATA_PATH)
            .with_status(200)
            .with_body(r#"{"id": 123, "host_uuid": "123abc"}"#)
            .create();

        let provider = Akamai;
        let result = provider.check_metadata_server(&url, Duration::from_secs(1));

        token_mock.assert();
        metadata_mock.assert();
        assert!(result);
    }

    #[test]
    fn test_check_metadata_server_failure() {
        let mut server = Server::new();
        let url = server.url();

        let token_mock = server
            .mock("GET", METADATA_TOKEN_PATH)
            .with_status(200)
            .with_body("123abc")
            .create();

        let metadata_mock = server
            .mock("GET", METADATA_PATH)
            .with_status(200)
            .with_body(r#"{"id": 0, "host_uuid": ""}"#)
            .create();

        let provider = Akamai;
        let result = provider.check_metadata_server(&url, Duration::from_secs(1));

        token_mock.assert();
        metadata_mock.assert();
        assert!(!result);
    }
}
