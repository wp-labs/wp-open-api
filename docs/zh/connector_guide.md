# `wp-connector-api` 开发指南

本指南基于当前 `wp-connector-api` 代码，囊括配置期适配器、Sink/Source 运行时接口、错误模型以及常用实践，方便在实现新连接器或调试现有实现时快速查阅。

## 1. 配置期接口

- **ParamMap**：统一的参数容器（`BTreeMap<String, serde_json::Value>`），通过 `config::param` 中的 `parammap_from_toml_table/map` 将 TOML 配置扁平化，保持键排序以便 diff/缓存。
- **ConnectorKindAdapter**：负责把 `conn_url` 等人类可读输入转换成 ParamMap。实现需提供：
  - `kind(&self) -> &'static str`：唯一标识。
  - `defaults(&self)`：每个连接器的默认键值，返回 `ParamMap::new()` 时表示无默认项。
  - `url_to_params(&self, url: &str)`：解析 URL → ParamMap，遇到不支持的格式返回 `anyhow::Error`。

### 1.1 连接器定义 API

**ConnectorScope**：表示连接器的作用域枚举，可选值：
- `Source`（默认值）：数据源连接器。
- `Sink`：数据目标连接器。

**ConnectorDef**：连接器元数据定义结构体，字段说明：
- `id: String`：连接器唯一标识。
- `kind: String`：连接器类型（序列化时字段名为 `type`）。
- `scope: ConnectorScope`：作用域，运行时字段，不参与序列化。
- `allow_override: Vec<String>`：允许覆盖的参数键列表。
- `default_params: ParamMap`：默认参数（序列化时字段名为 `params`）。
- `origin: Option<String>`：来源标识，运行时字段，不参与序列化。

`ConnectorDef` 提供链式构造方法：
- `with_scope(scope: ConnectorScope) -> Self`：设置作用域并返回自身。

**ConnectorDefProvider** trait：为连接器实现提供统一的定义与验证接口。
- `source_def(&self) -> ConnectorDef`：返回 Source 连接器定义，未实现时会 panic。
- `sink_def(&self) -> ConnectorDef`：返回 Sink 连接器定义，未实现时会 panic。
- `validate_source(&self, def: &ConnectorDef) -> Result<(), String>`：校验 Source 定义，默认返回 `Ok(())`。
- `validate_sink(&self, def: &ConnectorDef) -> Result<(), String>`：校验 Sink 定义，默认返回 `Ok(())`。

## 2. Sink 运行时接口

### 2.1 核心 Trait

- `AsyncCtrl`：运行期控制。
  - `stop(&mut self)`：幂等停止，释放所有资源；调用后应保证 `receive` 等任务停止。
  - `reconnect(&mut self)`：重建连接或刷新上下文，需保证外部语义不变。
- `AsyncRecordSink`：结构化记录写入。
  - `sink_record(&mut self, &DataRecord)`：单条写入。
  - `sink_records(&mut self, Vec<Arc<DataRecord>>)`：批量写入，保持批次顺序。
- `AsyncRawDataSink`：原始文本/字节写入。
  - `sink_str` / `sink_bytes`：单条输入。
  - `sink_str_batch` / `sink_bytes_batch`：批量输入。
- `AsyncSink`：组合 `AsyncCtrl + AsyncRecordSink + AsyncRawDataSink + Send + Sync`。给实现体 `impl AsyncSink for MySink {}` 即可，让 orchestrator 统一调度。

### 2.2 构建管线

- `SinkBuildCtx`
  - `work_root: PathBuf`：每个实例的沙箱目录。
  - `replica_idx/replica_cnt`：并行构建序号与总数（均为 0-based/>=1）。
  - `rate_limit_rps`：上游推荐速率限制，可用于限速或发号器。
- `ResolvedSinkSpec`
  - `group/name/kind/connector_id`：识别信息。
  - `params: ParamMap`：已经扁平化的运行参数。
  - `filter: Option<String>`：可选过滤表达式，具体语义由使用者决定。
- `SinkFactory`
  - `kind()`：注册名。
  - `validate_spec()`：轻量参数校验，默认 no-op。
  - `build(spec, ctx) -> SinkHandle`：构造 `Box<dyn AsyncSink>`；失败时返回 `anyhow::Error`，由 orchestrator 记录。

## 3. Source 运行时接口

### 3.1 DataSource 行为

- `receive(&mut self) -> SourceResult<SourceBatch>`：唯一必需方法，批量返回 `SourceEvent`。空 Vec 表示暂时无数据。
- `try_receive(&mut self) -> Option<SourceBatch>`：仅当 `supports_try_receive()` 与 `can_try_receive()` 同时满足时使用；否则返回 `None`。
- `supports_try_receive(&self)`：静态能力；默认 `false`。
- `can_try_receive(&mut self)`：动态能力；默认沿用静态能力。
- `identifier()` / `identifier_ref()`：用于日志与指标。`identifier_ref` 默认回退到 `String`，建议实现零分配版本。
- 生命周期：
  - `start(&mut self, CtrlRx)`：启动前置资源，可监听控制事件，默认 no-op。
  - `close(&mut self)`：幂等关闭。
- 拓展能力：
  - `caps(&self) -> SourceCaps`：声明 `ack`/`seek`/`parallel` 支持。
  - `ack(&mut self, Arc<dyn AckToken>)`：默认返回 `SupplierError("ack unsupported")`。
  - `seek(&mut self, Arc<dyn SeekPosition>)`：默认 `SupplierError("seek unsupported")`。

### 3.2 事件与控制

- `SourceEvent`
- `event_id`、`src_key`、`payload: RawData`、`tags: Arc<Tags>`、`ups_ip`、`preproc`。`payload` 支持 `String`/`Bytes`/`Arc<Vec<u8>>`，调试输出会自动汇总长度。
- `ControlEvent`
  - `Stop`：请求立即停产。
  - `Isolate(bool)`：`true` 进入隔离暂停，`false` 恢复。
  - `Seek(Arc<dyn SeekPosition>)`：请求定位。
- `CtrlRx = async_broadcast::Receiver<ControlEvent>`：在 `start()` 中监听控制命令，及时响应。
- `Tags`
  - 内部使用 `SmallVec` 保持排序；提供 `set/get/is_empty` 等方法。已有单元测试保证插入/更新顺序稳定。

### 3.3 SourceFactory 管线

- `SourceBuildCtx { work_root }`：与 Sink 相同，提供实例本地目录。
- `SourceMeta { name, kind, tags }`：用于 UI/监控展示。
- `SourceHandle { source, metadata }`：单个可拉取实例。
- `AcceptorHandle { name, acceptor }`：面向 server-side source（如 HTTP 接入）的监听器。
- `SourceSvcIns { sources, acceptor }`：`SourceFactory::build` 的返回值，允许同一个 spec 注册多个 `DataSource` 或额外 acceptor。
- `ResolvedSourceSpec`
  - 字段：`name`、`kind`、`connector_id`、`params: ParamMap`、`tags: Vec<String>`。
  - `SourceFactory` 需实现 `kind()`、可选 `validate_spec()`、以及 `build(spec, ctx)`。

## 4. 错误模型

- Sink 侧：
  - `SinkReason`/`SinkError` 基于 `orion_error::StructError`，提供 `SinkReason::sink(msg)` 以及 `SinkErrorOwe` trait（`owe_sink("msg")?`）用于包装外部错误。
  - 常见枚举：`Sink(String)`、`Mock`、`StgCtrl`、`Uvs`。所有 `ErrorCode` 统一返回 `255`，方便与外部系统对齐。
- Source 侧：
  - `SourceReason`/`SourceError` 同样走 `StructError`，包含 `NotData`、`EOF`、`SupplierError(String)` 等变体。
  - `SourceResult<T>` = `Result<T, StructError<SourceReason>>`，在 `DataSource` 实现中直接使用。

## 5. 示例：内存连接器

下面是一个完整的内存连接器示例，同时实现 Source 和 Sink，可作为开发新连接器的参考模板。

### 5.1 MemorySource 实现

```rust
use std::sync::Arc;
use async_trait::async_trait;
use wp_connector_api::{
    DataSource, SourceBatch, SourceEvent, SourceResult, Tags,
};
use wp_parse_api::RawData;

struct MemorySource {
    name: String,
    events: Vec<String>,
    cursor: usize,
}

#[async_trait]
impl DataSource for MemorySource {
    async fn receive(&mut self) -> SourceResult<SourceBatch> {
        if self.cursor >= self.events.len() {
            return Ok(vec![]); // 无更多数据
        }
        let event = SourceEvent::new(
            self.cursor as u64,
            Arc::new(self.name.clone()),
            RawData::from_string(&self.events[self.cursor]),
            Arc::new(Tags::default()),
        );
        self.cursor += 1;
        Ok(vec![event])
    }

    fn try_receive(&mut self) -> Option<SourceBatch> {
        if self.cursor < self.events.len() {
            let event = SourceEvent::new(
                self.cursor as u64,
                Arc::new(self.name.clone()),
                RawData::from_string(&self.events[self.cursor]),
                Arc::new(Tags::default()),
            );
            self.cursor += 1;
            Some(vec![event])
        } else {
            None
        }
    }

    fn supports_try_receive(&self) -> bool { true }

    fn identifier(&self) -> String {
        format!("memory-source:{}", self.name)
    }
}
```

### 5.2 MemorySink 实现

```rust
use std::sync::{Arc, Mutex};
use async_trait::async_trait;
use wp_connector_api::{
    AsyncCtrl, AsyncRawDataSink, AsyncRecordSink, SinkResult,
};
use wp_model_core::model::DataRecord;

#[derive(Clone, Default)]
struct MemorySinkBuffer {
    data: Arc<Mutex<Vec<String>>>,
}

struct MemorySink {
    buffer: MemorySinkBuffer,
}

#[async_trait]
impl AsyncCtrl for MemorySink {
    async fn stop(&mut self) -> SinkResult<()> { Ok(()) }
    async fn reconnect(&mut self) -> SinkResult<()> { Ok(()) }
}

#[async_trait]
impl AsyncRecordSink for MemorySink {
    async fn sink_record(&mut self, _data: &DataRecord) -> SinkResult<()> {
        self.buffer.data.lock().unwrap().push("record".into());
        Ok(())
    }
    async fn sink_records(&mut self, data: Vec<Arc<DataRecord>>) -> SinkResult<()> {
        let mut buf = self.buffer.data.lock().unwrap();
        for _ in data { buf.push("record".into()); }
        Ok(())
    }
}

#[async_trait]
impl AsyncRawDataSink for MemorySink {
    async fn sink_str(&mut self, data: &str) -> SinkResult<()> {
        self.buffer.data.lock().unwrap().push(data.to_string());
        Ok(())
    }
    async fn sink_bytes(&mut self, data: &[u8]) -> SinkResult<()> {
        self.buffer.data.lock().unwrap().push(String::from_utf8_lossy(data).into());
        Ok(())
    }
    async fn sink_str_batch(&mut self, data: Vec<&str>) -> SinkResult<()> {
        let mut buf = self.buffer.data.lock().unwrap();
        for s in data { buf.push(s.to_string()); }
        Ok(())
    }
    async fn sink_bytes_batch(&mut self, data: Vec<&[u8]>) -> SinkResult<()> {
        let mut buf = self.buffer.data.lock().unwrap();
        for b in data { buf.push(String::from_utf8_lossy(b).into()); }
        Ok(())
    }
}
```

### 5.3 ConnectorFactory 实现

```rust
use async_trait::async_trait;
use wp_connector_api::{
    ConnectorDef, ConnectorDefProvider, ConnectorScope,
    SinkFactory, SinkHandle, SinkBuildCtx, SinkSpec, SinkResult,
    SourceFactory, SourceHandle, SourceMeta, SourceBuildCtx,
    SourceSpec, SourceResult, SourceSvcIns,
};

struct DemoConnectorFactory {
    sink_buffer: MemorySinkBuffer,
    source_events: Vec<String>,
}

impl ConnectorDefProvider for DemoConnectorFactory {
    fn source_def(&self) -> ConnectorDef {
        ConnectorDef {
            id: "demo-source".into(),
            kind: "memory".into(),
            scope: ConnectorScope::Source,
            allow_override: vec!["events".into()],
            default_params: Default::default(),
            origin: Some("demo".into()),
        }
    }
    fn sink_def(&self) -> ConnectorDef {
        ConnectorDef {
            id: "demo-sink".into(),
            kind: "memory".into(),
            scope: ConnectorScope::Sink,
            ..Default::default()
        }
    }
}

#[async_trait]
impl SourceFactory for DemoConnectorFactory {
    fn kind(&self) -> &'static str { "memory" }

    async fn build(&self, spec: &SourceSpec, _ctx: &SourceBuildCtx) -> SourceResult<SourceSvcIns> {
        let source = MemorySource {
            name: spec.name.clone(),
            events: self.source_events.clone(),
            cursor: 0,
        };
        let handle = SourceHandle::new(
            Box::new(source),
            SourceMeta::new(&spec.name, "memory"),
        );
        Ok(SourceSvcIns::new().with_sources(vec![handle]))
    }
}

#[async_trait]
impl SinkFactory for DemoConnectorFactory {
    fn kind(&self) -> &'static str { "memory" }

    async fn build(&self, _spec: &SinkSpec, _ctx: &SinkBuildCtx) -> SinkResult<SinkHandle> {
        Ok(SinkHandle::new(Box::new(MemorySink {
            buffer: self.sink_buffer.clone(),
        })))
    }
}
```

完整示例代码参见 `wp-connector-api/tests/demo_connector.rs`。

## 6. 实践建议

1. **参数校验前置**：在 `ConnectorKindAdapter::url_to_params` 和 `SinkFactory::validate_spec` 中尽早发现拼写/格式问题，避免运行期才报错。
2. **幂等与错误传播**：`stop`/`receive`/`ack` 等接口都要求幂等，不要 `unwrap/expect`，统一返回 `SinkResult/SourceResult`。
3. **零拷贝优先**：`SourceEvent.payload` 支持 `RawData::ArcBytes`，若上游可直接提供 `Arc<Vec<u8>>`，`into_bytes()` 会尝试 `Arc::try_unwrap`，在引用计数为 1 时实现零拷贝复用。
4. **测试配置转换**：`config::param` 已添加 `parammap_from_toml_*` 的单元测试，若新增复杂结构（例如多层数组或自定义类型），请同步扩充测试数据，保证不同 TOML 表达式转到 ParamMap 后一致。
5. **文档同步**：新增接口或语义时请更新本文件以及 README，确保多语言版本保持一致。

如有进一步问题，可在 PR/Issue 中 @WarpParse Dev Team 讨论。
