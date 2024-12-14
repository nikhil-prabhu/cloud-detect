//! Alibaba Cloud.

use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use async_trait::async_trait;
use tracing::{debug, error, info, Level};

use crate::{register_provider, Provider};

const METADATA_URL: &str =
    "http://100.100.100.200/latest/meta-data/latest/meta-data/instance/virtualization-solution";
const VENDOR_FILE: &str = "/sys/class/dmi/id/product_name";
pub const IDENTIFIER: &str = "alibaba";

pub struct Alibaba;

static _REGISTER: LazyLock<()> = LazyLock::new(|| {
    register_provider!(IDENTIFIER, Alibaba);
});

#[async_trait]
impl Provider for Alibaba {
    /// Tries to identify Alibaba using all the implemented options.
    async fn identify(&self) -> bool {
        info!("Checking Alibaba Cloud");
        self.check_vendor_file().await || self.check_metadata_server().await
    }
}

impl Alibaba {
    /// Tries to identify Alibaba via metadata server.
    async fn check_metadata_server(&self) -> bool {
        let span = tracing::span!(Level::TRACE, "check_metadata_server");
        let _enter = span.enter();

        debug!(
            "Checking {} metadata using url: {}",
            IDENTIFIER, METADATA_URL
        );
        match reqwest::get(METADATA_URL).await {
            Ok(resp) => match resp.text().await {
                Ok(text) => text.contains("ECS Virt"),
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

    /// Tries to identify Alibaba using vendor file(s).
    async fn check_vendor_file(&self) -> bool {
        let span = tracing::span!(Level::TRACE, "check_vendor_file");
        let _enter = span.enter();

        debug!("Checking {} vendor file: {}", IDENTIFIER, VENDOR_FILE);
        let vendor_file = Path::new(VENDOR_FILE);

        if vendor_file.is_file() {
            return match fs::read_to_string(vendor_file) {
                Ok(content) => content.contains("Alibaba Cloud ECS"),
                Err(err) => {
                    error!("Error reading file: {:?}", err);
                    false
                }
            };
        }

        false
    }
}
