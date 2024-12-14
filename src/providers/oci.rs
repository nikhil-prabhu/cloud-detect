//! Oracle Cloud Infrastructure (OCI).

use std::fs;
use std::path::Path;
use std::sync::LazyLock;

use async_trait::async_trait;
use tracing::{debug, error, info, Level};

use crate::{register_provider, Provider};

const VENDOR_FILE: &str = "/sys/class/dmi/id/chassis_asset_tag";
pub const IDENTIFIER: &str = "oci";

pub struct OCI;

static _REGISTER: LazyLock<()> = LazyLock::new(|| {
    register_provider!(IDENTIFIER, OCI);
});

#[async_trait]
impl Provider for OCI {
    /// Tries to identify OCI using all the implemented options.
    async fn identify(&self) -> bool {
        info!("Checking Oracle Cloud Infrastructure");
        self.check_vendor_file().await || self.check_metadata_server().await
    }
}

impl OCI {
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
