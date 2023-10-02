///! Detect a host's cloud service provider.

use crate::providers::aws::AWS;

mod providers;

/// Represents a cloud service provider.
pub(crate) trait Provider {
    fn identifier() -> &'static str;
    async fn identify() -> bool;
    async fn check_metadata_server() -> bool;
    async fn check_vendor_file() -> bool;
}

const SUPPORTED_PROVIDERS: [&str; 1] = [
    AWS::identifier(),
];
