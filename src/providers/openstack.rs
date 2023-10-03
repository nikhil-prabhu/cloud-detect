use async_trait::async_trait;

use crate::Provider;

const METADATA_URL: &str = "http://169.254.169.254/openstack/";

pub(crate) struct OpenStack;

#[async_trait]
impl Provider for OpenStack {
    fn identifier() -> &'static str {
        "openstack"
    }

    /// Tries to identify OpenStack using all the implemented options.
    async fn identify() -> bool {
        Self::check_vendor_file().await || Self::check_metadata_server().await
    }

    /// Tries to identify OpenStack via metadata server.
    async fn check_metadata_server() -> bool {
        reqwest::get(METADATA_URL).await.is_ok()
    }

    /// Tries to identify OpenStack using vendor file(s).
    async fn check_vendor_file() -> bool {
        // Vendor file checking is currently not implemented (because I have no clue how to do so).
        false
    }
}
