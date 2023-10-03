use std::fs;
use std::path::Path;

use async_trait::async_trait;
use serde::Deserialize;

use crate::Provider;

const METADATA_URL: &str = "http://169.254.169.254/latest/dynamic/instance-identity/document";
const VENDOR_FILES: [&str; 2] = [
    "/sys/class/dmi/id/product_version",
    "/sys/class/dmi/id/bios_vendor",
];

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
    /// Returns the identifier string for AWS.
    fn identifier() -> &'static str {
        "aws"
    }

    /// Tries to identify AWS using all the implemented options.
    async fn identify() -> bool {
        Self::check_vendor_file().await || Self::check_metadata_server().await
    }

    /// Tries to identify AWS via metadata server.
    async fn check_metadata_server() -> bool {
        return match reqwest::get(METADATA_URL).await {
            Ok(resp) => {
                return match resp.json::<MetadataResponse>().await {
                    Ok(resp) => resp.image_id.starts_with("ami-") && resp.instance_id.starts_with("i-"),
                    Err(_) => false,
                };
            }
            Err(_) => false,
        };
    }

    /// Tries to identify AWS using vendor file(s).
    async fn check_vendor_file() -> bool {
        for vendor_file in VENDOR_FILES {
            let vendor_file = Path::new(vendor_file);

            if vendor_file.is_file() {
                return match fs::read_to_string(vendor_file) {
                    Ok(content) => content.to_lowercase().contains("amazon"),
                    Err(_) => false,
                };
            }
        }

        false
    }
}
