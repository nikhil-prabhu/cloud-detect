//! Detect a host's cloud service provider.

use std::collections::HashMap;
use std::time::Duration;
use std::sync::LazyLock;

use async_trait::async_trait;
use tokio::sync::mpsc;
use tokio::time::timeout as tokio_timeout;
use tracing::{debug, info, Level};

use crate::providers::{alibaba, aws, azure, digitalocean, gcp, oci, openstack, vultr};

pub mod providers;

const UNKNOWN_PROVIDER: &str = "unknown";
const DETECTION_TIMEOUT: u64 = 5; // seconds

/// Represents a cloud service provider.
#[async_trait]
pub trait Provider: Send + Sync {
    async fn identify(&self) -> bool;
    async fn check_metadata_server(&self) -> bool;
    async fn check_vendor_file(&self) -> bool;
}

macro_rules! count_exprs {
    () => { 0 };
    ($head:expr) => { 1 };
    ($head:expr, $($tail:expr),*) => { 1 + count_exprs!($($tail),*) };
}

macro_rules! register_providers {
    // This macro takes in a list of tuples: (String, Provider)
    ($($name:expr => $provider:expr),*) => {
        // Create a HashMap to hold the provider mappings
        pub static PROVIDERS: LazyLock<HashMap<&'static str, Box<dyn Provider>>> = LazyLock::new(|| {
            let mut map = HashMap::new();
            $(
                map.insert($name, Box::new($provider) as Box<dyn Provider>);
            )*
            map
        });

        // Populate the list of supported providers (just the keys of the map)
        /// List of supported cloud providers.
        pub const SUPPORTED_PROVIDERS: [&'static str; count_exprs!($($name),*)] = [$($name),*];
    };
}

register_providers!(
    aws::IDENTIFIER => aws::AWS,
    azure::IDENTIFIER => azure::Azure,
    gcp::IDENTIFIER => gcp::GCP,
    alibaba::IDENTIFIER => alibaba::Alibaba,
    digitalocean::IDENTIFIER => digitalocean::DigitalOcean,
    oci::IDENTIFIER => oci::OCI,
    openstack::IDENTIFIER => openstack::OpenStack,
    vultr::IDENTIFIER => vultr::Vultr
);

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
/// a value from [`const@SUPPORTED_PROVIDERS`].
///
/// # Arguments
///
/// * `timeout` - Maximum time(seconds) allowed for detection. Defaults to 5 if `None`.
pub async fn detect(timeout: Option<u64>) -> &'static str {
    let span = tracing::span!(Level::TRACE, "detect");
    let _enter = span.enter();

    type P = Box<dyn Provider + Send + Sync>;

    let timeout = Duration::from_secs(timeout.unwrap_or(DETECTION_TIMEOUT));
    let (tx, mut rx) = mpsc::channel::<&str>(1);

    for (id, provider) in PROVIDERS.iter() {
        let tx = tx.clone();

        debug!("Attempting to identify {}", id);
        tokio::spawn(async move {
            if provider.identify().await {
                if let Err(err) = tx.send(&id).await {
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
