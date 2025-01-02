pub(crate) mod providers;

use std::sync::mpsc::Sender;
use std::sync::{mpsc, Arc, LazyLock, Mutex};
use std::time::Duration;

use anyhow::Result;

use crate::blocking::providers::*;
use crate::{ProviderId, DEFAULT_DETECTION_TIMEOUT};

/// Represents a cloud service provider.
#[allow(dead_code)]
pub(crate) trait Provider: Send + Sync {
    fn identifier(&self) -> ProviderId;
    fn identify(&self, tx: Sender<ProviderId>, timeout: Duration);
}

type P = Arc<dyn Provider>;

static PROVIDERS: LazyLock<Mutex<Vec<P>>> = LazyLock::new(|| {
    Mutex::new(vec![
        Arc::new(alibaba::Alibaba) as P,
        Arc::new(aws::Aws) as P,
        Arc::new(azure::Azure) as P,
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

pub fn detect(timeout: Option<u64>) -> Result<ProviderId> {
    let timeout = Duration::from_secs(timeout.unwrap_or(DEFAULT_DETECTION_TIMEOUT));
    let (tx, rx) = mpsc::channel::<ProviderId>();
    let guard = PROVIDERS
        .lock()
        .map_err(|_| anyhow::anyhow!("Error locking providers"))?;
    let provider_entries: Vec<P> = guard.iter().cloned().collect();
    let providers_count = provider_entries.len();
    let mut handles = Vec::with_capacity(providers_count);

    for provider in provider_entries {
        let tx = tx.clone();
        let handle = std::thread::spawn(move || provider.identify(tx, timeout));

        handles.push(handle);
    }

    if let Ok(provider_id) = rx.recv_timeout(timeout) {
        return Ok(provider_id);
    }

    Ok(ProviderId::Unknown)
}
