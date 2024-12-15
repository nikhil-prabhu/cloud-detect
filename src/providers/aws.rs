//! Amazon Web Services (AWS).

use std::path::Path;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::fs;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, info, Level};

use crate::{Provider, ProviderId};

const METADATA_URL: &str = "http://169.254.169.254/latest/dynamic/instance-identity/document";
const VENDOR_FILES: [&str; 2] = [
    "/sys/class/dmi/id/product_version",
    "/sys/class/dmi/id/bios_vendor",
];
pub const IDENTIFIER: ProviderId = ProviderId::AWS;

#[derive(Deserialize)]
struct MetadataResponse {
    #[serde(rename = "imageId")]
    image_id: String,
    #[serde(rename = "instanceId")]
    instance_id: String,
}

pub struct AWS;

#[async_trait]
impl Provider for AWS {
    fn identifier(&self) -> ProviderId {
        IDENTIFIER
    }

    /// Tries to identify AWS using all the implemented options.
    async fn identify(&self, tx: Sender<ProviderId>) {
        info!("Checking Amazon Web Services");
        if self.check_vendor_file().await || self.check_metadata_server().await {
            let res = tx.send(IDENTIFIER).await;

            if let Err(err) = res {
                error!("Error sending message: {:?}", err);
            }
        }
    }
}

impl AWS {
    /// Tries to identify AWS via metadata server.
    async fn check_metadata_server(&self) -> bool {
        let span = tracing::span!(Level::TRACE, "check_metadata_server");
        let _enter = span.enter();

        debug!(
            "Checking {} metadata using url: {}",
            IDENTIFIER, METADATA_URL
        );
        match reqwest::get(METADATA_URL).await {
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

    /// Tries to identify AWS using vendor file(s).
    async fn check_vendor_file(&self) -> bool {
        let span = tracing::span!(Level::TRACE, "check_vendor_file");
        let _enter = span.enter();

        for vendor_file in VENDOR_FILES {
            debug!("Checking {} vendor file: {}", IDENTIFIER, vendor_file);
            let vendor_file = Path::new(vendor_file);

            if vendor_file.is_file() {
                return match fs::read_to_string(vendor_file).await {
                    Ok(content) => content.to_lowercase().contains("amazon"),
                    Err(err) => {
                        error!("Error reading file: {:?}", err);
                        false
                    }
                };
            }
        }

        false
    }
}
