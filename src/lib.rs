use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use strum::Display;
use tokio::sync::mpsc::Sender;
use tokio::sync::{mpsc, Notify};
use tracing::{debug, Level};

use crate::providers::*;

pub mod providers;

pub const DETECTION_TIMEOUT: u64 = 5; // seconds

/// Represents an identifier for a cloud service provider.
#[non_exhaustive]
#[derive(Debug, Default, Display, Eq, PartialEq)]
pub enum ProviderId {
    /// Unknown cloud service provider.
    #[default]
    #[strum(serialize = "unknown")]
    Unknown,
    /// Alibaba Cloud.
    #[strum(serialize = "alibaba")]
    Alibaba,
    /// Amazon Web Services (AWS).
    #[strum(serialize = "aws")]
    AWS,
    /// Microsoft Azure.
    #[strum(serialize = "azure")]
    Azure,
    /// DigitalOcean.
    #[strum(serialize = "digitalocean")]
    DigitalOcean,
    /// Google Cloud Platform (GCP).
    #[strum(serialize = "gcp")]
    GCP,
    /// Oracle Cloud Infrastructure (OCI).
    #[strum(serialize = "oci")]
    OCI,
    /// OpenStack.
    #[strum(serialize = "openstack")]
    OpenStack,
    /// Vultr.
    #[strum(serialize = "vultr")]
    Vultr,
}

/// Represents a cloud service provider.
#[async_trait]
pub trait Provider: Send + Sync {
    fn identifier(&self) -> ProviderId;
    async fn identify(&self, tx: Sender<ProviderId>);
}

type P = Arc<dyn Provider>;

static PROVIDERS: LazyLock<Mutex<Vec<P>>> = LazyLock::new(|| {
    Mutex::new(vec![
        Arc::new(alibaba::Alibaba) as P,
        Arc::new(aws::AWS) as P,
        Arc::new(azure::Azure) as P,
        Arc::new(digitalocean::DigitalOcean) as P,
        Arc::new(gcp::GCP) as P,
        Arc::new(oci::OCI) as P,
        Arc::new(openstack::OpenStack) as P,
        Arc::new(vultr::Vultr) as P,
    ])
});

/// Returns a list of currently supported providers.
pub fn supported_providers() -> Vec<String> {
    let guard = PROVIDERS.lock().unwrap();
    let providers: Vec<String> = guard.iter().map(|p| p.identifier().to_string()).collect();

    drop(guard);

    providers
}

/// Detects the host's cloud provider.
///
/// Returns [ProviderId::Unknown] if the detection failed or timed out. If the detection was successful, it returns
/// a value from [ProviderId](enum.ProviderId.html).
///
/// # Arguments
///
/// * `timeout` - Maximum time(seconds) allowed for detection. Defaults to [DETECTION_TIMEOUT](constant.DETECTION_TIMEOUT.html) if `None`.
pub async fn detect(timeout: Option<u64>) -> ProviderId {
    let span = tracing::span!(Level::TRACE, "detect");
    let _enter = span.enter();
    let timeout = Duration::from_secs(timeout.unwrap_or(DETECTION_TIMEOUT));
    let (tx, mut rx) = mpsc::channel::<ProviderId>(1);

    let guard = PROVIDERS.lock().unwrap();
    let provider_entries: Vec<P> = guard.iter().map(|p| p.clone()).collect();
    drop(guard);

    let providers_count = provider_entries.len();
    let mut handles = Vec::with_capacity(providers_count);

    // Create a counter that will be decremented as tasks complete
    let counter = Arc::new(AtomicUsize::new(providers_count));
    let complete = Arc::new(Notify::new());

    for provider in provider_entries {
        let tx = tx.clone();
        let counter = counter.clone();
        let complete = complete.clone();

        handles.push(tokio::spawn(async move {
            debug!("Spawning task for provider: {}", provider.identifier());
            provider.identify(tx).await;

            // Decrement counter and notify if we're the last task
            if counter.fetch_sub(1, Ordering::SeqCst) == 1 {
                complete.notify_one();
            }
        }));
    }

    tokio::select! {
        biased;

        // Priority 1: If we receive an identifier, return it immediately
        res = rx.recv() => {
            debug!("Received result from channel: {:?}", res);
            res.unwrap_or_default()
        }

        // Priority 2: If all tasks complete without finding an identifier
        _ = complete.notified() => {
            debug!("All providers have finished identifying");
            Default::default()
        }

        // Priority 3: If we time out
        _ = tokio::time::sleep(timeout) => {
            debug!("Detection timed out");
            Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_supported_providers() {
        let providers = supported_providers();
        assert_eq!(providers.len(), 8);
        assert!(providers.contains(&alibaba::IDENTIFIER.to_string()));
        assert!(providers.contains(&aws::IDENTIFIER.to_string()));
        assert!(providers.contains(&azure::IDENTIFIER.to_string()));
        assert!(providers.contains(&digitalocean::IDENTIFIER.to_string()));
        assert!(providers.contains(&gcp::IDENTIFIER.to_string()));
        assert!(providers.contains(&oci::IDENTIFIER.to_string()));
        assert!(providers.contains(&openstack::IDENTIFIER.to_string()));
        assert!(providers.contains(&vultr::IDENTIFIER.to_string()));
    }

    // FIXME: This test will fail on actual cloud instances.
    #[tokio::test]
    async fn test_detect() {
        let provider = detect(None).await;
        assert_eq!(provider, Default::default());
    }
}
