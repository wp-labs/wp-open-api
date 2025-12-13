// Centralized error module for wp-connector-api
pub mod sink;
pub mod source;

pub use sink::{ReasonSummary, SinkError, SinkErrorOwe, SinkReason, SinkResult};
pub use source::{SourceError, SourceReason, SourceResult};
