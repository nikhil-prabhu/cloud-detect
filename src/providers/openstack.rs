//! OpenStack.

use std::path::Path;

use async_trait::async_trait;
use tokio::fs;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, info, instrument};

use crate::{Provider, ProviderId};

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
pub const IDENTIFIER: ProviderId = ProviderId::OpenStack;

pub struct OpenStack;

#[async_trait]
impl Provider for OpenStack {
    fn identifier(&self) -> ProviderId {
        IDENTIFIER
    }

    /// Tries to identify OpenStack using all the implemented options.
    #[instrument(skip_all)]
    async fn identify(&self, tx: Sender<ProviderId>) {
        info!("Checking OpenStack");
        if self
            .check_vendor_files(PRODUCT_NAME_FILE, CHASSIS_ASSET_TAG_FILE)
            .await
            || self.check_metadata_server(METADATA_URI).await
        {
            info!("Identified OpenStack");
            let res = tx.send(IDENTIFIER).await;

            if let Err(err) = res {
                error!("Error sending message: {:?}", err);
            }
        }
    }
}

impl OpenStack {
    /// Tries to identify OpenStack via metadata server.
    #[instrument(skip_all)]
    async fn check_metadata_server(&self, metadata_uri: &str) -> bool {
        let url = format!("{}{}", metadata_uri, METADATA_PATH);
        debug!("Checking {} metadata using url: {}", IDENTIFIER, url);

        match reqwest::get(url).await {
            Ok(resp) => resp.status().is_success(),
            Err(err) => {
                error!("Error making request: {:?}", err);
                false
            }
        }
    }

    /// Tries to identify OpenStack using vendor file(s).
    #[instrument(skip_all)]
    async fn check_vendor_files<P: AsRef<Path>>(
        &self,
        product_name_file: P,
        chassis_asset_tag_file: P,
    ) -> bool {
        debug!(
            "Checking {} vendor file: {}",
            IDENTIFIER,
            product_name_file.as_ref().display()
        );

        if product_name_file.as_ref().is_file() {
            match fs::read_to_string(product_name_file).await {
                Ok(content) => {
                    if PRODUCT_NAMES.iter().any(|&name| content.contains(name)) {
                        return true;
                    }
                }
                Err(err) => {
                    error!("Error reading file: {:?}", err);
                }
            }
        }

        debug!(
            "Checking {} vendor file: {}",
            IDENTIFIER,
            chassis_asset_tag_file.as_ref().display(),
        );

        if chassis_asset_tag_file.as_ref().is_file() {
            match fs::read_to_string(chassis_asset_tag_file).await {
                Ok(content) => {
                    if CHASSIS_ASSET_TAGS
                        .iter()
                        .any(|&name| content.contains(name))
                    {
                        return true;
                    }
                }
                Err(err) => {
                    error!("Error reading file: {:?}", err);
                }
            }
        }

        false
    }
}
