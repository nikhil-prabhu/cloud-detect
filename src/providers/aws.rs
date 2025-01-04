//! Amazon Web Services (AWS).

use std::path::Path;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use tokio::fs;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, info, instrument};

use crate::{Provider, ProviderId};

const METADATA_URI: &str = "http://169.254.169.254";
const METADATA_PATH: &str = "/latest/dynamic/instance-identity/document";
const METADATA_TOKEN_PATH: &str = "/latest/api/token";
const PRODUCT_VERSION_FILE: &str = "/sys/class/dmi/id/product_version";
const BIOS_VENDOR_FILE: &str = "/sys/class/dmi/id/bios_vendor";
pub const IDENTIFIER: ProviderId = ProviderId::AWS;

#[derive(Serialize, Deserialize)]
struct MetadataResponse {
    #[serde(rename = "imageId")]
    image_id: String,
    #[serde(rename = "instanceId")]
    instance_id: String,
}

pub struct Aws;

#[async_trait]
impl Provider for Aws {
    fn identifier(&self) -> ProviderId {
        IDENTIFIER
    }

    /// Tries to identify AWS using all the implemented options.
    #[instrument(skip_all)]
    async fn identify(&self, tx: Sender<ProviderId>, timeout: Duration) {
        info!("Checking Amazon Web Services");
        if self.check_product_version_file(PRODUCT_VERSION_FILE).await
            || self.check_bios_vendor_file(BIOS_VENDOR_FILE).await
            || self
                .check_metadata_server_imdsv2(METADATA_URI, timeout)
                .await
            || self
                .check_metadata_server_imdsv1(METADATA_URI, timeout)
                .await
        {
            info!("Identified Amazon Web Services");
            let res = tx.send(IDENTIFIER).await;

            if let Err(err) = res {
                error!("Error sending message: {:?}", err);
            }
        }
    }
}

impl Aws {
    /// Tries to identify AWS via metadata server (using IMDSv2).
    #[instrument(skip_all)]
    async fn check_metadata_server_imdsv2(&self, metadata_uri: &str, timeout: Duration) -> bool {
        let token_url = format!("{}{}", metadata_uri, METADATA_TOKEN_PATH);
        debug!("Retrieving {} IMDSv2 token from: {}", IDENTIFIER, token_url);

        let client = if let Ok(client) = reqwest::Client::builder().timeout(timeout).build() {
            client
        } else {
            error!("Error creating client");
            return false;
        };

        let token = match client
            .get(token_url)
            .header("X-aws-ec2-metadata-token-ttl-seconds", "60")
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
            error!("IMDSv2 token is empty");
            return false;
        }

        // Request to use the token to get metadata
        let metadata_url = format!("{}{}", metadata_uri, METADATA_PATH);
        debug!(
            "Checking {} metadata using url: {}",
            IDENTIFIER, metadata_url
        );

        let resp = match client
            .get(metadata_url)
            .header("X-aws-ec2-metadata-token", token)
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
            Ok(metadata) => {
                metadata.image_id.starts_with("ami-") && metadata.instance_id.starts_with("i-")
            }
            Err(err) => {
                error!("Error reading response: {:?}", err);
                false
            }
        }
    }

    /// Tries to identify AWS via metadata server (using IMDSv1).
    #[instrument(skip_all)]
    async fn check_metadata_server_imdsv1(&self, metadata_uri: &str, timeout: Duration) -> bool {
        let url = format!("{}{}", metadata_uri, METADATA_PATH);
        debug!("Checking {} metadata using url: {}", IDENTIFIER, url);

        let client = if let Ok(client) = reqwest::Client::builder().timeout(timeout).build() {
            client
        } else {
            error!("Error creating client");
            return false;
        };

        match client.get(url).send().await {
            Ok(resp) => match resp.json::<MetadataResponse>().await {
                Ok(resp) => resp.image_id.starts_with("ami-") && resp.instance_id.starts_with("i-"),
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

    /// Tries to identify AWS using the product version file.
    #[instrument(skip_all)]
    async fn check_product_version_file<P: AsRef<Path>>(&self, product_version_file: P) -> bool {
        debug!(
            "Checking {} product version file: {}",
            IDENTIFIER,
            product_version_file.as_ref().display()
        );

        if product_version_file.as_ref().is_file() {
            return match fs::read_to_string(product_version_file).await {
                Ok(content) => content.to_lowercase().contains("amazon"),
                Err(err) => {
                    error!("Error reading file: {:?}", err);
                    false
                }
            };
        }

        false
    }

    /// Tries to identify AWS using the BIOS vendor file.
    #[instrument(skip_all)]
    async fn check_bios_vendor_file<P: AsRef<Path>>(&self, bios_vendor_file: P) -> bool {
        debug!(
            "Checking {} BIOS vendor file: {}",
            IDENTIFIER,
            bios_vendor_file.as_ref().display()
        );

        if bios_vendor_file.as_ref().is_file() {
            return match fs::read_to_string(bios_vendor_file).await {
                Ok(content) => content.to_lowercase().contains("amazon"),
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
    use wiremock::matchers::{header, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;

    #[tokio::test]
    async fn test_check_metadata_server_imdsv2_success() {
        let mock_server = MockServer::start().await;

        Mock::given(path(METADATA_TOKEN_PATH))
            .and(header("X-aws-ec2-metadata-token-ttl-seconds", "60"))
            .respond_with(ResponseTemplate::new(200).set_body_string("123abc"))
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(path(METADATA_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(MetadataResponse {
                image_id: "ami-123abc".to_string(),
                instance_id: "i-123abc".to_string(),
            }))
            .expect(1)
            .mount(&mock_server)
            .await;

        let provider = Aws;
        let metadata_uri = mock_server.uri();
        let result = provider
            .check_metadata_server_imdsv2(&metadata_uri, Duration::from_secs(1))
            .await;

        assert!(result);
    }

    #[tokio::test]
    async fn test_check_metadata_server_imdsv2_failure() {
        let mock_server = MockServer::start().await;

        Mock::given(path(METADATA_TOKEN_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_string("123abc"))
            .expect(1)
            .mount(&mock_server)
            .await;

        Mock::given(path(METADATA_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(MetadataResponse {
                image_id: "abc".to_string(),
                instance_id: "abc".to_string(),
            }))
            .expect(1)
            .mount(&mock_server)
            .await;

        let provider = Aws;
        let metadata_uri = mock_server.uri();
        let result = provider
            .check_metadata_server_imdsv2(&metadata_uri, Duration::from_secs(1))
            .await;

        assert!(!result);
    }

    #[tokio::test]
    async fn test_check_metadata_server_imdsv1_success() {
        let mock_server = MockServer::start().await;
        Mock::given(path(METADATA_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(MetadataResponse {
                image_id: "ami-123abc".to_string(),
                instance_id: "i-123abc".to_string(),
            }))
            .expect(1)
            .mount(&mock_server)
            .await;

        let provider = Aws;
        let metadata_uri = mock_server.uri();
        let result = provider
            .check_metadata_server_imdsv1(&metadata_uri, Duration::from_secs(1))
            .await;

        assert!(result);
    }

    #[tokio::test]
    async fn test_check_metadata_server_imdsv1_failure() {
        let mock_server = MockServer::start().await;
        Mock::given(path(METADATA_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(MetadataResponse {
                image_id: "abc".to_string(),
                instance_id: "abc".to_string(),
            }))
            .expect(1)
            .mount(&mock_server)
            .await;

        let provider = Aws;
        let metadata_uri = mock_server.uri();
        let result = provider
            .check_metadata_server_imdsv1(&metadata_uri, Duration::from_secs(1))
            .await;

        assert!(!result);
    }

    #[tokio::test]
    async fn test_check_product_version_file_success() -> Result<()> {
        let mut product_version_file = NamedTempFile::new()?;
        product_version_file.write_all(b"amazon")?;

        let provider = Aws;
        let result = provider
            .check_product_version_file(product_version_file.path())
            .await;

        assert!(result);

        Ok(())
    }

    #[tokio::test]
    async fn test_check_product_version_file_failure() -> Result<()> {
        let product_version_file = NamedTempFile::new()?;

        let provider = Aws;
        let result = provider
            .check_product_version_file(product_version_file.path())
            .await;

        assert!(!result);

        Ok(())
    }

    #[tokio::test]
    async fn test_check_bios_vendor_file_success() -> Result<()> {
        let mut bios_vendor_file = NamedTempFile::new()?;
        bios_vendor_file.write_all(b"amazon")?;

        let provider = Aws;
        let result = provider
            .check_bios_vendor_file(bios_vendor_file.path())
            .await;

        assert!(result);

        Ok(())
    }

    #[tokio::test]
    async fn test_check_bios_vendor_file_failure() -> Result<()> {
        let bios_vendor_file = NamedTempFile::new()?;

        let provider = Aws;
        let result = provider
            .check_bios_vendor_file(bios_vendor_file.path())
            .await;

        assert!(!result);

        Ok(())
    }
}
