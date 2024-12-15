//! Vultr.

use std::path::Path;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::fs;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, info, instrument};

use crate::{Provider, ProviderId};

const METADATA_URL: &str = "http://169.254.169.254/v1.json";
const VENDOR_FILE: &str = "/sys/class/dmi/id/sys_vendor";
pub const IDENTIFIER: ProviderId = ProviderId::Vultr;

pub struct Vultr;

#[derive(Deserialize)]
struct MetadataResponse {
    #[serde(rename = "instanceid")]
    instance_id: String,
}

#[async_trait]
impl Provider for Vultr {
    fn identifier(&self) -> ProviderId {
        IDENTIFIER
    }

    /// Tries to identify Vultr using all the implemented options.
    #[instrument(skip_all)]
    async fn identify(&self, tx: Sender<ProviderId>) {
        info!("Checking Vultr");
        if self.check_vendor_file().await || self.check_metadata_server().await {
            info!("Identified Vultr");
            let res = tx.send(IDENTIFIER).await;

            if let Err(err) = res {
                error!("Error sending message: {:?}", err);
            }
        }
    }
}

impl Vultr {
    /// Tries to identify Vultr via metadata server.
    #[instrument(skip_all)]
    async fn check_metadata_server(&self) -> bool {
        debug!(
            "Checking {} metadata using url: {}",
            IDENTIFIER, METADATA_URL
        );
        match reqwest::get(METADATA_URL).await {
            Ok(resp) => match resp.json::<MetadataResponse>().await {
                Ok(resp) => resp.instance_id.len() > 0,
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

    /// Tries to identify Vultr using vendor file(s).
    #[instrument(skip_all)]
    async fn check_vendor_file(&self) -> bool {
        debug!("Checking {} vendor file: {}", IDENTIFIER, VENDOR_FILE);
        let vendor_file = Path::new(VENDOR_FILE);

        if vendor_file.is_file() {
            return match fs::read_to_string(vendor_file).await {
                Ok(content) => content.contains("Vultr"),
                Err(err) => {
                    error!("Error reading file: {:?}", err);
                    false
                }
            };
        }

        false
    }
}
