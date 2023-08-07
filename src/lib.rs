mod consts;

use std::collections::HashMap;

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
        map
    };
}

/// Returns a list of the currently supported cloud service providers.
pub fn supported_providers() -> Vec<&'static str> {
    PROVIDER_METADATA_MAP
        .keys()
        .copied()
        .collect::<Vec<&'static str>>()
}
