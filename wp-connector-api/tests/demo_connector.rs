//! Demo connector integration tests
//!
//! This module provides a minimal in-memory connector implementation
//! for both Source and Sink, demonstrating how to build connectors
//! using the wp-connector-api traits.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use wp_connector_api::{
    AsyncCtrl, AsyncRawDataSink, AsyncRecordSink, ConnectorDef, ConnectorScope, DataSource,
    SinkBuildCtx, SinkDefProvider, SinkFactory, SinkHandle, SinkResult, SinkSpec, SourceBatch,
    SourceBuildCtx, SourceDefProvider, SourceEvent, SourceFactory, SourceHandle, SourceMeta,
    SourceResult, SourceSpec, SourceSvcIns, Tags,
};
use wp_model_core::model::DataRecord;
use wp_parse_api::RawData;

// ============================================================================
// Demo Source: produces configurable events from memory
// ============================================================================

struct MemorySource {
    name: String,
    events: Vec<String>,
    cursor: usize,
}

impl MemorySource {
    fn new(name: impl Into<String>, events: Vec<String>) -> Self {
        Self {
            name: name.into(),
            events,
            cursor: 0,
        }
    }
}

#[async_trait]
impl DataSource for MemorySource {
    async fn receive(&mut self) -> SourceResult<SourceBatch> {
        if self.cursor >= self.events.len() {
            return Ok(vec![]);
        }

        let event = SourceEvent::new(
            self.cursor as u64,
            self.name.as_str(),
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
                self.name.as_str(),
                RawData::from_string(&self.events[self.cursor]),
                Arc::new(Tags::default()),
            );
            self.cursor += 1;
            Some(vec![event])
        } else {
            None
        }
    }

    fn supports_try_receive(&self) -> bool {
        true
    }

    fn identifier(&self) -> String {
        format!("memory-source:{}", self.name)
    }
}

// ============================================================================
// Demo Sink: collects data into shared memory buffer
// ============================================================================

#[derive(Clone, Default)]
struct MemorySinkBuffer {
    data: Arc<Mutex<Vec<String>>>,
}

impl MemorySinkBuffer {
    fn new() -> Self {
        Self::default()
    }

    fn push(&self, s: String) {
        self.data.lock().unwrap().push(s);
    }

    fn snapshot(&self) -> Vec<String> {
        self.data.lock().unwrap().clone()
    }
}

struct MemorySink {
    buffer: MemorySinkBuffer,
}

impl MemorySink {
    fn new(buffer: MemorySinkBuffer) -> Self {
        Self { buffer }
    }
}

#[async_trait]
impl AsyncCtrl for MemorySink {
    async fn stop(&mut self) -> SinkResult<()> {
        Ok(())
    }

    async fn reconnect(&mut self) -> SinkResult<()> {
        Ok(())
    }
}

#[async_trait]
impl AsyncRecordSink for MemorySink {
    async fn sink_record(&mut self, _data: &DataRecord) -> SinkResult<()> {
        self.buffer.push("record".to_string());
        Ok(())
    }

    async fn sink_records(&mut self, data: Vec<Arc<DataRecord>>) -> SinkResult<()> {
        for _ in data {
            self.buffer.push("record".to_string());
        }
        Ok(())
    }
}

#[async_trait]
impl AsyncRawDataSink for MemorySink {
    async fn sink_str(&mut self, data: &str) -> SinkResult<()> {
        self.buffer.push(data.to_string());
        Ok(())
    }

    async fn sink_bytes(&mut self, data: &[u8]) -> SinkResult<()> {
        self.buffer.push(String::from_utf8_lossy(data).to_string());
        Ok(())
    }

    async fn sink_str_batch(&mut self, data: Vec<&str>) -> SinkResult<()> {
        for s in data {
            self.buffer.push(s.to_string());
        }
        Ok(())
    }

    async fn sink_bytes_batch(&mut self, data: Vec<&[u8]>) -> SinkResult<()> {
        for b in data {
            self.buffer.push(String::from_utf8_lossy(b).to_string());
        }
        Ok(())
    }
}

// ============================================================================
// Demo Connector Factory
// ============================================================================

struct DemoConnectorFactory {
    sink_buffer: MemorySinkBuffer,
    source_events: Vec<String>,
}

impl DemoConnectorFactory {
    fn new(source_events: Vec<String>) -> Self {
        Self {
            sink_buffer: MemorySinkBuffer::new(),
            source_events,
        }
    }

    fn sink_buffer(&self) -> MemorySinkBuffer {
        self.sink_buffer.clone()
    }
}

impl SourceDefProvider for DemoConnectorFactory {
    fn source_def(&self) -> ConnectorDef {
        ConnectorDef {
            id: "demo-source".into(),
            kind: "memory".into(),
            scope: ConnectorScope::Source,
            allow_override: vec!["events".into()],
            default_params: Default::default(),
            origin: Some("test".into()),
        }
    }
}

impl SinkDefProvider for DemoConnectorFactory {
    fn sink_def(&self) -> ConnectorDef {
        ConnectorDef {
            id: "demo-sink".into(),
            kind: "memory".into(),
            scope: ConnectorScope::Sink,
            allow_override: vec![],
            default_params: Default::default(),
            origin: Some("test".into()),
        }
    }
}

#[async_trait]
impl SourceFactory for DemoConnectorFactory {
    fn kind(&self) -> &'static str {
        "memory"
    }

    async fn build(&self, spec: &SourceSpec, _ctx: &SourceBuildCtx) -> SourceResult<SourceSvcIns> {
        let source = MemorySource::new(&spec.name, self.source_events.clone());
        let handle = SourceHandle::new(
            Box::new(source),
            SourceMeta::new(&spec.name, SourceFactory::kind(self)),
        );
        Ok(SourceSvcIns::new().with_sources(vec![handle]))
    }
}

#[async_trait]
impl SinkFactory for DemoConnectorFactory {
    fn kind(&self) -> &'static str {
        "memory"
    }

    async fn build(&self, _spec: &SinkSpec, _ctx: &SinkBuildCtx) -> SinkResult<SinkHandle> {
        let sink = MemorySink::new(self.sink_buffer.clone());
        Ok(SinkHandle::new(Box::new(sink)))
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test]
async fn test_source_produces_events_and_sink_collects() {
    // Setup factory with test events
    let events = vec!["event-1".into(), "event-2".into(), "event-3".into()];
    let factory = DemoConnectorFactory::new(events.clone());

    // Build source
    let source_spec = SourceSpec {
        name: "test-source".into(),
        kind: "memory".into(),
        connector_id: "demo".into(),
        params: Default::default(),
        tags: vec![],
    };
    let ctx = SourceBuildCtx::new(PathBuf::from("/tmp/test"));
    let mut svc = SourceFactory::build(&factory, &source_spec, &ctx)
        .await
        .unwrap();

    assert_eq!(svc.sources.len(), 1);
    let source = &mut svc.sources[0];

    // Consume all events from source
    let mut received = vec![];
    for _ in 0..3 {
        let batch = source.source.receive().await.unwrap();
        for event in batch {
            if let RawData::String(s) = event.payload {
                received.push(s);
            }
        }
    }
    assert_eq!(received, events);

    // Build sink and write data
    let sink_spec = SinkSpec {
        group: "default".into(),
        name: "test-sink".into(),
        kind: "memory".into(),
        connector_id: "demo".into(),
        params: Default::default(),
        filter: None,
    };
    let sink_ctx = SinkBuildCtx::new(PathBuf::from("/tmp/test"));
    let mut sink_handle = SinkFactory::build(&factory, &sink_spec, &sink_ctx)
        .await
        .unwrap();

    // Write received data to sink
    for data in &received {
        sink_handle.sink.sink_str(data).await.unwrap();
    }

    // Verify sink buffer contains all events
    let buffer = factory.sink_buffer();
    assert_eq!(buffer.snapshot(), events);
}

#[tokio::test]
async fn test_def_providers_return_correct_metadata() {
    let factory = DemoConnectorFactory::new(vec![]);

    // Test SourceDefProvider
    let source_def = factory.source_def();
    assert_eq!(source_def.id, "demo-source");
    assert_eq!(source_def.kind, "memory");
    assert_eq!(source_def.scope, ConnectorScope::Source);

    // Test SinkDefProvider
    let sink_def = factory.sink_def();
    assert_eq!(sink_def.id, "demo-sink");
    assert_eq!(sink_def.kind, "memory");
    assert_eq!(sink_def.scope, ConnectorScope::Sink);
}
