//! OpenStack.

use async_trait::async_trait;
use tracing::{debug, error, Level};

use crate::Provider;

const METADATA_URL: &str = "http://169.254.169.254/openstack/";
pub const IDENTIFIER: &str = "openstack";

pub struct OpenStack;

#[async_trait]
impl Provider for OpenStack {
    /// Tries to identify OpenStack using all the implemented options.
    async fn identify(&self) -> bool {
        crate::identify(self, IDENTIFIER).await
    }

    /// Tries to identify OpenStack via metadata server.
    async fn check_metadata_server(&self) -> bool {
        let span = tracing::span!(Level::TRACE, "check_metadata_server");
        let _enter = span.enter();

        debug!(
            "Checking {} metadata using url: {}",
            IDENTIFIER, METADATA_URL
        );
        return match reqwest::get(METADATA_URL).await {
            Ok(resp) => resp.status().is_success(),
            Err(err) => {
                error!("Error making request: {:?}", err);
                false
            }
        };
    }

    /// Tries to identify OpenStack using vendor file(s).
    async fn check_vendor_file(&self) -> bool {
        let span = tracing::span!(Level::TRACE, "check_vendor_file");
        let _enter = span.enter();

        // Vendor file checking is currently not implemented (because I have no clue how to do so).
        debug!("Vendor file checking currently unimplemented");
        false
    }
}
