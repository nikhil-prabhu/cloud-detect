//! DigitalOcean.

use std::path::Path;

use async_trait::async_trait;
use serde::Deserialize;
use tokio::fs;
use tokio::sync::mpsc::Sender;
use tracing::{debug, error, info, instrument};

use crate::{Provider, ProviderId};

const METADATA_URL: &str = "http://169.254.169.254/metadata/v1.json";
const VENDOR_FILE: &str = "/sys/class/dmi/id/sys_vendor";
pub const IDENTIFIER: ProviderId = ProviderId::DigitalOcean;

pub struct DigitalOcean;

#[derive(Deserialize)]
struct MetadataResponse {
    droplet_id: usize,
}

#[async_trait]
impl Provider for DigitalOcean {
    fn identifier(&self) -> ProviderId {
        IDENTIFIER
    }

    /// Tries to identify DigitalOcean using all the implemented options.
    #[instrument(skip_all)]
    async fn identify(&self, tx: Sender<ProviderId>) {
        info!("Checking DigitalOcean");
        if self.check_vendor_file().await || self.check_metadata_server().await {
            info!("Identified DigitalOcean");
            let res = tx.send(IDENTIFIER).await;

            if let Err(err) = res {
                error!("Error sending message: {:?}", err);
            }
        }
    }
}

impl DigitalOcean {
    /// Tries to identify DigitalOcean via metadata server.
    #[instrument(skip_all)]
    async fn check_metadata_server(&self) -> bool {
        debug!(
            "Checking {} metadata using url: {}",
            IDENTIFIER, METADATA_URL
        );
        match reqwest::get(METADATA_URL).await {
            Ok(resp) => {
                return match resp.json::<MetadataResponse>().await {
                    Ok(resp) => resp.droplet_id > 0,
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
        }
    }

    /// Tries to identify DigitalOcean using vendor file(s).
    #[instrument(skip_all)]
    async fn check_vendor_file(&self) -> bool {
        debug!("Checking {} vendor file: {}", IDENTIFIER, VENDOR_FILE);
        let vendor_file = Path::new(VENDOR_FILE);

        if vendor_file.is_file() {
            return match fs::read_to_string(vendor_file).await {
                Ok(content) => content.contains("DigitalOcean"),
                Err(err) => {
                    error!("Error reading file: {:?}", err);
                    false
                }
            };
        }

        false
    }
}
