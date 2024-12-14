//! Microsoft Azure.

use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use async_trait::async_trait;
use serde::Deserialize;
use tracing::{debug, error, Level};

use crate::{register_provider, Provider};

const METADATA_URL: &str = "http://169.254.169.254/metadata/instance?api-version=2017-12-01";
const VENDOR_FILE: &str = "/sys/class/dmi/id/sys_vendor";
pub const IDENTIFIER: &str = "azure";

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

static _REGISTER: LazyLock<()> = LazyLock::new(|| {
    register_provider!(IDENTIFIER, Azure);
});

#[async_trait]
impl Provider for Azure {
    /// Tries to identify Azure using all the implemented options.
    async fn identify(&self) -> bool {
        crate::identify(self, IDENTIFIER).await
    }

    /// Tries to identify Azure via metadata server.
    async fn check_metadata_server(&self) -> bool {
        let span = tracing::span!(Level::TRACE, "check_metadata_server");
        let _enter = span.enter();

        debug!(
            "Checking {} metadata using url: {}",
            IDENTIFIER, METADATA_URL
        );
        let client = reqwest::Client::new();
        let req = client.get(METADATA_URL).header("Metadata", "true");

        return match req.send().await {
            Ok(resp) => {
                return match resp.json::<MetadataResponse>().await {
                    Ok(resp) => resp.compute.vm_id.len() > 0,
                    Err(err) => {
                        error!("Error reading response: {:?}", err);
                        false
                    }
                }
            }
            Err(err) => {
                error!("Error making request: {:?}", err);
                false
            }
        };
    }

    /// Tries to identify Azure using vendor file(s).
    async fn check_vendor_file(&self) -> bool {
        let span = tracing::span!(Level::TRACE, "check_vendor_file");
        let _enter = span.enter();

        debug!("Checking {} vendor file: {}", IDENTIFIER, VENDOR_FILE);
        let vendor_file = Path::new(VENDOR_FILE);

        if vendor_file.is_file() {
            return match fs::read_to_string(vendor_file) {
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
