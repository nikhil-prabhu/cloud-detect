pub(crate) mod providers;

use std::sync::mpsc::Sender;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Duration;

use anyhow::Result;

use crate::blocking::providers::*;
use crate::ProviderId;

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

#[allow(unused_variables)]
pub fn detect(timeout: Option<u64>) -> Result<ProviderId> {
    todo!()
}
