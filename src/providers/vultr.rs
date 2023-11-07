//! Vultr.

use std::fs;
use std::path::Path;

use async_trait::async_trait;
use serde::Deserialize;
use tracing::{debug, error, Level};

use crate::Provider;

const METADATA_URL: &str = "http://169.254.169.254/v1.json";
const VENDOR_FILE: &str = "/sys/class/dmi/id/sys_vendor";
pub const IDENTIFIER: &str = "vultr";

pub struct Vultr;

#[derive(Deserialize)]
struct MetadataResponse {
    #[serde(rename = "instanceid")]
    instance_id: String,
}

#[async_trait]
impl Provider for Vultr {
    /// Tries to identify Vultr using all the implemented options.
    async fn identify(&self) -> bool {
        crate::identify(self, IDENTIFIER).await
    }

    /// Tries to identify Vultr via metadata server.
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
                    Ok(resp) => resp.instance_id.len() > 0,
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

    /// Tries to identify Vultr using vendor file(s).
    async fn check_vendor_file(&self) -> bool {
        let span = tracing::span!(Level::TRACE, "check_vendor_file");
        let _enter = span.enter();

        debug!("Checking {} vendor file: {}", IDENTIFIER, VENDOR_FILE);
        let vendor_file = Path::new(VENDOR_FILE);

        if vendor_file.is_file() {
            return match fs::read_to_string(vendor_file) {
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
