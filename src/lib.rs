//! # Cloud Detect
//!
//! A library to detect the cloud service provider of a host.
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! [dependencies]
//! # ...
//! cloud_detect = "1.1.0"
//! tokio = { version = "1", features = ["full"] }
//! tracing-subscriber = { version = "0.2", features = ["env-filter"] } # Optional; for logging
//! ```
//!
//! ## Examples
//!
//! Detect the cloud provider and print the result (with default timeout).
//!
//! ```rust
//! use cloud_detect::detect;
//!
//! #[tokio::main]
//! async fn main() {
//!     tracing_subscriber::fmt::init(); // Optional; for logging
//!
//!     let provider = detect(None).await;
//!     println!("Detected provider: {}", provider);
//! }
//! ```
//!
//! Detect the cloud provider and print the result (with custom timeout).
//!
//! ```rust
//! use cloud_detect::detect;
//!
//! #[tokio::main]
//! async fn main() {
//!     tracing_subscriber::fmt::init(); // Optional; for logging
//!
//!     let provider = detect(Some(10)).await;
//!     println!("Detected provider: {}", provider);
//! }
//! ```

use std::fmt::Debug;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::Duration;

use async_trait::async_trait;
use strum::Display;
use tokio::sync::mpsc::Sender;
use tokio::sync::{mpsc, Mutex, Notify};
use tracing::{debug, instrument};

use crate::providers::*;

pub(crate) mod providers;

/// Maximum time allowed for detection.
pub const DEFAULT_DETECTION_TIMEOUT: u64 = 5; // seconds

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
pub(crate) trait Provider: Send + Sync {
    fn identifier(&self) -> ProviderId;
    async fn identify(&self, tx: Sender<ProviderId>);
}

type P = Arc<dyn Provider>;

static PROVIDERS: LazyLock<Mutex<Vec<P>>> = LazyLock::new(|| {
    Mutex::new(vec![
        Arc::new(alibaba::Alibaba) as P,
        Arc::new(aws::Aws) as P,
        Arc::new(azure::Azure) as P,
        Arc::new(digitalocean::DigitalOcean) as P,
        Arc::new(gcp::Gcp) as P,
        Arc::new(oci::Oci) as P,
        Arc::new(openstack::OpenStack) as P,
        Arc::new(vultr::Vultr) as P,
    ])
});

/// Returns a list of currently supported providers.
///
/// # Examples
///
/// Print the list of supported providers.
///
/// ```
/// use cloud_detect::supported_providers;
///
/// #[tokio::main]
/// async fn main() {
///     let providers = supported_providers().await;
///     println!("Supported providers: {:?}", providers);
/// }
/// ```
pub async fn supported_providers() -> Vec<String> {
    let guard = PROVIDERS.lock().await;
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
/// * `timeout` - Maximum time (seconds) allowed for detection. Defaults to [DEFAULT_DETECTION_TIMEOUT](constant.DEFAULT_DETECTION_TIMEOUT.html) if `None`.
///
/// # Examples
///
/// Detect the cloud provider and print the result (with default timeout).
///
/// ```
/// use cloud_detect::detect;
///
/// #[tokio::main]
/// async fn main() {
///     let provider = detect(None).await;
///     println!("Detected provider: {}", provider);
/// }
/// ```
///
/// Detect the cloud provider and print the result (with custom timeout).
///
/// ```
/// use cloud_detect::detect;
///
/// #[tokio::main]
/// async fn main() {
///     let provider = detect(Some(10)).await;
///     println!("Detected provider: {}", provider);
/// }
/// ```
#[instrument]
pub async fn detect(timeout: Option<u64>) -> ProviderId {
    let timeout = Duration::from_secs(timeout.unwrap_or(DEFAULT_DETECTION_TIMEOUT));
    let (tx, mut rx) = mpsc::channel::<ProviderId>(1);
    let guard = PROVIDERS.lock().await;
    let provider_entries: Vec<P> = guard.iter().cloned().collect();
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
        let providers = supported_providers().await;
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
        let provider = detect(Some(1)).await;
        assert_eq!(provider, Default::default());
    }
}
