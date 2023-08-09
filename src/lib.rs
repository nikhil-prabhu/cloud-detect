///! Detect a host's cloud service provider.
mod consts;

#[cfg(feature = "blocking")]
pub mod blocking;

use std::collections::HashMap;
use std::error::Error;
use std::sync::mpsc::{self, TryRecvError};
use std::time::{Duration, Instant};

use lazy_static::lazy_static;

use consts::*;

lazy_static! {
    /// A mapping of supported cloud providers with their metadata URLs.
    pub(crate) static ref PROVIDER_METADATA_MAP: HashMap<&'static str, &'static str> = {
        let mut map = HashMap::new();
        map.insert(AMAZON_WEB_SERVICES, "http://169.254.169.254/latest/");
        map.insert(
            MICROSOFT_AZURE,
            "http://169.254.169.254/metadata/v1/InstanceInfo",
        );
        map.insert(
            GOOGLE_CLOUD_PLATFORM,
            "http://metadata.google.internal/computeMetadata/",
        );
        map.insert(ALIBABA_CLOUD, "http://100.100.100.200/latest/");
        map.insert(OPENSTACK, "http://169.254.169.254/openstack/");
        map
    };
}

/// Makes a GET request to the specified metadata URL and returns true if successful.
///
/// # Arguments
///
/// * `metadata_url` - The metadata URL for the cloud service provider.
async fn ping(metadata_url: &str) -> bool {
    match reqwest::get(metadata_url).await {
        Ok(resp) => resp.status() == reqwest::StatusCode::OK,
        Err(_) => false,
    }
}

// TODO: add test(s)
/// Returns a list of the currently supported cloud service providers.
pub fn supported_providers() -> Vec<&'static str> {
    PROVIDER_METADATA_MAP
        .keys()
        .copied()
        .collect::<Vec<&'static str>>()
}

// TODO: add test(s)
/// Detects the current host's cloud service provider.
/// Returns "unknown" if the detection failed, if the current cloud service provider is unsupported, or if minor errors occurred during detection.
///
/// # Arguments
///
/// * `timeout` - How long to attempt detection for (in seconds). Defaults to 3 seconds.
pub async fn detect(timeout: Option<u64>) -> String {
    // Set default timeout if none specified.
    let timeout_duration = Duration::from_secs(timeout.unwrap_or(DETECTION_TIMEOUT));

    // Concurrently check if the current host belongs to any of the supported providers and write the detected provider
    // to a channel.
    let (tx, rx) = mpsc::sync_channel::<String>(1);
    for (provider, metadata_url) in PROVIDER_METADATA_MAP.iter() {
        let tx = tx.clone();
        tokio::spawn(async move {
            if ping(metadata_url).await {
                tx.send(provider.to_string()).unwrap();
            }
        });
    }

    // Wait for a value from the channel or timeout.
    let start_time = Instant::now();
    let provider = loop {
        match rx.try_recv() {
            Ok(value) => break value,
            Err(TryRecvError::Empty) => {
                if start_time.elapsed() >= timeout_duration {
                    break "unknown".to_string();
                }
            }
            Err(_) => break "unknown".to_string(),
        }
    };

    provider
}

// TODO: add test(s)
/// Attempts to detect the current host's cloud service provider.
/// If we encounter an error, we return it rather than unwrapping or assuming the provider as "unknown".
///
/// **NOTE**: This also means that this function returns an error if the current host's provider is unsupported.
///
/// # Arguments
///
/// * `timeout` - How long to attempt detection for (in seconds). Defaults to 3 seconds.
pub async fn try_detect(timeout: Option<u64>) -> Result<String, Box<dyn Error>> {
    // Set default timeout if none specified.
    let timeout_duration = Duration::from_secs(timeout.unwrap_or(DETECTION_TIMEOUT));

    // Concurrently check if the current host belongs to any of the supported providers and write the detected provider
    // to a channel.
    let (tx, rx) = mpsc::sync_channel::<String>(1);
    for (provider, metadata_url) in PROVIDER_METADATA_MAP.iter() {
        let tx = tx.clone();
        tokio::spawn(async move {
            if ping(metadata_url).await {
                tx.send(provider.to_string())?;
            }

            Ok::<(), Box<dyn Error + Send + Sync>>(())
        });
    }

    // Wait for a value from the channel or timeout.
    let start_time = Instant::now();
    let provider = loop {
        match rx.try_recv() {
            Ok(value) => break Ok(value),
            Err(TryRecvError::Empty) => {
                if start_time.elapsed() >= timeout_duration {
                    break Err("Timed out when attempting to detect provider".to_string())?;
                }
            }
            Err(err) => break Err(err),
        }
    }?;

    Ok(provider)
}
