//! Microsoft Azure.

use std::path::Path;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::fs;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, info, instrument};

use crate::{Provider, ProviderId};

const METADATA_URL: &str = "http://169.254.169.254/metadata/instance?api-version=2017-12-01";
const VENDOR_FILE: &str = "/sys/class/dmi/id/sys_vendor";
pub const IDENTIFIER: ProviderId = ProviderId::Azure;

#[derive(Deserialize)]
struct Compute {
    #[serde(rename = "vmId")]
    vm_id: String,
}

#[derive(Deserialize)]
struct MetadataResponse {
    compute: Compute,
}

pub struct Azure;

#[async_trait]
impl Provider for Azure {
    fn identifier(&self) -> ProviderId {
        IDENTIFIER
    }

    /// Tries to identify Azure using all the implemented options.
    #[instrument(skip_all)]
    async fn identify(&self, tx: Sender<ProviderId>) {
        info!("Checking Microsoft Azure");
        if self.check_vendor_file().await || self.check_metadata_server().await {
            info!("Identified Microsoft Azure");
            let res = tx.send(IDENTIFIER).await;

            if let Err(err) = res {
                error!("Error sending message: {:?}", err);
            }
        }
    }
}

impl Azure {
    /// Tries to identify Azure via metadata server.
    #[instrument(skip_all)]
    async fn check_metadata_server(&self) -> bool {
        debug!(
            "Checking {} metadata using url: {}",
            IDENTIFIER, METADATA_URL
        );
        let client = reqwest::Client::new();
        let req = client.get(METADATA_URL).header("Metadata", "true");

        match req.send().await {
            Ok(resp) => match resp.json::<MetadataResponse>().await {
                Ok(resp) => resp.compute.vm_id.len() > 0,
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

    /// Tries to identify Azure using vendor file(s).
    #[instrument(skip_all)]
    async fn check_vendor_file(&self) -> bool {
        debug!("Checking {} vendor file: {}", IDENTIFIER, VENDOR_FILE);
        let vendor_file = Path::new(VENDOR_FILE);

        if vendor_file.is_file() {
            return match fs::read_to_string(vendor_file).await {
                Ok(content) => content.contains("Microsoft Corporation"),
                Err(err) => {
                    error!("Error reading file: {:?}", err);
                    false
                }
            };
        }

        false
    }
}
