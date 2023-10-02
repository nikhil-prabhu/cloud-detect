///! Detect a host's cloud service provider.

/// Represents a cloud service provider.
pub(crate) trait Provider {
    fn identifier() -> &'static str;
    async fn identify() -> bool;
    async fn check_metadata_server() -> bool;
    async fn check_vendor_file() -> bool;
}
