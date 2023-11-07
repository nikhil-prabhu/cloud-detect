//! Oracle Cloud Infrastructure (OCI).

use std::fs;
use std::path::Path;

use async_trait::async_trait;
use serde::Deserialize;
use tracing::{debug, error, Level};

use crate::Provider;

const VENDOR_FILE: &str = "/sys/class/dmi/id/chassis_asset_tag";
pub const IDENTIFIER: &str = "oci";

pub struct OCI;

#[async_trait]
impl Provider for OCI {
    /// Tries to identify OCI using all the implemented options.
    async fn identify(&self) -> bool {
        crate::identify(self, IDENTIFIER).await
    }

    /// Tries to identify OCI via metadata server.
    async fn check_metadata_server(&self) -> bool {
        let span = tracing::span!(Level::TRACE, "check_metadata_server");
        let _enter = span.enter();

        // Vendor file checking is currently not implemented.
        debug!("Metadata server checking currently unimplemented");
        false
    }

    /// Tries to identify OCI using vendor file(s).
    async fn check_vendor_file(&self) -> bool {
        let span = tracing::span!(Level::TRACE, "check_vendor_file");
        let _enter = span.enter();

        debug!("Checking {} vendor file: {}", IDENTIFIER, VENDOR_FILE);
        let vendor_file = Path::new(VENDOR_FILE);

        if vendor_file.is_file() {
            return match fs::read_to_string(vendor_file) {
                Ok(content) => content.contains("OracleCloud"),
                Err(err) => {
                    error!("Error reading file: {:?}", err);
                    false
                }
            };
        }

        false
    }
}
