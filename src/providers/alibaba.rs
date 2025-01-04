//! Alibaba Cloud.

use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use tokio::fs;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, info, instrument};

use crate::{Provider, ProviderId};

const METADATA_URI: &str = "http://100.100.100.200";
const METADATA_PATH: &str = "/latest/meta-data/latest/meta-data/instance/virtualization-solution";
const VENDOR_FILE: &str = "/sys/class/dmi/id/product_name";
pub const IDENTIFIER: ProviderId = ProviderId::Alibaba;

pub struct Alibaba;

#[async_trait]
impl Provider for Alibaba {
    fn identifier(&self) -> ProviderId {
        IDENTIFIER
    }

    /// Tries to identify Alibaba Cloud using all the implemented options.
    #[instrument(skip_all)]
    async fn identify(&self, tx: Sender<ProviderId>, timeout: Duration) {
        info!("Checking Alibaba Cloud");
        if self.check_vendor_file(VENDOR_FILE).await
            || self.check_metadata_server(METADATA_URI, timeout).await
        {
            info!("Identified Alibaba Cloud");
            let res = tx.send(IDENTIFIER).await;

            if let Err(err) = res {
                error!("Error sending message: {:?}", err);
            }
        }
    }
}

impl Alibaba {
    /// Tries to identify Alibaba via metadata server.
    #[instrument(skip_all)]
    async fn check_metadata_server(&self, metadata_uri: &str, timeout: Duration) -> bool {
        let url = format!("{}{}", metadata_uri, METADATA_PATH);
        debug!("Checking {} metadata using url: {}", IDENTIFIER, url);

        let client = if let Ok(client) = reqwest::Client::builder().timeout(timeout).build() {
            client
        } else {
            error!("Error creating client");
            return false;
        };

        match client.get(url).send().await {
            Ok(resp) => match resp.text().await {
                Ok(text) => text.contains("ECS Virt"),
                Err(err) => {
                    error!("Error reading response: {:?}", err);
                    false
                }
            },
            Err(err) => {
                error!("Error making request: {:?}", err);
                false
            }
        }
    }

    /// Tries to identify Alibaba using vendor file(s).
    #[instrument(skip_all)]
    async fn check_vendor_file<P: AsRef<Path>>(&self, vendor_file: P) -> bool {
        debug!(
            "Checking {} vendor file: {}",
            IDENTIFIER,
            vendor_file.as_ref().display()
        );

        if vendor_file.as_ref().is_file() {
            return match fs::read_to_string(vendor_file).await {
                Ok(content) => content.contains("Alibaba Cloud ECS"),
                Err(err) => {
                    error!("Error reading file: {:?}", err);
                    false
                }
            };
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use anyhow::Result;
    use tempfile::NamedTempFile;
    use wiremock::matchers::path;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[tokio::test]
    async fn test_check_metadata_server_success() {
        let mock_server = MockServer::start().await;
        Mock::given(path(METADATA_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_string("ECS Virt"))
            .expect(1)
            .mount(&mock_server)
            .await;

        let provider = Alibaba;
        let metadata_uri = mock_server.uri();
        let result = provider
            .check_metadata_server(&metadata_uri, Duration::from_secs(1))
            .await;

        assert!(result);
    }

    #[tokio::test]
    async fn test_check_metadata_server_failure() {
        let mock_server = MockServer::start().await;
        Mock::given(path(METADATA_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_string("ABC"))
            .expect(1)
            .mount(&mock_server)
            .await;

        let provider = Alibaba;
        let metadata_uri = mock_server.uri();
        let result = provider
            .check_metadata_server(&metadata_uri, Duration::from_secs(1))
            .await;

        assert!(!result);
    }

    #[tokio::test]
    async fn test_check_vendor_file_success() -> Result<()> {
        let mut vendor_file = NamedTempFile::new()?;
        vendor_file.write_all(b"Alibaba Cloud ECS")?;

        let provider = Alibaba;
        let result = provider.check_vendor_file(vendor_file.path()).await;

        assert!(result);

        Ok(())
    }

    #[tokio::test]
    async fn test_check_vendor_file_failure() -> Result<()> {
        let vendor_file = NamedTempFile::new()?;

        let provider = Alibaba;
        let result = provider.check_vendor_file(vendor_file.path()).await;

        assert!(!result);

        Ok(())
    }
}
