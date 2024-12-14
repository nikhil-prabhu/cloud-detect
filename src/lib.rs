use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::sync::mpsc::Sender;
use tokio::time::timeout as tokio_timeout;
use tracing::{debug, Level};

use crate::providers::*;

pub mod providers;

const UNKNOWN_PROVIDER: &str = "unknown";
const DETECTION_TIMEOUT: u64 = 5; // seconds

/// Represents a cloud service provider.
#[async_trait]
pub trait Provider: Send + Sync {
    fn identifier(&self) -> &'static str;
    async fn identify(&self, tx: Sender<&'static str>);
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

/// Returns a list of supported providers.
pub fn supported_providers() -> Vec<&'static str> {
    let guard = PROVIDERS.lock().unwrap();
    let providers: Vec<&'static str> = guard.iter().map(|p| p.identifier()).collect();

    drop(guard);

    providers
}

/// Detects the host's cloud provider.
///
/// Returns "unknown" if the detection failed or timed out. If the detection was successful, it returns
/// a value from [`supported_providers`](fn.supported_providers.html).
///
/// # Arguments
///
/// * `timeout` - Maximum time(seconds) allowed for detection. Defaults to 5 if `None`.
pub async fn detect(timeout: Option<u64>) -> &'static str {
    let span = tracing::span!(Level::TRACE, "detect");
    let _enter = span.enter();

    let timeout = Duration::from_secs(timeout.unwrap_or(DETECTION_TIMEOUT));
    let (tx, mut rx) = mpsc::channel::<&str>(1);

    let guard = PROVIDERS.lock().unwrap();

    // Collect the Arc<dyn Provider> values
    let provider_entries: Vec<P> = guard
        .iter()
        .map(|p| p.clone()) // Clone the Arc
        .collect();

    drop(guard); // Explicitly drop the lock

    for provider in provider_entries {
        let tx = tx.clone();

        debug!("Spawning task for provider: {}", provider.identifier());
        tokio::spawn(async move {
            provider.identify(tx).await;
        });
    }

    match tokio_timeout(timeout, rx.recv()).await {
        Ok(Some(provider)) => provider,
        _ => UNKNOWN_PROVIDER,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_supported_providers() {
        let providers = supported_providers();
        assert_eq!(providers.len(), 8);
        assert!(providers.contains(&alibaba::IDENTIFIER));
        assert!(providers.contains(&aws::IDENTIFIER));
        assert!(providers.contains(&azure::IDENTIFIER));
        assert!(providers.contains(&digitalocean::IDENTIFIER));
        assert!(providers.contains(&gcp::IDENTIFIER));
        assert!(providers.contains(&oci::IDENTIFIER));
        assert!(providers.contains(&openstack::IDENTIFIER));
        assert!(providers.contains(&vultr::IDENTIFIER));
    }

    // FIXME: This test will fail on actual cloud instances.
    #[tokio::test]
    async fn test_detect() {
        let provider = detect(None).await;
        assert_eq!(provider, UNKNOWN_PROVIDER);
    }
}
