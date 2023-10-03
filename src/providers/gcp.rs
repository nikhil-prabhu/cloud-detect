use std::fs;
use std::path::Path;

use async_trait::async_trait;

use crate::Provider;

const METADATA_URL: &str = "http://metadata.google.internal/computeMetadata/v1/instance/tags";
const VENDOR_FILE: &str = "/sys/class/dmi/id/product_name";

pub(crate) struct GCP;

#[async_trait]
impl Provider for GCP {
    fn identifier() -> &'static str {
        "gcp"
    }

    /// Tries to identify GCP using all the implemented options.
    async fn identify() -> bool {
        Self::check_vendor_file().await || Self::check_metadata_server().await
    }

    /// Tries to identify GCP via metadata server.
    async fn check_metadata_server() -> bool {
        let client = reqwest::Client::new();
        let req = client.get(METADATA_URL).header("Metadata-Flavor", "Google");

        req.send().await.is_ok()
    }

    /// Tries to identify GCP using vendor file(s).
    async fn check_vendor_file() -> bool {
        let vendor_file = Path::new(VENDOR_FILE);

        if vendor_file.is_file() {
            return match fs::read_to_string(vendor_file) {
                Ok(content) => content.contains("Google"),
                Err(_) => false,
            };
        }

        false
    }
}
