//! Blocking API for cloud provider detection.
//!
//! This module provides a blocking API for detecting the host's cloud provider. It is built on top of the asynchronous API
//! and executes the blocking provider identification within threads.
//!
//! This module is intended for use in synchronous applications or in situations where the asynchronous API is not suitable.
//! While not guaranteed, the performance of this module should be comparable to the asynchronous API.
//!
//! ## Optional
//!
//! This requires the `blocking` feature to be enabled.
//!
//! ## Usage
//!
//! Add the following to your `Cargo.toml`:
//!
//! ```toml
//! # ...
//! cloud_detect = { version = "2", features = ["blocking"] }
//! tracing-subscriber = { version = "0.3", features = ["env-filter"] } # Optional; for logging
//! ```
//!
//! ## Examples
//!
//! Detect the cloud provider and print the result (with default timeout).
//!
//! ```rust
//! use cloud_detect::blocking::detect;
//!
//! tracing_subscriber::fmt::init(); // Optional; for logging
//!
//! let provider = detect(None).unwrap();
//! println!("Detected provider: {:?}", provider);
//! ```
//!
//! Detect the cloud provider and print the result (with custom timeout).
//!
//! ```rust
//! use cloud_detect::blocking::detect;
//!
//! tracing_subscriber::fmt::init(); // Optional; for logging
//!
//! let provider = detect(Some(10)).unwrap();
//! println!("Detected provider: {:?}", provider);
//! ```

pub(crate) mod providers;

use std::sync::mpsc::RecvTimeoutError;
use std::sync::mpsc::SyncSender;
use std::sync::{mpsc, Arc, LazyLock, Mutex};
use std::time::Duration;

use anyhow::Result;

use crate::blocking::providers::*;
use crate::{ProviderId, DEFAULT_DETECTION_TIMEOUT};

/// Represents a cloud service provider.
#[allow(dead_code)]
pub(crate) trait Provider: Send + Sync {
    fn identifier(&self) -> ProviderId;
    fn identify(&self, tx: SyncSender<ProviderId>, timeout: Duration);
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
/// use cloud_detect::blocking::supported_providers;
///
/// let providers = supported_providers().unwrap();
/// println!("Supported providers: {:?}", providers);
/// ```
pub fn supported_providers() -> Result<Vec<String>> {
    let guard = PROVIDERS
        .lock()
        .map_err(|_| anyhow::anyhow!("Error locking providers"))?;
    let providers: Vec<String> = guard.iter().map(|p| p.identifier().to_string()).collect();

    drop(guard);

    Ok(providers)
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
/// use cloud_detect::blocking::detect;
///
/// let provider = detect(None).unwrap();
/// println!("Detected provider: {:?}", provider);
/// ```
///
/// Detect the cloud provider and print the result (with custom timeout).
///
/// ```
/// use cloud_detect::blocking::detect;
///
/// let provider = detect(Some(10)).unwrap();
/// println!("Detected provider: {:?}", provider);
/// ```
pub fn detect(timeout: Option<u64>) -> Result<ProviderId> {
    let timeout = Duration::from_secs(timeout.unwrap_or(DEFAULT_DETECTION_TIMEOUT));
    let (tx, rx) = mpsc::sync_channel::<ProviderId>(1);
    let guard = PROVIDERS
        .lock()
        .map_err(|_| anyhow::anyhow!("Error locking providers"))?;
    let provider_entries: Vec<P> = guard.iter().cloned().collect();

    for provider in provider_entries {
        let tx = tx.clone();
        std::thread::spawn(move || provider.identify(tx, timeout));
    }

    match rx.recv_timeout(timeout) {
        Ok(provider_id) => Ok(provider_id),
        Err(err) => match err {
            RecvTimeoutError::Timeout => Ok(ProviderId::Unknown),
            RecvTimeoutError::Disconnected => Err(anyhow::anyhow!("Error receiving message")),
        },
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::*;

    #[test]
    fn test_supported_providers() -> Result<()> {
        let providers = supported_providers()?;
        assert_eq!(providers.len(), 8);
        assert!(providers.contains(&alibaba::IDENTIFIER.to_string()));
        assert!(providers.contains(&aws::IDENTIFIER.to_string()));
        assert!(providers.contains(&azure::IDENTIFIER.to_string()));
        assert!(providers.contains(&digitalocean::IDENTIFIER.to_string()));
        assert!(providers.contains(&gcp::IDENTIFIER.to_string()));
        assert!(providers.contains(&oci::IDENTIFIER.to_string()));
        assert!(providers.contains(&openstack::IDENTIFIER.to_string()));
        assert!(providers.contains(&vultr::IDENTIFIER.to_string()));

        Ok(())
    }
}
