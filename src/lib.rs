///! Detect a host's cloud service provider.
use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::time::timeout as tokio_timeout;
use tracing::{info, Level};

use crate::providers::{alibaba, aws, azure, gcp, openstack};

mod providers;

const UNKNOWN_PROVIDER: &str = "unknown";

/// Represents a cloud service provider.
#[async_trait]
pub(crate) trait Provider {
    async fn identify(&self) -> bool;
    async fn check_metadata_server(&self) -> bool;
    async fn check_vendor_file(&self) -> bool;
}

/// The list of currently supported providers.
const SUPPORTED_PROVIDERS: [&str; 5] = [
    aws::IDENTIFIER,
    azure::IDENTIFIER,
    gcp::IDENTIFIER,
    alibaba::IDENTIFIER,
    openstack::IDENTIFIER,
];

/// Convenience function that identifies a [`Provider`] using the [`Provider::check_metadata_server`]
/// and [`Provider::check_vendor_file`] methods.
///
/// This function just serves to reduce code duplication across the crate.
///
/// # Arguments
///
/// * `provider` - The concrete provider object.
/// * `identifier` - The identifier string for the provider.
pub(crate) async fn identify<P: Provider>(provider: &P, identifier: &str) -> bool {
    let span = tracing::span!(Level::TRACE, "identify");
    let _enter = span.enter();

    info!("Attempting to identify {}", identifier);
    provider.check_vendor_file().await || provider.check_metadata_server().await
}

/// Detects the host's cloud provider.
///
/// Returns "unknown" if the detection failed or timed out. If the detection was successful, it returns
/// a value from [`struct@SUPPORTED_PROVIDERS`].
///
/// # Arguments
///
/// * `timeout` - Maximum time(seconds) allowed for detection. Defaults to 5 if `None`.
pub async fn detect(timeout: Option<u64>) -> &'static str {
    type P = Box<dyn Provider + Send + Sync>;

    let timeout = Duration::from_secs(timeout.unwrap_or(5));
    let (tx, mut rx) = mpsc::channel::<&str>(1);
    let mut identifiers: HashMap<&str, P> = HashMap::from([
        (aws::IDENTIFIER, Box::new(aws::AWS) as P),
        (azure::IDENTIFIER, Box::new(azure::Azure) as P),
        (gcp::IDENTIFIER, Box::new(gcp::GCP) as P),
        (alibaba::IDENTIFIER, Box::new(alibaba::Alibaba) as P),
        (openstack::IDENTIFIER, Box::new(openstack::OpenStack) as P),
    ]);

    for provider in SUPPORTED_PROVIDERS.iter() {
        let tx = tx.clone();
        let identifier = identifiers.remove(provider).unwrap();

        tokio::spawn(async move {
            if identifier.identify().await {
                tx.send(&provider).await.unwrap();
            }
        });
    }

    match tokio_timeout(timeout, rx.recv()).await {
        Ok(Some(provider)) => provider,
        _ => UNKNOWN_PROVIDER,
    }
}
