use serde::{Deserialize, Serialize};

use crate::ParamMap;

/// Defines whether a connector operates as a data source or sink.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub enum ConnectorScope {
    /// Connector reads data (default)
    #[default]
    Source,
    /// Connector writes data
    Sink,
}

/// Connector definition containing metadata and default configuration.
///
/// This struct is used to describe a connector's capabilities and defaults.
/// It can be serialized/deserialized for configuration files.
///
/// # Serialization
/// - `kind` is serialized as `"type"`
/// - `default_params` is serialized as `"params"`
/// - `scope` and `origin` are runtime-only fields (not serialized)
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConnectorDef {
    /// Unique identifier for this connector instance
    pub id: String,
    /// Connector type (e.g., "kafka", "mysql", "elasticsearch")
    #[serde(rename = "type")]
    pub kind: String,
    /// Whether this definition is for source or sink (runtime only)
    #[serde(skip, default)]
    pub scope: ConnectorScope,
    /// Parameter keys that can be overridden at runtime
    #[serde(default)]
    pub allow_override: Vec<String>,
    /// Default parameter values
    #[serde(default, rename = "params")]
    pub default_params: ParamMap,
    /// Origin identifier for tracking (runtime only)
    #[serde(skip, default)]
    pub origin: Option<String>,
}

impl ConnectorDef {
    /// Set the scope and return self for chaining.
    pub fn with_scope(mut self, scope: ConnectorScope) -> Self {
        self.scope = scope;
        self
    }
}

/// Trait for connectors that can act as a data source.
///
/// Implement this trait to provide source connector metadata and validation.
/// Used by [`SourceFactory`] to obtain connector definitions.
///
/// # Example
/// ```ignore
/// impl SourceDefProvider for MyConnector {
///     fn source_def(&self) -> ConnectorDef {
///         ConnectorDef {
///             id: "my-source".into(),
///             kind: "custom".into(),
///             scope: ConnectorScope::Source,
///             allow_override: vec!["batch_size".into()],
///             default_params: Default::default(),
///             origin: None,
///         }
///     }
/// }
/// ```
pub trait SourceDefProvider: Send + Sync + 'static {
    /// Returns the connector definition for source mode.
    fn source_def(&self) -> ConnectorDef;

    /// Validates a source connector definition.
    ///
    /// Override to add custom validation logic. Returns `Ok(())` by default.
    fn validate_source(&self, _def: &ConnectorDef) -> Result<(), String> {
        Ok(())
    }
}

/// Trait for connectors that can act as a data sink.
///
/// Implement this trait to provide sink connector metadata and validation.
/// Used by [`SinkFactory`] to obtain connector definitions.
///
/// # Example
/// ```ignore
/// impl SinkDefProvider for MyConnector {
///     fn sink_def(&self) -> ConnectorDef {
///         ConnectorDef {
///             id: "my-sink".into(),
///             kind: "custom".into(),
///             scope: ConnectorScope::Sink,
///             allow_override: vec![],
///             default_params: Default::default(),
///             origin: None,
///         }
///     }
/// }
/// ```
pub trait SinkDefProvider: Send + Sync + 'static {
    /// Returns the connector definition for sink mode.
    fn sink_def(&self) -> ConnectorDef;

    /// Validates a sink connector definition.
    ///
    /// Override to add custom validation logic. Returns `Ok(())` by default.
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

    // Test that SourceDefProvider can be implemented independently
    struct SourceOnlyConnector;

    impl SourceDefProvider for SourceOnlyConnector {
        fn source_def(&self) -> ConnectorDef {
            ConnectorDef {
                id: "source-only".into(),
                kind: "test".into(),
                scope: ConnectorScope::Source,
                allow_override: vec![],
                default_params: Default::default(),
                origin: None,
            }
        }
    }

    #[test]
    fn test_source_only_connector() {
        let connector = SourceOnlyConnector;
        let def = connector.source_def();
        assert_eq!(def.id, "source-only");
        assert_eq!(def.scope, ConnectorScope::Source);

        // validate_source has default implementation
        assert!(connector.validate_source(&def).is_ok());
    }

    // Test that SinkDefProvider can be implemented independently
    struct SinkOnlyConnector;

    impl SinkDefProvider for SinkOnlyConnector {
        fn sink_def(&self) -> ConnectorDef {
            ConnectorDef {
                id: "sink-only".into(),
                kind: "test".into(),
                scope: ConnectorScope::Sink,
                allow_override: vec![],
                default_params: Default::default(),
                origin: None,
            }
        }
    }

    #[test]
    fn test_sink_only_connector() {
        let connector = SinkOnlyConnector;
        let def = connector.sink_def();
        assert_eq!(def.id, "sink-only");
        assert_eq!(def.scope, ConnectorScope::Sink);

        // validate_sink has default implementation
        assert!(connector.validate_sink(&def).is_ok());
    }
}
