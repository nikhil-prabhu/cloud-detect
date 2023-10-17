use async_trait::async_trait;

use crate::Provider;

const METADATA_URL: &str = "http://169.254.169.254/openstack/";
pub const IDENTIFIER: &str = "openstack";

pub(crate) struct OpenStack;

#[async_trait]
impl Provider for OpenStack {
    /// Tries to identify OpenStack using all the implemented options.
    async fn identify(&self) -> bool {
        self.check_vendor_file().await || self.check_metadata_server().await
    }

    /// Tries to identify OpenStack via metadata server.
    async fn check_metadata_server(&self) -> bool {
        reqwest::get(METADATA_URL).await.is_ok()
    }

    /// Tries to identify OpenStack using vendor file(s).
    async fn check_vendor_file(&self) -> bool {
        // Vendor file checking is currently not implemented (because I have no clue how to do so).
        false
    }
}
