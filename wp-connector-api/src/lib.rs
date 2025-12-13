mod config;
mod errors;
mod runtime;
mod types;
// keep top-level convenient re-exports stable
pub use config::param::{parammap_from_toml_map, parammap_from_toml_table};
pub use errors::{
    ReasonSummary, SinkError, SinkErrorOwe, SinkReason, SinkResult, SourceError, SourceReason,
    SourceResult,
};
// Friendly aliases exposed at crate root
pub use errors::sink::{ReasonSummary as ErrorSummary, SinkErrorOwe as SinkResultExt};

// --- Convenient top-level re-exports for common use cases ---
// Config-time adapter (conn_url -> params)
pub use config::adapter::ConnectorKindAdapter;
pub use types::ParamMap;
// Runtime: sink side
pub use runtime::sink::{
    AsyncCtrl, AsyncRawDataSink, AsyncRecordSink, AsyncSink, ResolvedSinkSpec as SinkSpec,
    SinkBuildCtx, SinkFactory, SinkHandle,
};

pub use runtime::source::{
    AcceptorHandle, AckToken, ControlEvent, CtrlRx, DataSource, EventPreHook,
    ResolvedSourceSpec as SourceSpec, SeekPosition, ServiceAcceptor, SourceBatch, SourceBuildCtx,
    SourceCaps, SourceEvent, SourceFactory, SourceHandle, SourceMeta, SourceSvcIns, Tags,
};
