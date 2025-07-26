//! OpenStack.

use std::fs;
use std::path::Path;
use std::sync::mpsc::SyncSender;
use std::time::Duration;

use reqwest::blocking::Client;
use tracing::{debug, error, info, instrument};

use crate::blocking::Provider;
use crate::ProviderId;

const METADATA_URI: &str = "http://169.254.169.254";
const METADATA_PATH: &str = "/openstack/";
const PRODUCT_NAME_FILE: &str = "/sys/class/dmi/id/product_name";
const PRODUCT_NAMES: [&str; 2] = ["Openstack Nova", "OpenStack Compute"];
const CHASSIS_ASSET_TAG_FILE: &str = "/sys/class/dmi/id/chassis_asset_tag";
const CHASSIS_ASSET_TAGS: [&str; 5] = [
    "HUAWEICLOUD",
    "OpenTelekomCloud",
    "SAP CCloud VM",
    "OpenStack Nova",
    "OpenStack Compute",
];
pub(crate) const IDENTIFIER: ProviderId = ProviderId::OpenStack;

pub(crate) struct OpenStack;

impl Provider for OpenStack {
    fn identifier(&self) -> ProviderId {
        IDENTIFIER
    }

    /// Tries to identify OpenStack using all the implemented options.
    #[instrument(skip_all)]
    fn identify(&self, tx: SyncSender<ProviderId>, timeout: Duration) {
        info!("Checking OpenStack");
        if self.check_vendor_files(PRODUCT_NAME_FILE, CHASSIS_ASSET_TAG_FILE)
            || self.check_metadata_server(METADATA_URI, timeout)
        {
            info!("Identified OpenStack");
            if let Err(err) = tx.send(IDENTIFIER) {
                error!("Error sending message: {:?}", err);
            }
        }
    }
}

impl OpenStack {
    /// Tries to identify OpenStack via metadata server.
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
            Ok(resp) => resp.status().is_success(),
            Err(err) => {
                error!("Error making request: {:?}", err);
                false
            }
        }
    }

    /// Tries to identify OpenStack using vendor file(s).
    #[instrument(skip_all)]
    fn check_vendor_files<P: AsRef<Path>>(
        &self,
        product_name_file: P,
        chassis_asset_tag_file: P,
    ) -> bool {
        debug!(
            "Checking {} vendor file: {}",
            IDENTIFIER,
            product_name_file.as_ref().display(),
        );

        if product_name_file.as_ref().is_file() {
            match fs::read_to_string(product_name_file) {
                Ok(content) => {
                    if PRODUCT_NAMES.iter().any(|name| content.contains(name)) {
                        return true;
                    }
                }
                Err(err) => {
                    error!("Error reading file: {:?}", err);
                }
            };
        }

        debug!(
            "Checking {} vendor file: {}",
            IDENTIFIER,
            chassis_asset_tag_file.as_ref().display(),
        );

        if chassis_asset_tag_file.as_ref().is_file() {
            match fs::read_to_string(chassis_asset_tag_file) {
                Ok(content) => {
                    if CHASSIS_ASSET_TAGS.iter().any(|tag| content.contains(tag)) {
                        return true;
                    }
                }
                Err(err) => {
                    error!("Error reading file: {:?}", err);
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

        let mock = server.mock("GET", METADATA_PATH).with_status(200).create();

        let provider = OpenStack;
        let result = provider.check_metadata_server(&url, Duration::from_secs(1));

        mock.assert();
        assert!(result);
    }

    #[test]
    fn test_check_metadata_server_failure() {
        let mut server = Server::new();
        let url = server.url();

        let mock = server.mock("GET", METADATA_PATH).with_status(500).create();

        let provider = OpenStack;
        let result = provider.check_metadata_server(&url, Duration::from_secs(1));

        mock.assert();
        assert!(!result);
    }

    #[test]
    fn test_check_vendor_files_success() -> Result<()> {
        let mut product_name_file = NamedTempFile::new()?;
        let mut chassis_asset_tag_file = NamedTempFile::new()?;

        product_name_file.write_all(PRODUCT_NAMES[0].as_bytes())?;
        chassis_asset_tag_file.write_all(PRODUCT_NAMES[0].as_bytes())?;

        let provider = OpenStack;
        let result =
            provider.check_vendor_files(product_name_file.path(), chassis_asset_tag_file.path());

        assert!(result);

        Ok(())
    }

    #[test]
    fn test_check_vendor_files_failure() -> Result<()> {
        let product_name_file = NamedTempFile::new()?;
        let chassis_asset_tag_file = NamedTempFile::new()?;

        let provider = OpenStack;
        let result =
            provider.check_vendor_files(product_name_file.path(), chassis_asset_tag_file.path());

        assert!(!result);

        Ok(())
    }
}
