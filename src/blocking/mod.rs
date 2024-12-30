use std::sync::mpsc::Sender;
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use anyhow::Result;

use crate::{ProviderId, P};

/// Represents a cloud service provider.
#[allow(dead_code)]
pub(crate) trait Provider: Send + Sync {
    fn identifier(&self) -> ProviderId;
    fn identify(&self, tx: Sender<ProviderId>, timeout: Duration);
}

#[allow(dead_code)]
static PROVIDERS: LazyLock<Mutex<Vec<P>>> = LazyLock::new(|| todo!());

pub fn supported_providers() -> Result<ProviderId> {
    todo!()
}

#[allow(unused_variables)]
pub fn detect(timeout: Option<u64>) -> Result<ProviderId> {
    todo!()
}
