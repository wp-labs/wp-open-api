# `wp-connector-api` Developer Guide

This guide mirrors the current `wp-connector-api` implementation. It aggregates the configuration adapters, Sink/Source runtime traits, error model, and practical advice so you can build or debug connectors quickly.

## 1. Configuration-Time Interfaces

- **ParamMap**: unified parameter container (`BTreeMap<String, serde_json::Value>`). Use the helpers in `config::param` (`parammap_from_toml_table/map`) to flatten TOML tables and keep keys sorted for stable diffs.
- **ConnectorKindAdapter**: maps human-friendly inputs (such as `conn_url`) into a ParamMap. Implementors must provide:
  - `kind(&self) -> &'static str`: unique identifier.
  - `defaults(&self) -> ParamMap`: connector-specific defaults; returning `ParamMap::new()` means “no defaults”.
  - `url_to_params(&self, url: &str) -> anyhow::Result<ParamMap>`: parse the URL; return an error when encountering unsupported formats.

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

- **Sink**: `SinkReason` / `SinkError` wrap `orion_error::StructError`. Use `SinkReason::sink(ctx)` and the `SinkErrorOwe` helper (`some_call().owe_sink("context")?`) to annotate external failures. Variants include `Sink(String)`, `Mock`, `StgCtrl`, `Uvs`, all sharing `ErrorCode = 255`.
- **Source**: `SourceReason` / `SourceError` mirror the sink side with variants such as `NotData`, `EOF`, `SupplierError(String)`. `SourceResult<T>` is an alias for `Result<T, StructError<SourceReason>>`.

## 5. Practical Tips

1. **Validate early**: run parameter checks in `ConnectorKindAdapter::url_to_params` and `SinkFactory::validate_spec` to catch issues before runtime.
2. **Enforce idempotency**: `stop`, `receive`, `ack`, etc. must be safe to retry. Never `unwrap/expect`; propagate errors via `SinkResult`/`SourceResult`.
3. **Prioritize zero-copy**: `SourceEvent.payload` can be `RawData::ArcBytes`. Provide `Arc<Vec<u8>>` so `into_bytes()` can attempt `Arc::try_unwrap` and reuse buffers when the refcount is 1.
4. **Test ParamMap conversions**: `config::param` already has unit tests; extend them when adding more complex structures.
5. **Keep docs in sync**: whenever you add APIs or change semantics, update both the Chinese (docs/zh) and English (docs/en) versions.

Questions? Mention the WarpParse Dev Team in your PR or issue.
