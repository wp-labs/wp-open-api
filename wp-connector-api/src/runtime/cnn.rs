use serde::{Deserialize, Serialize};

use crate::ParamMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ConnectorScope {
    Source,
    Sink,
}

impl Default for ConnectorScope {
    fn default() -> Self {
        ConnectorScope::Source
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectorDef {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(skip, default)]
    pub scope: ConnectorScope,
    #[serde(default)]
    pub allow_override: Vec<String>,
    #[serde(default, rename = "params")]
    pub default_params: ParamMap,
    #[serde(skip, default)]
    pub origin: Option<String>,
}

impl ConnectorDef {
    pub fn with_scope(mut self, scope: ConnectorScope) -> Self {
        self.scope = scope;
        self
    }
}

pub trait ConnectorDefProvider: Send + Sync + 'static {
    fn source_def(&self) -> ConnectorDef {
        panic!("source_def not implemented")
    }
    fn sink_def(&self) -> ConnectorDef {
        panic!("sink_def not implemented")
    }
    fn validate_source(&self, _def: &ConnectorDef) -> Result<(), String> {
        Ok(())
    }
    fn validate_sink(&self, _def: &ConnectorDef) -> Result<(), String> {
        Ok(())
    }
}
