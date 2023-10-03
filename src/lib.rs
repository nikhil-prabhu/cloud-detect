///! Detect a host's cloud service provider.
use async_trait::async_trait;

use crate::providers::aws::AWS;
use crate::providers::azure::Azure;
use crate::providers::gcp::GCP;

mod providers;

/// Represents a cloud service provider.
#[async_trait]
pub(crate) trait Provider {
    fn identifier() -> &'static str;
    async fn identify() -> bool;
    async fn check_metadata_server() -> bool;
    async fn check_vendor_file() -> bool;
}

/// A list of the currently supported cloud providers.
pub static SUPPORTED_PROVIDERS: [&str; 3] = [
    AWS::identifier(),
    Azure::identifier(),
    GCP::identifier(),
];
