use serde::{Deserialize, Serialize};

use crate::ParamMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum ConnectorScope {
    #[default]
    Source,
    Sink,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connector_scope_default() {
        let scope = ConnectorScope::default();
        assert_eq!(scope, ConnectorScope::Source);
    }

    #[test]
    fn test_connector_def_serde_and_with_scope() {
        let json = r#"{"id": "mysql-prod", "type": "mysql", "params": {"host": "localhost"}}"#;
        let def: ConnectorDef = serde_json::from_str(json).unwrap();

        assert_eq!(def.id, "mysql-prod");
        assert_eq!(def.kind, "mysql");
        assert_eq!(def.scope, ConnectorScope::Source); // default
        assert_eq!(def.default_params.get("host").unwrap(), "localhost");

        let def = def.with_scope(ConnectorScope::Sink);
        assert_eq!(def.scope, ConnectorScope::Sink);
    }
}
