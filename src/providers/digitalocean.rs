//! DigitalOcean.

use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use async_trait::async_trait;
use serde::Deserialize;
use tracing::{debug, error, info, Level};

use crate::{register_provider, Provider};

const METADATA_URL: &str = "http://169.254.169.254/metadata/v1.json";
const VENDOR_FILE: &str = "/sys/class/dmi/id/sys_vendor";
pub const IDENTIFIER: &str = "digitalocean";

pub struct DigitalOcean;

static _REGISTER: LazyLock<()> = LazyLock::new(|| {
    register_provider!(IDENTIFIER, DigitalOcean);
});

#[derive(Deserialize)]
struct MetadataResponse {
    droplet_id: usize,
}

#[async_trait]
impl Provider for DigitalOcean {
    /// Tries to identify DigitalOcean using all the implemented options.
    async fn identify(&self) -> bool {
        info!("Checking DigitalOcean");
        self.check_vendor_file().await || self.check_metadata_server().await
    }
}

impl DigitalOcean {
    /// Tries to identify DigitalOcean via metadata server.
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
        };
    }

    /// Tries to identify DigitalOcean using vendor file(s).
    async fn check_vendor_file(&self) -> bool {
        let span = tracing::span!(Level::TRACE, "check_vendor_file");
        let _enter = span.enter();

        debug!("Checking {} vendor file: {}", IDENTIFIER, VENDOR_FILE);
        let vendor_file = Path::new(VENDOR_FILE);

        if vendor_file.is_file() {
            return match fs::read_to_string(vendor_file) {
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
