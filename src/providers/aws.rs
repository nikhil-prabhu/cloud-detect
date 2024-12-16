//! Amazon Web Services (AWS).

use std::path::Path;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::fs;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, info, instrument};

use crate::{Provider, ProviderId};

const METADATA_URI: &str = "http://169.254.169.254";
const METADATA_PATH: &str = "/latest/dynamic/instance-identity/document";
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
    #[instrument(skip_all)]
    async fn identify(&self, tx: Sender<ProviderId>) {
        info!("Checking Amazon Web Services");
        if self.check_vendor_files(VENDOR_FILES).await
            || self.check_metadata_server(METADATA_URI).await
        {
            info!("Identified Amazon Web Services");
            let res = tx.send(IDENTIFIER).await;

            if let Err(err) = res {
                error!("Error sending message: {:?}", err);
            }
        }
    }
}

impl AWS {
    /// Tries to identify AWS via metadata server.
    #[instrument(skip_all)]
    async fn check_metadata_server(&self, metadata_uri: &str) -> bool {
        let url = format!("{}{}", metadata_uri, METADATA_PATH);
        debug!("Checking {} metadata using url: {}", IDENTIFIER, url);

        match reqwest::get(url).await {
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
    #[instrument(skip_all)]
    async fn check_vendor_files<I>(&self, vendor_files: I) -> bool
    where
        I: IntoIterator,
        I::Item: AsRef<Path>,
    {
        for vendor_file in vendor_files {
            debug!(
                "Checking {} vendor file: {}",
                IDENTIFIER,
                vendor_file.as_ref().display()
            );

            if vendor_file.as_ref().is_file() {
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
