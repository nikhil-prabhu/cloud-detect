use std::fs;
use std::path::Path;

use async_trait::async_trait;

use crate::Provider;

const METADATA_URL: &str =
    "http://100.100.100.200/latest/meta-data/latest/meta-data/instance/virtualization-solution";
const VENDOR_FILE: &str = "/sys/class/dmi/id/product_name";
pub const IDENTIFIER: &str = "alibaba";

pub(crate) struct Alibaba;

#[async_trait]
impl Provider for Alibaba {
    /// Tries to identify Alibaba using all the implemented options.
    async fn identify(&self) -> bool {
        self.check_vendor_file().await || self.check_metadata_server().await
    }

    /// Tries to identify Alibaba via metadata server.
    async fn check_metadata_server(&self) -> bool {
        return match reqwest::get(METADATA_URL).await {
            Ok(resp) => {
                return match resp.text().await {
                    Ok(text) => text.contains("ECS Virt"),
                    Err(_) => false,
                };
            }
            Err(_) => false,
        };
    }

    /// Tries to identify Alibaba using vendor file(s).
    async fn check_vendor_file(&self) -> bool {
        let vendor_file = Path::new(VENDOR_FILE);

        if vendor_file.is_file() {
            return match fs::read_to_string(vendor_file) {
                Ok(content) => content.contains("Alibaba Cloud ECS"),
                Err(_) => false,
            };
        }

        false
    }
}
