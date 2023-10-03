///! Detect a host's cloud service provider.
use async_trait::async_trait;
use lazy_static::lazy_static;

mod providers;

/// Represents a cloud service provider.
#[async_trait]
pub(crate) trait Provider {
    fn identifier() -> &'static str;
    async fn identify() -> bool;
    async fn check_metadata_server() -> bool;
    async fn check_vendor_file() -> bool;
}

lazy_static! {
    /// A list of the currently supported cloud providers.
    pub static ref SUPPORTED_PROVIDERS: [&'static str; 4] = [
        crate::providers::aws::AWS::identifier(),
        crate::providers::azure::Azure::identifier(),
        crate::providers::gcp::GCP::identifier(),
        crate::providers::alibaba::Alibaba::identifier(),
    ];
}
