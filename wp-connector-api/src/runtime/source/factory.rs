use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::types::{CtrlRx, DataSource, Tags};
use crate::{ConnectorDefProvider, SourceResult, types::ParamMap};

#[async_trait]
pub trait ServiceAcceptor: Send {
    /// 接受连接/启动服务端式源，处理外部控制事件
    async fn accept_connection(&mut self, ctrl_rx: CtrlRx) -> SourceResult<()>;
}

#[derive(Clone, Debug)]
pub struct SourceBuildCtx {
    pub work_root: PathBuf,
}

impl SourceBuildCtx {
    pub fn new(work_root: PathBuf) -> Self {
        Self { work_root }
    }
}

/// 数据源元信息，供 orchestrator/调度层用于统计与展示。
#[derive(Clone, Debug)]
pub struct SourceMeta {
    pub name: String,
    pub kind: String,
    pub tags: Tags,
}

impl SourceMeta {
    pub fn new(name: impl Into<String>, kind: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: kind.into(),
            tags: Tags::default(),
        }
    }
}

/// 单个可注册的数据源实例。
pub struct SourceHandle {
    pub source: Box<dyn DataSource + 'static>,
    pub metadata: SourceMeta,
}

impl std::fmt::Debug for SourceHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceHandle")
            .field("source", &"Box<dyn DataSource>")
            .field("metadata", &self.metadata)
            .finish()
    }
}

impl SourceHandle {
    pub fn new(source: Box<dyn DataSource + 'static>, metadata: SourceMeta) -> Self {
        Self { source, metadata }
    }
}

/// 包含 acceptor 具体实例及可读名称。
pub struct AcceptorHandle {
    pub name: String,
    pub acceptor: Box<dyn ServiceAcceptor + Send>,
}

impl std::fmt::Debug for AcceptorHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AcceptorHandle")
            .field("name", &self.name)
            .field("acceptor", &"Box<dyn ServiceAcceptor>")
            .finish()
    }
}

impl AcceptorHandle {
    pub fn new(name: impl Into<String>, acceptor: Box<dyn ServiceAcceptor + Send>) -> Self {
        Self {
            name: name.into(),
            acceptor,
        }
    }
}

/// SourceFactory::build 的统一返回结构。
#[derive(Default)]
pub struct SourceSvcIns {
    pub sources: Vec<SourceHandle>,
    pub acceptor: Option<AcceptorHandle>,
}

impl std::fmt::Debug for SourceSvcIns {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceSvcIns")
            .field("sources", &format!("len={}", self.sources.len()))
            .field(
                "acceptor",
                if self.acceptor.is_some() {
                    &"Some(AcceptorHandle)"
                } else {
                    &"None"
                },
            )
            .finish()
    }
}

impl SourceSvcIns {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_sources(mut self, sources: Vec<SourceHandle>) -> Self {
        self.sources = sources;
        self
    }

    pub fn push_source(&mut self, instance: SourceHandle) {
        self.sources.push(instance);
    }

    pub fn with_acceptor(mut self, acceptor: AcceptorHandle) -> Self {
        self.acceptor = Some(acceptor);
        self
    }
}

/// ResolvedSourceSpec：统一 Factory 构建使用的规格（包含 connector_id，参数一律扁平）。
#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct ResolvedSourceSpec {
    pub name: String,
    pub kind: String,
    pub connector_id: String,
    #[serde(default)]
    pub params: ParamMap,
    /// Optional tags propagated from CoreSpec/config. Keep here to ease adapters.
    #[serde(default)]
    pub tags: Vec<String>,
}

#[async_trait]
pub trait SourceFactory: ConnectorDefProvider + Send + Sync + 'static {
    fn kind(&self) -> &'static str;
    /// 可选：轻量级参数校验（不产生 I/O），用于尽早暴露参数错误。
    fn validate_spec(&self, _spec: &ResolvedSourceSpec) -> SourceResult<()> {
        Ok(())
    }
    async fn build(
        &self,
        spec: &ResolvedSourceSpec,
        ctx: &SourceBuildCtx,
    ) -> SourceResult<SourceSvcIns>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::source::SourceBatch;
    use async_trait::async_trait;
    use serde_json::json;
    use std::path::PathBuf;

    #[derive(Default)]
    struct DummySource {
        id: &'static str,
    }

    #[async_trait]
    impl DataSource for DummySource {
        async fn receive(&mut self) -> SourceResult<SourceBatch> {
            Ok(Vec::new())
        }

        fn try_receive(&mut self) -> Option<SourceBatch> {
            None
        }

        fn identifier(&self) -> String {
            self.id.to_string()
        }
    }

    #[derive(Default)]
    struct DummyAcceptor;

    #[async_trait]
    impl ServiceAcceptor for DummyAcceptor {
        async fn accept_connection(&mut self, _ctrl_rx: CtrlRx) -> SourceResult<()> {
            Ok(())
        }
    }

    fn make_source_handle(id: &'static str) -> SourceHandle {
        SourceHandle::new(Box::new(DummySource { id }), SourceMeta::new(id, "dummy"))
    }

    #[test]
    fn source_build_ctx_and_meta_helpers() {
        let ctx = SourceBuildCtx::new(PathBuf::from("/tmp/source"));
        assert_eq!(ctx.work_root, PathBuf::from("/tmp/source"));

        let meta = SourceMeta::new("orders", "http");
        assert_eq!(meta.name, "orders");
        assert_eq!(meta.kind, "http");
        assert_eq!(meta.tags.len(), 0);
    }

    #[test]
    fn handle_constructors_store_inner_state() {
        let meta = SourceMeta::new("alpha", "kafka");
        let handle = SourceHandle::new(Box::new(DummySource { id: "alpha" }), meta.clone());
        assert_eq!(handle.metadata.name, meta.name);
        assert_eq!(handle.source.identifier(), "alpha");

        let acceptor = AcceptorHandle::new("http", Box::new(DummyAcceptor));
        assert_eq!(acceptor.name, "http");
    }

    #[test]
    fn source_svc_ins_builders_manage_members() {
        let mut svc = SourceSvcIns::new().with_sources(vec![make_source_handle("a")]);
        assert_eq!(svc.sources.len(), 1);

        svc.push_source(make_source_handle("b"));
        assert_eq!(svc.sources.len(), 2);

        let svc = svc.with_acceptor(AcceptorHandle::new("svc", Box::new(DummyAcceptor)));
        assert!(svc.acceptor.is_some());
    }

    #[test]
    fn resolved_source_spec_defaults_optional_fields() {
        let spec: ResolvedSourceSpec = serde_json::from_value(json!({
            "name": "demo",
            "kind": "http",
            "connector_id": "conn-1"
        }))
        .unwrap();

        assert!(spec.params.is_empty());
        assert!(spec.tags.is_empty());
    }
}
