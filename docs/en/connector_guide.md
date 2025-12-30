# `wp-connector-api` Developer Guide

This guide mirrors the current `wp-connector-api` implementation. It aggregates the configuration adapters, Sink/Source runtime traits, error model, and practical advice so you can build or debug connectors quickly.

## 1. Configuration-Time Interfaces

- **ParamMap**: unified parameter container (`BTreeMap<String, serde_json::Value>`). Use the helpers in `config::param` (`parammap_from_toml_table/map`) to flatten TOML tables and keep keys sorted for stable diffs.
- **ConnectorKindAdapter**: maps human-friendly inputs (such as `conn_url`) into a ParamMap. Implementors must provide:
  - `kind(&self) -> &'static str`: unique identifier.
  - `defaults(&self) -> ParamMap`: connector-specific defaults; returning `ParamMap::new()` means "no defaults".
  - `url_to_params(&self, url: &str) -> anyhow::Result<ParamMap>`: parse the URL; return an error when encountering unsupported formats.

### 1.1 Connector Definition API

**ConnectorScope**: an enum representing the connector's scope:
- `Source` (default): data source connector.
- `Sink`: data sink connector.

**ConnectorDef**: connector metadata struct with the following fields:
- `id: String`: unique connector identifier.
- `kind: String`: connector type (serialized as `type`).
- `scope: ConnectorScope`: runtime-only field, not serialized.
- `allow_override: Vec<String>`: list of parameter keys that can be overridden.
- `default_params: ParamMap`: default parameters (serialized as `params`).
- `origin: Option<String>`: origin identifier, runtime-only field, not serialized.

`ConnectorDef` provides a builder method:
- `with_scope(scope: ConnectorScope) -> Self`: set the scope and return self.

**SourceDefProvider** trait: interface for Source connector definition and validation.
- `source_def(&self) -> ConnectorDef`: returns the Source connector definition (required).
- `validate_source(&self, def: &ConnectorDef) -> Result<(), String>`: validate a Source definition; defaults to `Ok(())`.

**SinkDefProvider** trait: interface for Sink connector definition and validation.
- `sink_def(&self) -> ConnectorDef`: returns the Sink connector definition (required).
- `validate_sink(&self, def: &ConnectorDef) -> Result<(), String>`: validate a Sink definition; defaults to `Ok(())`.

> Connectors can implement one or both traits as needed. For example, a pure Source connector only needs to implement `SourceDefProvider`, while a pure Sink connector only needs `SinkDefProvider`.

## 2. Sink Runtime Interfaces

### 2.1 Core Traits

- `AsyncCtrl` – runtime control.
  - `stop(&mut self)`: idempotent shutdown; stop all tasks and release resources.
  - `reconnect(&mut self)`: rebuild connections or reset state without changing external semantics.
- `AsyncRecordSink` – structured records.
  - `sink_record(&mut self, &DataRecord)`: single record.
  - `sink_records(&mut self, Vec<Arc<DataRecord>>)`: batch write while preserving order.
- `AsyncRawDataSink` – raw text/bytes.
  - `sink_str` / `sink_bytes`: single payload.
  - `sink_str_batch` / `sink_bytes_batch`: batch payloads.
- `AsyncSink` – blanket impl combining `AsyncCtrl + AsyncRecordSink + AsyncRawDataSink + Send + Sync`. Implement `impl AsyncSink for MySink {}` so the orchestrator can treat your sink uniformly.

### 2.2 Build Pipeline

- `SinkBuildCtx`
  - `work_root: PathBuf`: sandbox directory per instance.
  - `replica_idx/replica_cnt`: zero-based replica index and total replicas (>= 1).
  - `rate_limit_rps`: upstream hint for rate limiting.
- `ResolvedSinkSpec`
  - `group/name/kind/connector_id`: identifiers.
  - `params: ParamMap`: flattened runtime params.
  - `filter: Option<String>`: optional filter string; semantics depend on the caller.
- `SinkFactory`
  - `kind()`: registry name.
  - `validate_spec()`: optional lightweight validation (defaults to no-op).
  - `build(spec, ctx) -> SinkHandle`: construct `Box<dyn AsyncSink>`; propagate `anyhow::Error` on failure.

## 3. Source Runtime Interfaces

### 3.1 `DataSource` Behavior

- `receive(&mut self) -> SourceResult<SourceBatch>`: the only mandatory method. A batch may be empty to indicate “no data yet”.
- `try_receive(&mut self) -> Option<SourceBatch>`: only call when `supports_try_receive()` and `can_try_receive()` both return true; otherwise return `None`.
- `supports_try_receive(&self)`: static capability (default `false`).
- `can_try_receive(&mut self)`: dynamic capability (defaults to the static value).
- `identifier()` / `identifier_ref()` for logs/metrics. `identifier_ref()` falls back to `String`; override it to avoid allocations.
- Lifecycle hooks:
  - `start(&mut self, CtrlRx)`: prepare resources and start listening to control events (default no-op).
  - `close(&mut self)`: idempotent shutdown.
- Optional capabilities:
  - `caps(&self) -> SourceCaps`: advertise `ack` / `seek` / `parallel` support.
  - `ack(&mut self, Arc<dyn AckToken>)`: default `SupplierError("ack unsupported")`.
  - `seek(&mut self, Arc<dyn SeekPosition>)`: default `SupplierError("seek unsupported")`.

### 3.2 Events and Control

- `SourceEvent`
  - Fields: `event_id`, `src_key`, `payload: RawData`, `tags: Arc<Tags>`, `ups_ip`, `preproc`. `payload` accepts `String`, `Bytes`, or `Arc<Vec<u8>>`; debug output summarizes lengths.
- `ControlEvent`
  - `Stop`: request immediate stop.
  - `Isolate(bool)`: pause (`true`) or resume (`false`).
  - `Seek(Arc<dyn SeekPosition>)`: seek to a position.
- `CtrlRx = async_broadcast::Receiver<ControlEvent>`: listen inside `start()` for orchestrator commands.
- `Tags`: sorted `SmallVec` with `set/get/is_empty` helpers; unit tests guarantee deterministic order.

### 3.3 `SourceFactory` Pipeline

- `SourceBuildCtx { work_root }`: provides per-instance workspace similar to sinks.
- `SourceMeta { name, kind, tags }`: metadata for UI/monitoring.
- `SourceHandle { source, metadata }`: a pull-based instance.
- `AcceptorHandle { name, acceptor }`: server-side listener (HTTP, gRPC, ...).
- `SourceSvcIns { sources, acceptor }`: return value of `SourceFactory::build`, allowing multiple sources plus an optional acceptor.
- `ResolvedSourceSpec`: fields `name`, `kind`, `connector_id`, `params: ParamMap`, `tags: Vec<String>`. Factories must implement `kind()`, optional `validate_spec()`, and `build(spec, ctx)`.

## 4. Error Model

- **Sink**: `SinkReason` / `SinkError` wrap `orion_error::StructError`. Use `SinkReason::sink(ctx)` and the `SinkErrorOwe` helper (`some_call().owe_sink("context")?`) to annotate external failures.
  - Error code mapping:
    - `Sink(String)`: 500 — General sink unavailable
    - `Mock`: 599 — Mock/test error
    - `StgCtrl`: 510 — Storage control error
    - `Uvs(UvsReason)`: Delegates to inner UvsReason's error code
- **Source**: `SourceReason` / `SourceError` mirror the sink side.
  - Error code mapping:
    - `NotData`: 100 — Temporary no data available (normal)
    - `EOF`: 101 — End of data stream (normal)
    - `Disconnect(String)`: 503 — Connection lost (retryable)
    - `SupplierError(String)`: 500 — Upstream supplier error
    - `Other(String)`: 520 — Unclassified error
    - `Uvs(UvsReason)`: Delegates to inner UvsReason's error code
  - `SourceResult<T>` is an alias for `Result<T, StructError<SourceReason>>`.

## 5. Example: In-Memory Connector

Below is a complete in-memory connector example implementing both Source and Sink. Use it as a reference template when developing new connectors.

### 5.1 MemorySource Implementation

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
            return Ok(vec![]); // No more data
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

### 5.2 MemorySink Implementation

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

### 5.3 ConnectorFactory Implementation

```rust
use async_trait::async_trait;
use wp_connector_api::{
    ConnectorDef, ConnectorScope, SinkDefProvider, SourceDefProvider,
    SinkFactory, SinkHandle, SinkBuildCtx, SinkSpec, SinkResult,
    SourceFactory, SourceHandle, SourceMeta, SourceBuildCtx,
    SourceSpec, SourceResult, SourceSvcIns,
};

struct DemoConnectorFactory {
    sink_buffer: MemorySinkBuffer,
    source_events: Vec<String>,
}

// Implement SourceDefProvider as needed
impl SourceDefProvider for DemoConnectorFactory {
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
}

// Implement SinkDefProvider as needed
impl SinkDefProvider for DemoConnectorFactory {
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

See `wp-connector-api/tests/demo_connector.rs` for the complete working example.

## 6. Practical Tips

1. **Validate early**: run parameter checks in `ConnectorKindAdapter::url_to_params` and `SinkFactory::validate_spec` to catch issues before runtime.
2. **Enforce idempotency**: `stop`, `receive`, `ack`, etc. must be safe to retry. Never `unwrap/expect`; propagate errors via `SinkResult`/`SourceResult`.
3. **Prioritize zero-copy**: `SourceEvent.payload` can be `RawData::ArcBytes`. Provide `Arc<Vec<u8>>` so `into_bytes()` can attempt `Arc::try_unwrap` and reuse buffers when the refcount is 1.
4. **Test ParamMap conversions**: `config::param` already has unit tests; extend them when adding more complex structures.
5. **Keep docs in sync**: whenever you add APIs or change semantics, update both the Chinese (docs/zh) and English (docs/en) versions.

Questions? Mention the WarpParse Dev Team in your PR or issue.
