///! Detects a host's cloud service provider in a blocking manner.
use std::error::Error;
use std::sync::mpsc::{self, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

use crate::{DETECTION_TIMEOUT, PROVIDER_METADATA_MAP};

/// Makes a blocking GET request to the specified metadata URL and returns true if successful.
///
/// # Arguments
///
/// * `metadata_url` - The metadata URL for the cloud service provider.
fn ping(metadata_url: &str) -> bool {
    match reqwest::blocking::get(metadata_url) {
        Ok(resp) => resp.status() == reqwest::StatusCode::OK,
        Err(_) => false,
    }
}

// TODO: add test(s)
/// Detects the current host's cloud service provider in a blocking manner.
/// Returns "unknown" if the detection failed, if the current cloud service provider is unsupported, or if minor errors occurred during detection.
///
/// # Arguments
///
/// * `timeout` - How long to attempt detection for (in seconds). Defaults to 3 seconds.
pub fn detect(timeout: Option<u64>) -> String {
    // Set default timeout if none specified.
    let timeout_duration = Duration::from_secs(timeout.unwrap_or(DETECTION_TIMEOUT));

    // Concurrently check if the current host belongs to any of the supported providers and write the detected provider
    // to a channel.
    let (tx, rx) = mpsc::sync_channel::<String>(1);
    for (provider, metadata_url) in PROVIDER_METADATA_MAP.iter() {
        let tx = tx.clone();
        thread::spawn(move || {
            if ping(metadata_url) {
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
/// Attempts to detect the current host's cloud service provider in a blocking manner.
/// If we encounter an error, we return it rather than unwrapping or assuming the provider as "unknown".
///
/// **NOTE**: This also means that this function returns an error if the current host's provider is unsupported.
///
/// # Arguments
///
/// * `timeout` - How long to attempt detection for (in seconds). Defaults to 3 seconds.
pub fn try_detect(timeout: Option<u64>) -> Result<String, Box<dyn Error>> {
    // Set default timeout if none specified.
    let timeout_duration = Duration::from_secs(timeout.unwrap_or(DETECTION_TIMEOUT));

    // Concurrently check if the current host belongs to any of the supported providers and write the detected provider
    // to a channel.
    let (tx, rx) = mpsc::sync_channel::<String>(1);
    for (provider, metadata_url) in PROVIDER_METADATA_MAP.iter() {
        let tx = tx.clone();
        thread::spawn(move || {
            if ping(metadata_url) {
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
