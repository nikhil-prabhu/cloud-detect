use std::fs;
use std::path::Path;

use async_trait::async_trait;
use serde::Deserialize;
use tracing::{debug, error, Level};

use crate::Provider;

const METADATA_URL: &str = "http://169.254.169.254/latest/dynamic/instance-identity/document";
const VENDOR_FILES: [&str; 2] = [
    "/sys/class/dmi/id/product_version",
    "/sys/class/dmi/id/bios_vendor",
];
pub const IDENTIFIER: &str = "aws";

#[derive(Deserialize)]
struct MetadataResponse {
    #[serde(rename = "imageId")]
    image_id: String,
    #[serde(rename = "instanceId")]
    instance_id: String,
}

pub(crate) struct AWS;

#[async_trait]
impl Provider for AWS {
    /// Tries to identify AWS using all the implemented options.
    async fn identify(&self) -> bool {
        crate::identify(self, IDENTIFIER).await
    }

    /// Tries to identify AWS via metadata server.
    async fn check_metadata_server(&self) -> bool {
        let span = tracing::span!(Level::TRACE, "check_metadata_server");
        let _enter = span.enter();

        debug!(
            "Checking {} metadata using url: {}",
            IDENTIFIER, METADATA_URL
        );
        return match reqwest::get(METADATA_URL).await {
            Ok(resp) => {
                return match resp.json::<MetadataResponse>().await {
                    Ok(resp) => {
                        resp.image_id.starts_with("ami-") && resp.instance_id.starts_with("i-")
                    }
                    Err(err) => {
                        error!("Error reading response: {:?}", err);
                        false
                    }
                };
            }
            Err(err) => {
                error!("Error making request: {:?}", err);
                false
            }
        };
    }

    /// Tries to identify AWS using vendor file(s).
    async fn check_vendor_file(&self) -> bool {
        let span = tracing::span!(Level::TRACE, "check_vendor_file");
        let _enter = span.enter();

        for vendor_file in VENDOR_FILES {
            debug!("Checking {} vendor file: {}", IDENTIFIER, vendor_file);
            let vendor_file = Path::new(vendor_file);

            if vendor_file.is_file() {
                return match fs::read_to_string(vendor_file) {
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
