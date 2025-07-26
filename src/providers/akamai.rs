//! Akamai Cloud

use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, info, instrument};

use crate::{Provider, ProviderId};

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

#[async_trait]
impl Provider for Akamai {
    fn identifier(&self) -> ProviderId {
        IDENTIFIER
    }

    /// Tries to identify Akamai using all the implemented options.
    #[instrument(skip_all)]
    async fn identify(&self, tx: Sender<ProviderId>, timeout: Duration) {
        info!("Checking Akamai Cloud");
        if self.check_metadata_server(METADATA_URI, timeout).await {
            info!("Identified Akamai Cloud");
            let res = tx.send(IDENTIFIER).await;

            if let Err(err) = res {
                error!("Error sending message: {:?}", err);
            }
        }
    }
}

impl Akamai {
    #[instrument(skip_all)]
    async fn check_metadata_server(&self, metadata_uri: &str, timeout: Duration) -> bool {
        let token_url = format!("{}{}", metadata_uri, METADATA_TOKEN_PATH);
        debug!("Retrieving {} token from: {}", IDENTIFIER, token_url);

        let client = if let Ok(client) = reqwest::Client::builder().timeout(timeout).build() {
            client
        } else {
            error!("Error creating client");
            return false;
        };

        let token = match client
            .get(token_url)
            .header("Metadata-Token-Expiry-Seconds", "60")
            .send()
            .await
        {
            Ok(resp) => resp.text().await.unwrap_or_else(|err| {
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
            .await
        {
            Ok(resp) => resp.json::<MetadataResponse>().await,
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
    use wiremock::matchers::{header, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[tokio::test]
    async fn test_check_metadata_server_success() {
        let mock_server = MockServer::start().await;

        Mock::given(path(METADATA_TOKEN_PATH))
            .and(header("Metadata-Token-Expiry-Seconds", "60"))
            .respond_with(ResponseTemplate::new(200).set_body_string("123abc"))
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(path(METADATA_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(MetadataResponse {
                id: 123,
                host_uuid: "123456".to_string(),
            }))
            .expect(1)
            .mount(&mock_server)
            .await;

        let provider = Akamai;
        let metadata_uri = mock_server.uri();
        let result = provider
            .check_metadata_server(&metadata_uri, Duration::from_secs(1))
            .await;

        assert!(result);
    }

    #[tokio::test]
    async fn test_check_metadata_server_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(path(METADATA_TOKEN_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_string("123abc"))
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(path(METADATA_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(MetadataResponse {
                id: 0,
                host_uuid: "".to_string(),
            }))
            .expect(1)
            .mount(&mock_server)
            .await;

        let provider = Akamai;
        let metadata_uri = mock_server.uri();
        let result = provider
            .check_metadata_server(&metadata_uri, Duration::from_secs(1))
            .await;

        assert!(!result);
    }
}
