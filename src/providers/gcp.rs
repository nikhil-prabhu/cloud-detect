//! Google Cloud Platform (GCP).

use std::fs;
use std::path::Path;

use async_trait::async_trait;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, info, Level};

use crate::Provider;

const METADATA_URL: &str = "http://metadata.google.internal/computeMetadata/v1/instance/tags";
const VENDOR_FILE: &str = "/sys/class/dmi/id/product_name";
pub const IDENTIFIER: &str = "gcp";

pub struct GCP;

#[async_trait]
impl Provider for GCP {
    fn identifier(&self) -> &'static str {
        IDENTIFIER
    }

    /// Tries to identify GCP using all the implemented options.
    async fn identify(&self, tx: Sender<&'static str>) {
        info!("Checking Google Cloud Platform");
        if self.check_vendor_file().await || self.check_metadata_server().await {
            tx.send(IDENTIFIER).await.unwrap();
        }
    }
}

impl GCP {
    /// Tries to identify GCP via metadata server.
    async fn check_metadata_server(&self) -> bool {
        let span = tracing::span!(Level::TRACE, "check_metadata_server");
        let _enter = span.enter();

        debug!(
            "Checking {} metadata using url: {}",
            IDENTIFIER, METADATA_URL
        );
        let client = reqwest::Client::new();
        let req = client.get(METADATA_URL).header("Metadata-Flavor", "Google");

        req.send().await.is_ok()
    }

    /// Tries to identify GCP using vendor file(s).
    async fn check_vendor_file(&self) -> bool {
        let span = tracing::span!(Level::TRACE, "check_vendor_file");
        let _enter = span.enter();

        debug!("Checking {} vendor file: {}", IDENTIFIER, VENDOR_FILE);
        let vendor_file = Path::new(VENDOR_FILE);

        if vendor_file.is_file() {
            return match fs::read_to_string(vendor_file) {
                Ok(content) => content.contains("Google"),
                Err(err) => {
                    error!("Error reading file: {:?}", err);
                    false
                }
            };
        }

        false
    }
}
