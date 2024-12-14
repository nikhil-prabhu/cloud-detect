use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::time::timeout as tokio_timeout;
use tracing::{debug, Level};

pub mod providers;

const UNKNOWN_PROVIDER: &str = "unknown";
const DETECTION_TIMEOUT: u64 = 5; // seconds

/// Represents a cloud service provider.
#[async_trait]
pub trait Provider: Send + Sync {
    async fn identify(&self) -> bool;
}

static PROVIDERS: LazyLock<Mutex<HashMap<&'static str, Arc<dyn Provider + Send + Sync>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Registers a provider with the global provider map.
///
/// # Arguments
///
/// * `id` - The identifier string for the provider.
/// * `provider` - The concrete provider object.
#[macro_export]
macro_rules! register_provider {
    ($id:expr, $provider:expr) => {{
        use std::sync::Arc as _Arc;

        use crate::PROVIDERS as _PROVIDERS;

        let mut providers = _PROVIDERS.lock().unwrap();
        providers.insert($id, _Arc::new($provider));
    }};
}

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
