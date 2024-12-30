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

#[allow(dead_code)]
static PROVIDERS: LazyLock<Mutex<Vec<P>>> =
    LazyLock::new(|| Mutex::new(vec![Arc::new(alibaba::Alibaba) as P]));

pub fn supported_providers() -> Result<ProviderId> {
    todo!()
}

#[allow(unused_variables)]
pub fn detect(timeout: Option<u64>) -> Result<ProviderId> {
    todo!()
}
