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

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use std::{path::PathBuf, sync::Arc};
    use wp_model_core::model::DataRecord;

    #[derive(Default)]
    struct NoopSink;

    #[async_trait]
    impl AsyncCtrl for NoopSink {
        async fn stop(&mut self) -> SinkResult<()> {
            Ok(())
        }

        async fn reconnect(&mut self) -> SinkResult<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl AsyncRecordSink for NoopSink {
        async fn sink_record(&mut self, _data: &DataRecord) -> SinkResult<()> {
            Ok(())
        }

        async fn sink_records(&mut self, _data: Vec<Arc<DataRecord>>) -> SinkResult<()> {
            Ok(())
        }
    }

    #[async_trait]
    impl AsyncRawDataSink for NoopSink {
        async fn sink_str(&mut self, _data: &str) -> SinkResult<()> {
            Ok(())
        }

        async fn sink_bytes(&mut self, _data: &[u8]) -> SinkResult<()> {
            Ok(())
        }

        async fn sink_str_batch(&mut self, _data: Vec<&str>) -> SinkResult<()> {
            Ok(())
        }

        async fn sink_bytes_batch(&mut self, _data: Vec<&[u8]>) -> SinkResult<()> {
            Ok(())
        }
    }

    #[test]
    fn sink_build_ctx_defaults_and_limits() {
        let ctx = SinkBuildCtx::new(PathBuf::from("/tmp/work"));
        assert_eq!(ctx.work_root, PathBuf::from("/tmp/work"));
        assert_eq!(ctx.replica_idx, 0);
        assert_eq!(ctx.replica_cnt, 1);
        assert_eq!(ctx.rate_limit_rps, 0);

        let replica_ctx = SinkBuildCtx::new_with_replica(PathBuf::from("/tmp/work"), 2, 0);
        assert_eq!(replica_ctx.replica_idx, 2);
        assert_eq!(
            replica_ctx.replica_cnt, 1,
            "replica count should clamp to >=1"
        );

        let limited = SinkBuildCtx::new(PathBuf::from("/tmp/work")).with_limit(250);
        assert_eq!(limited.rate_limit_rps, 250);
        assert_eq!(limited.replica_cnt, 1);
    }

    #[test]
    fn sink_handle_wraps_async_sink() {
        let handle = SinkHandle::new(Box::new(NoopSink));
        assert!(format!("{handle:?}").contains("SinkHandle"));
    }
}
