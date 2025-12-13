use async_trait::async_trait;
use std::{path::PathBuf, sync::Arc};
use wp_model_core::model::DataRecord;

use crate::SinkResult;

// Reuse workspace error type to avoid duplicating an error abstraction

// ---------- Core Sink Traits ----------

#[async_trait]
pub trait AsyncCtrl {
    async fn stop(&mut self) -> SinkResult<()>;
    async fn reconnect(&mut self) -> SinkResult<()>;
}

#[async_trait]
pub trait AsyncRecordSink {
    async fn sink_record(&mut self, data: &DataRecord) -> SinkResult<()>;
    async fn sink_records(&mut self, data: Vec<Arc<DataRecord>>) -> SinkResult<()>;
}

#[async_trait]
pub trait AsyncRawDataSink {
    async fn sink_str(&mut self, data: &str) -> SinkResult<()>;

    async fn sink_bytes(&mut self, data: &[u8]) -> SinkResult<()>;

    async fn sink_str_batch(&mut self, data: Vec<&str>) -> SinkResult<()>;

    async fn sink_bytes_batch(&mut self, data: Vec<&[u8]>) -> SinkResult<()>;
}

pub trait AsyncSink: AsyncRecordSink + AsyncRawDataSink + AsyncCtrl + Send + Sync {}

impl<T> AsyncSink for T where T: AsyncRecordSink + AsyncRawDataSink + AsyncCtrl + Send + Sync {}

// ---------- Build Ctx ----------

#[derive(Clone, Debug)]
pub struct SinkBuildCtx {
    pub work_root: PathBuf,
    /// Replica index for parallel group builds (0-based). Defaults to 0.
    pub replica_idx: usize,
    /// Replica count for the group (>=1). Defaults to 1.
    pub replica_cnt: usize,
    pub rate_limit_rps: usize,
}

impl SinkBuildCtx {
    pub fn new(work_root: PathBuf) -> Self {
        Self {
            work_root,
            replica_idx: 0,
            replica_cnt: 1,
            rate_limit_rps: 0,
        }
    }
    pub fn new_with_replica(work_root: PathBuf, replica_idx: usize, replica_cnt: usize) -> Self {
        Self {
            work_root,
            replica_idx,
            replica_cnt: replica_cnt.max(1),
            rate_limit_rps: 0,
        }
    }
    pub fn with_limit(mut self, rate_limit_rps: usize) -> Self {
        self.rate_limit_rps = rate_limit_rps;
        self
    }
}

pub struct SinkHandle {
    pub sink: Box<dyn AsyncSink + 'static>,
}

impl std::fmt::Debug for SinkHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Name matches the type to avoid confusion in logs/diagnostics
        f.debug_struct("SinkHandle")
            .field("sink", &"Box<dyn AsyncSink>")
            .finish()
    }
}

impl SinkHandle {
    pub fn new(sink: Box<dyn AsyncSink + 'static>) -> Self {
        Self { sink }
    }
}

// ---------- Resolved Route Spec + Factory (for runtime decoupling) ----------

#[derive(Debug, Clone)]
pub struct ResolvedSinkSpec {
    pub group: String,
    pub name: String,
    pub kind: String,
    pub connector_id: String,
    pub params: crate::types::ParamMap,
    pub filter: Option<String>,
}

#[async_trait]
pub trait SinkFactory: Send + Sync + 'static {
    fn kind(&self) -> &'static str;
    /// 可选的类型特有校验（默认空实现）
    fn validate_spec(&self, _spec: &ResolvedSinkSpec) -> anyhow::Result<()> {
        Ok(())
    }
    async fn build(
        &self,
        spec: &ResolvedSinkSpec,
        ctx: &SinkBuildCtx,
    ) -> anyhow::Result<SinkHandle>;
}
