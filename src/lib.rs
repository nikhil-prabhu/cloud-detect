///! Detect a host's cloud service provider.

use crate::providers::aws::AWS;
use crate::providers::azure::Azure;

mod providers;

/// Represents a cloud service provider.
pub(crate) trait Provider {
    fn identifier() -> &'static str;
    async fn identify() -> bool;
    async fn check_metadata_server() -> bool;
    async fn check_vendor_file() -> bool;
}

/// A list of the currently supported cloud providers.
pub const SUPPORTED_PROVIDERS: [&str; 2] = [
    AWS::identifier(),
    Azure::identifier(),
];
