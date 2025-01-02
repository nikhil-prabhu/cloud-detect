//! Alibaba Cloud.

use std::fs;
use std::path::Path;
use std::sync::mpsc::SyncSender;
use std::time::Duration;

use tracing::{debug, error, info, instrument};

use crate::blocking::Provider;
use crate::ProviderId;

const METADATA_URI: &str = "http://100.100.100.200";
const METADATA_PATH: &str = "/latest/meta-data/latest/meta-data/instance/virtualization-solution";
const VENDOR_FILE: &str = "/sys/class/dmi/id/product_name";
const IDENTIFIER: ProviderId = ProviderId::Alibaba;

pub(crate) struct Alibaba;

impl Provider for Alibaba {
    fn identifier(&self) -> ProviderId {
        IDENTIFIER
    }

    #[instrument(skip_all)]
    fn identify(&self, tx: SyncSender<ProviderId>, timeout: Duration) {
        info!("Checking Alibaba Cloud");
        if self.check_vendor_file(VENDOR_FILE) || self.check_metadata_server(METADATA_URI, timeout)
        {
            info!("Identified Alibaba Cloud");
            if let Err(err) = tx.send(IDENTIFIER) {
                error!("Error sending message: {:?}", err);
            }
        }
    }
}

impl Alibaba {
    #[instrument(skip_all)]
    fn check_metadata_server(&self, metadata_uri: &str, timeout: Duration) -> bool {
        let url = format!("{}{}", metadata_uri, METADATA_PATH);
        debug!("Checking {} metadata using url: {}", IDENTIFIER, url);

        let client = if let Ok(client) = reqwest::blocking::Client::builder()
            .timeout(timeout)
            .build()
        {
            client
        } else {
            error!("Error creating client");
            return false;
        };

        match client.get(url).send() {
            Ok(resp) => match resp.text() {
                Ok(text) => text.contains("ECS Virt"),
                Err(err) => {
                    error!("Error reading response: {:?}", err);
                    false
                }
            },
            Err(err) => {
                error!("Error sending request: {:?}", err);
                false
            }
        }
    }

    #[instrument(skip_all)]
    fn check_vendor_file<P: AsRef<Path>>(&self, vendor_file: P) -> bool {
        debug!(
            "Checking {} vendor file: {}",
            IDENTIFIER,
            vendor_file.as_ref().display()
        );

        if vendor_file.as_ref().is_file() {
            return match fs::read_to_string(vendor_file) {
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
    use mockito::Server;
    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn test_check_metadata_server_success() {
        let mut server = Server::new();

        let url = server.url();

        let mock = server
            .mock("GET", METADATA_PATH)
            .with_status(200)
            .with_body("ECS Virt")
            .create();

        let provider = Alibaba;
        let result = provider.check_metadata_server(&url, Duration::from_secs(1));

        mock.assert();
        assert!(result);
    }

    #[test]
    fn test_check_metadata_server_failure() {
        let mut server = Server::new();

        let url = server.url();

        let mock = server
            .mock("GET", METADATA_PATH)
            .with_status(200)
            .with_body("ABC")
            .create();

        let provider = Alibaba;
        let result = provider.check_metadata_server(&url, Duration::from_secs(1));

        mock.assert();
        assert!(!result);
    }

    #[test]
    fn test_check_vendor_file_success() -> Result<()> {
        let mut vendor_file = NamedTempFile::new()?;
        vendor_file.write_all("Alibaba Cloud ECS".as_bytes())?;

        let provider = Alibaba;
        let result = provider.check_vendor_file(vendor_file.path());

        assert!(result);

        Ok(())
    }

    #[test]
    fn test_check_vendor_file_failure() -> Result<()> {
        let vendor_file = NamedTempFile::new()?;

        let provider = Alibaba;
        let result = provider.check_vendor_file(vendor_file.path());

        assert!(!result);

        Ok(())
    }
}
