//! OpenStack.

use std::path::Path;

use async_trait::async_trait;
use tokio::fs;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, info, Level};

use crate::{Provider, ProviderId};

const METADATA_URL: &str = "http://169.254.169.254/openstack/";
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
    async fn identify(&self, tx: Sender<ProviderId>) {
        info!("Checking OpenStack");
        if self.check_vendor_file().await || self.check_metadata_server().await {
            let res = tx.send(IDENTIFIER).await;

            if let Err(err) = res {
                error!("Error sending message: {:?}", err);
            }
        }
    }
}

impl OpenStack {
    /// Tries to identify OpenStack via metadata server.
    async fn check_metadata_server(&self) -> bool {
        let span = tracing::span!(Level::TRACE, "check_metadata_server");
        let _enter = span.enter();

        debug!(
            "Checking {} metadata using url: {}",
            IDENTIFIER, METADATA_URL
        );
        match reqwest::get(METADATA_URL).await {
            Ok(resp) => resp.status().is_success(),
            Err(err) => {
                error!("Error making request: {:?}", err);
                false
            }
        }
    }

    /// Tries to identify OpenStack using vendor file(s).
    async fn check_vendor_file(&self) -> bool {
        let span = tracing::span!(Level::TRACE, "check_vendor_file");
        let _enter = span.enter();

        debug!("Checking {} vendor file: {}", IDENTIFIER, PRODUCT_NAME_FILE);
        let product_name_file = Path::new(PRODUCT_NAME_FILE);

        if product_name_file.is_file() {
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
            IDENTIFIER, CHASSIS_ASSET_TAG_FILE
        );
        let chassis_asset_tag_file = Path::new(CHASSIS_ASSET_TAG_FILE);

        if chassis_asset_tag_file.is_file() {
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
