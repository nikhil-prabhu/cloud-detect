//! DigitalOcean.

use std::fs;
use std::path::Path;
use std::sync::mpsc::SyncSender;
use std::time::Duration;

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use tracing::{debug, error, info, instrument};

use crate::blocking::Provider;
use crate::ProviderId;

const METADATA_URI: &str = "http://169.254.169.254";
const METADATA_PATH: &str = "/metadata/v1.json";
const VENDOR_FILE: &str = "/sys/class/dmi/id/sys_vendor";
pub(crate) const IDENTIFIER: ProviderId = ProviderId::DigitalOcean;

pub(crate) struct DigitalOcean;

#[derive(Serialize, Deserialize)]
struct MetadataResponse {
    droplet_id: usize,
}

impl Provider for DigitalOcean {
    fn identifier(&self) -> ProviderId {
        IDENTIFIER
    }

    /// Tries to identify DigitalOcean using all the implemented options.
    #[instrument(skip_all)]
    fn identify(&self, tx: SyncSender<ProviderId>, timeout: Duration) {
        info!("Checking DigitalOcean");
        if self.check_vendor_file(VENDOR_FILE) || self.check_metadata_server(METADATA_URI, timeout)
        {
            info!("Identified DigitalOcean");
            if let Err(err) = tx.send(IDENTIFIER) {
                error!("Error sending message: {:?}", err);
            }
        }
    }
}

impl DigitalOcean {
    /// Tries to identify DigitalOcean via metadata server.
    #[instrument(skip_all)]
    fn check_metadata_server(&self, metadata_uri: &str, timeout: Duration) -> bool {
        let url = format!("{metadata_uri}{METADATA_PATH}");
        debug!("Checking {} metadata using url: {}", IDENTIFIER, url);

        let client = if let Ok(client) = Client::builder().timeout(timeout).build() {
            client
        } else {
            error!("Error creating client");
            return false;
        };

        match client.get(url).send() {
            Ok(resp) => match resp.json::<MetadataResponse>() {
                Ok(resp) => resp.droplet_id > 0,
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

    /// Tries to identify DigitalOcean using vendor file(s).
    #[instrument(skip_all)]
    fn check_vendor_file<P: AsRef<Path>>(&self, vendor_file: P) -> bool {
        debug!(
            "Checking {} vendor file: {}",
            IDENTIFIER,
            vendor_file.as_ref().display()
        );

        if vendor_file.as_ref().is_file() {
            return match fs::read_to_string(vendor_file) {
                Ok(content) => content.contains("DigitalOcean"),
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
            .with_body(r#"{"droplet_id": 123}"#)
            .create();

        let provider = DigitalOcean;
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

        let provider = DigitalOcean;
        let result = provider.check_metadata_server(&url, Duration::from_secs(1));

        mock.assert();
        assert!(!result);
    }

    #[test]
    fn test_check_vendor_file_success() -> Result<()> {
        let mut vendor_file = NamedTempFile::new()?;
        vendor_file.write_all(b"DigitalOcean")?;

        let provider = DigitalOcean;
        let result = provider.check_vendor_file(vendor_file.path());

        assert!(result);

        Ok(())
    }

    #[test]
    fn test_check_vendor_file_failure() -> Result<()> {
        let vendor_file = NamedTempFile::new()?;

        let provider = DigitalOcean;
        let result = provider.check_vendor_file(vendor_file.path());

        assert!(!result);

        Ok(())
    }
}
