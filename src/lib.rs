///! Detect a host's cloud service provider.

use std::collections::HashMap;
use std::time::Duration;

use async_trait::async_trait;
use lazy_static::lazy_static;
use tokio::sync::mpsc;
use tokio::time::timeout as tokio_timeout;

use crate::providers::alibaba::Alibaba;
use crate::providers::aws::AWS;
use crate::providers::azure::Azure;
use crate::providers::gcp::GCP;
use crate::providers::openstack::OpenStack;

mod providers;

const UNKNOWN_PROVIDER: &str = "unknown";

/// Represents a cloud service provider.
#[async_trait]
pub(crate) trait Provider {
    fn identifier(&self) -> &'static str;
    async fn identify(&self) -> bool;
    async fn check_metadata_server(&self) -> bool;
    async fn check_vendor_file(&self) -> bool;
}

lazy_static! {
    /// A list of the currently supported cloud providers.
    pub static ref SUPPORTED_PROVIDERS: [&'static str; 5] = [
        crate::providers::aws::AWS.identifier(),
        crate::providers::azure::Azure.identifier(),
        crate::providers::gcp::GCP.identifier(),
        crate::providers::alibaba::Alibaba.identifier(),
        crate::providers::openstack::OpenStack.identifier(),
    ];
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
        (AWS.identifier(), Box::new(AWS) as P),
        (Azure.identifier(), Box::new(Azure) as P),
        (GCP.identifier(), Box::new(GCP) as P),
        (Alibaba.identifier(), Box::new(Alibaba) as P),
        (OpenStack.identifier(), Box::new(OpenStack) as P),
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
