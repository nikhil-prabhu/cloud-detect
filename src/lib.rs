use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::time::timeout as tokio_timeout;
use tracing::{debug, Level};

use crate::providers::*;

pub mod providers;

const UNKNOWN_PROVIDER: &str = "unknown";
const DETECTION_TIMEOUT: u64 = 5; // seconds

/// Represents a cloud service provider.
#[async_trait]
pub trait Provider: Send + Sync {
    async fn identify(&self) -> bool;
}

type P = Arc<dyn Provider + Send + Sync>;

static PROVIDERS: LazyLock<Mutex<HashMap<&'static str, P>>> = LazyLock::new(|| {
    Mutex::new(HashMap::from([
        (alibaba::IDENTIFIER, Arc::new(alibaba::Alibaba) as P),
        (aws::IDENTIFIER, Arc::new(aws::AWS) as P),
        (azure::IDENTIFIER, Arc::new(azure::Azure) as P),
        (
            digitalocean::IDENTIFIER,
            Arc::new(digitalocean::DigitalOcean) as P,
        ),
        (gcp::IDENTIFIER, Arc::new(gcp::GCP) as P),
        (oci::IDENTIFIER, Arc::new(oci::OCI) as P),
        (openstack::IDENTIFIER, Arc::new(openstack::OpenStack) as P),
        (vultr::IDENTIFIER, Arc::new(vultr::Vultr) as P),
    ]))
});

/// Returns a list of supported providers.
pub fn supported_providers() -> Vec<&'static str> {
    let guard = PROVIDERS.lock().unwrap();
    let keys: Vec<&'static str> = guard.keys().map(|k| *k).collect();
    drop(guard);

    keys
}

/// Detects the host's cloud provider.
///
/// Returns "unknown" if the detection failed or timed out. If the detection was successful, it returns
/// a value from [`const@SUPPORTED_PROVIDERS`].
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
    let provider_entries: Vec<(&str, Arc<dyn Provider + Send + Sync>)> = guard
        .iter()
        .map(|(k, v)| (*k, v.clone())) // Clone the Arc
        .collect();

    drop(guard); // Explicitly drop the lock

    for (id, provider) in provider_entries {
        let tx = tx.clone();

        debug!("Attempting to identify {}", id);
        tokio::spawn(async move {
            if provider.identify().await {
                if let Err(err) = tx.send(id).await {
                    debug!("Got error for provider {}: {:?}", id, err);
                }
            }
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
