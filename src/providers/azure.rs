use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::Provider;

const METADATA_URL: &str = "http://169.254.169.254/metadata/instance?api-version=2017-12-01";
const VENDOR_FILE: &str = "/sys/class/dmi/id/sys_vendor";

#[derive(Deserialize)]
struct Compute {
    #[serde(rename = "vmId")]
    vm_id: String,
}

#[derive(Deserialize)]
struct MetadataResponse {
    compute: Compute,
}

pub(crate) struct Azure;

impl Provider for Azure {
    fn identifier() -> &'static str {
        "azure"
    }

    /// Tries to identify Azure using all the implemented options.
    async fn identify() -> bool {
        Self::check_vendor_file().await || Self::check_metadata_server().await
    }

    /// Tries to identify Azure via metadata server.
    async fn check_metadata_server() -> bool {
        let client = reqwest::Client::new();
        let req = client.get(METADATA_URL).header("Metadata", "true");

        return match req.send() {
            Ok(resp) => {
                let resp: MetadataResponse = resp.json().await.unwrap();
                resp.compute.vm_id.len() > 0
            }
            Err(_) => false,
        };
    }

    /// Tries to identify Azure using vendor file(s).
    async fn check_vendor_file() -> bool {
        let vendor_file = Path::new(VENDOR_FILE);

        if vendor_file.is_file() {
            return match fs::read_to_string(vendor_file) {
                Ok(content) => content.contains("Microsoft Corporation"),
                Err(_) => false,
            };
        }

        false
    }
}
