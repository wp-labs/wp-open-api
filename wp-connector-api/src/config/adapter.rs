use crate::types::ParamMap;

/// Adapter trait for parsing connector URL into flattened params and providing defaults.
///
/// Note: The global registry and lifecycle management of adapters live in wp-engine.
/// This API crate only defines the trait and common ParamMap type to keep the interface stable.
pub trait ConnectorKindAdapter: Send + Sync {
    fn kind(&self) -> &'static str;
    fn defaults(&self) -> ParamMap {
        ParamMap::new()
    }
    fn url_to_params(&self, _url: &str) -> anyhow::Result<ParamMap> {
        Ok(ParamMap::new())
    }
}
