use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use smol_str::SmolStr;
use std::borrow::Cow;
use std::sync::Arc;

use super::event::SourceBatch;
use crate::{SourceReason, SourceResult};

/// Capability flags for data sources.
///
/// Used to advertise what optional features a source supports.
#[derive(Clone, Copy, Debug, Default)]
pub struct SourceCaps {
    /// Whether the source supports acknowledgment (commit) of consumed messages
    pub ack: bool,
    /// Whether the source supports seeking to arbitrary positions
    pub seek: bool,
    /// Whether the source supports parallel consumption
    pub parallel: bool,
}

/// Marker trait for acknowledgment tokens.
///
/// Implementors represent a position or state that can be acknowledged
/// to the upstream source (e.g., Kafka offset, SQS receipt handle).
pub trait AckToken: Send + Sync + std::fmt::Debug {}

/// Marker trait for seek positions.
///
/// Implementors represent a position that the source can seek to
/// (e.g., timestamp, offset, sequence number).
pub trait SeekPosition: Send + Sync + std::fmt::Debug {}

/// Control events for managing data source lifecycle.
///
/// These events are sent through [`CtrlRx`] to control source behavior
/// at runtime. Sources should handle these events in their `start()` method.
#[derive(Debug, Clone)]
pub enum ControlEvent {
    /// Request immediate stop. Source should exit its receive loop promptly.
    Stop,
    /// Toggle isolation mode. `true` = pause output, `false` = resume.
    Isolate(bool),
    /// Request to seek to a specific position.
    Seek(Arc<dyn SeekPosition>),
}

/// Control channel receiver type.
///
/// Based on `async_broadcast::Receiver<ControlEvent>`. Sources receive
/// this in their `start()` method to listen for control commands.
pub type CtrlRx = async_broadcast::Receiver<ControlEvent>;

/// Core trait for data sources.
///
/// A data source produces batches of events that can be consumed by the pipeline.
/// Implementations must be `Send + Sync` to allow concurrent access.
///
/// # Lifecycle
/// 1. Create the source instance
/// 2. Call `start(ctrl_rx)` to initialize and begin listening for control events
/// 3. Call `receive()` repeatedly to pull data batches
/// 4. Call `close()` when done to release resources
///
/// # Example
/// ```ignore
/// #[async_trait]
/// impl DataSource for MySource {
///     async fn receive(&mut self) -> SourceResult<SourceBatch> {
///         // Pull data from upstream
///         let data = self.client.poll().await?;
///         Ok(vec![SourceEvent::new(0, "my-source", data, Arc::new(Tags::new()))])
///     }
///
///     fn try_receive(&mut self) -> Option<SourceBatch> { None }
///     fn identifier(&self) -> String { "my-source".into() }
/// }
/// ```
#[async_trait]
pub trait DataSource: Send + Sync {
    /// Pull the next batch of events from the source.
    ///
    /// Returns a vector of [`SourceEvent`]s. An empty vector indicates no data
    /// is currently available (not EOF). Use error variants like `SourceReason::EOF`
    /// to signal end of stream.
    ///
    /// This method should be idempotent and safe to call repeatedly after `start()`.
    async fn receive(&mut self) -> SourceResult<SourceBatch>;

    /// Non-blocking attempt to receive data.
    ///
    /// Returns `Some(batch)` if data is immediately available, `None` otherwise.
    /// Only call this when `can_try_receive()` returns true.
    fn try_receive(&mut self) -> Option<SourceBatch>;

    /// Static capability: whether this source ever supports non-blocking receive.
    fn supports_try_receive(&self) -> bool {
        false
    }

    /// Dynamic capability: whether non-blocking receive is safe right now.
    fn can_try_receive(&mut self) -> bool {
        self.supports_try_receive()
    }

    /// Returns a unique identifier for this source instance (for logging/metrics).
    fn identifier(&self) -> String;

    /// Zero-allocation identifier access. Defaults to allocating via `identifier()`.
    fn identifier_ref(&self) -> Cow<'_, str> {
        Cow::Owned(self.identifier())
    }

    /// Returns capability flags for this source.
    fn caps(&self) -> SourceCaps {
        SourceCaps::default()
    }

    /// Initialize the source and start listening for control events.
    ///
    /// Must be called before `receive()`. The `ctrl_rx` channel delivers
    /// [`ControlEvent`]s for lifecycle management (stop, isolate, seek).
    async fn start(&mut self, _ctrl_rx: CtrlRx) -> SourceResult<()> {
        Ok(())
    }

    /// Stop the source and release all resources.
    ///
    /// Must be idempotent - safe to call multiple times or before `start()`.
    async fn close(&mut self) -> SourceResult<()> {
        Ok(())
    }

    /// Acknowledge consumption of events up to the given token.
    ///
    /// Only supported when `caps().ack == true`. Returns error by default.
    async fn ack(&mut self, _token: Arc<dyn AckToken>) -> SourceResult<()> {
        Err(SourceReason::SupplierError("ack unsupported".into()).into())
    }

    /// Seek to a specific position in the source.
    ///
    /// Only supported when `caps().seek == true`. Returns error by default.
    async fn seek(&mut self, _pos: Arc<dyn SeekPosition>) -> SourceResult<()> {
        Err(SourceReason::SupplierError("seek unsupported".into()).into())
    }
}

const INLINE_TAG_CAPACITY: usize = 16;

/// A lightweight, sorted collection of key-value string tags.
///
/// Tags are stored in a `SmallVec` with inline capacity for up to 16 entries,
/// avoiding heap allocation for typical use cases. Keys are kept sorted
/// for efficient binary search lookup.
///
/// Both keys and values use `SmolStr` for small string optimization,
/// providing zero-allocation storage for strings ≤22 bytes.
///
/// # Example
/// ```ignore
/// let mut tags = Tags::new();
/// tags.set("env", "prod");
/// tags.set("region", "us-west");
///
/// assert_eq!(tags.get("env"), Some("prod"));
/// assert!(tags.contains_key("region"));
/// ```
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Tags {
    item: SmallVec<[(SmolStr, SmolStr); INLINE_TAG_CAPACITY]>,
}

impl Tags {
    /// Create a new empty Tags collection.
    pub fn new() -> Self {
        Self {
            item: SmallVec::new(),
        }
    }

    /// Set a tag value. If the key exists, the value is updated.
    ///
    /// Keys are kept sorted for efficient lookup.
    pub fn set(&mut self, key: impl Into<SmolStr>, value: impl Into<SmolStr>) {
        let key = key.into();
        let value = value.into();
        match self
            .item
            .binary_search_by(|(existing, _)| existing.as_str().cmp(key.as_str()))
        {
            Ok(idx) => {
                self.item[idx].1 = value;
            }
            Err(idx) => {
                self.item.insert(idx, (key, value));
            }
        }
    }

    /// Get a tag value by key.
    ///
    /// Returns `None` if the key doesn't exist.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.item
            .binary_search_by(|(existing, _)| existing.as_str().cmp(key))
            .ok()
            .and_then(|idx| self.item.get(idx))
            .map(|(_, val)| val.as_str())
    }

    /// Check if a key exists in the tags.
    pub fn contains_key(&self, key: &str) -> bool {
        self.item
            .binary_search_by(|(existing, _)| existing.as_str().cmp(key))
            .is_ok()
    }

    /// Remove a tag by key.
    ///
    /// Returns the removed value if the key existed.
    pub fn remove(&mut self, key: &str) -> Option<SmolStr> {
        match self
            .item
            .binary_search_by(|(existing, _)| existing.as_str().cmp(key))
        {
            Ok(idx) => Some(self.item.remove(idx).1),
            Err(_) => None,
        }
    }

    /// Check if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.item.is_empty()
    }

    /// Return the number of tags.
    pub fn len(&self) -> usize {
        self.item.len()
    }

    /// Iterate over all key-value pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.item.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Iterate over all keys.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.item.iter().map(|(k, _)| k.as_str())
    }

    /// Iterate over all values.
    pub fn values(&self) -> impl Iterator<Item = &str> {
        self.item.iter().map(|(_, v)| v.as_str())
    }

    /// Clear all tags.
    pub fn clear(&mut self) {
        self.item.clear();
    }
}

// Deprecated compatibility alias
impl Tags {
    /// Set a tag value.
    #[deprecated(since = "0.6.0", note = "Use `set()` instead")]
    pub fn set_tag(&mut self, key: &str, value: String) {
        self.set(SmolStr::from(key), SmolStr::from(value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    // ========== Tags tests ==========

    #[test]
    fn tags_keep_sorted_insert_and_update() {
        let mut tags = Tags::new();
        tags.set("beta", "2");
        tags.set("alpha", "1");
        tags.set("gamma", "3");

        assert_eq!(tags.len(), 3);
        assert_eq!(tags.get("alpha"), Some("1"));
        assert_eq!(tags.get("beta"), Some("2"));
        assert_eq!(tags.get("gamma"), Some("3"));

        // 更新后应保持顺序并覆盖值
        tags.set("beta", "22");
        assert_eq!(tags.len(), 3);
        assert_eq!(tags.get("beta"), Some("22"));
    }

    #[test]
    fn tags_helpers_cover_empty_and_missing() {
        let mut tags = Tags::new();
        assert!(tags.is_empty());
        assert_eq!(tags.get("missing"), None);

        tags.set("key", "value");
        assert!(!tags.is_empty());
        assert_eq!(tags.get("key"), Some("value"));
    }

    #[test]
    fn tags_contains_key() {
        let mut tags = Tags::new();
        tags.set("exists", "value");

        assert!(tags.contains_key("exists"));
        assert!(!tags.contains_key("missing"));
    }

    #[test]
    fn tags_remove() {
        let mut tags = Tags::new();
        tags.set("a", "1");
        tags.set("b", "2");
        tags.set("c", "3");

        // Remove existing key
        let removed = tags.remove("b");
        assert_eq!(removed, Some(SmolStr::from("2")));
        assert_eq!(tags.len(), 2);
        assert!(!tags.contains_key("b"));

        // Remove non-existing key
        let not_found = tags.remove("missing");
        assert_eq!(not_found, None);
        assert_eq!(tags.len(), 2);
    }

    #[test]
    fn tags_iter() {
        let mut tags = Tags::new();
        tags.set("b", "2");
        tags.set("a", "1");
        tags.set("c", "3");

        let pairs: Vec<_> = tags.iter().collect();
        // Should be sorted by key
        assert_eq!(pairs, vec![("a", "1"), ("b", "2"), ("c", "3")]);
    }

    #[test]
    fn tags_keys_and_values() {
        let mut tags = Tags::new();
        tags.set("x", "1");
        tags.set("y", "2");

        let keys: Vec<_> = tags.keys().collect();
        let values: Vec<_> = tags.values().collect();

        assert_eq!(keys, vec!["x", "y"]);
        assert_eq!(values, vec!["1", "2"]);
    }

    #[test]
    fn tags_clear() {
        let mut tags = Tags::new();
        tags.set("a", "1");
        tags.set("b", "2");
        assert_eq!(tags.len(), 2);

        tags.clear();
        assert!(tags.is_empty());
        assert_eq!(tags.len(), 0);
    }

    #[test]
    #[allow(deprecated)]
    fn tags_deprecated_set_tag_still_works() {
        let mut tags = Tags::new();
        tags.set_tag("key", "value".to_string());
        assert_eq!(tags.get("key"), Some("value"));
    }

    // ========== ControlEvent tests ==========

    #[test]
    fn control_event_stop_is_cloneable() {
        let event = ControlEvent::Stop;
        let cloned = event.clone();
        assert!(matches!(cloned, ControlEvent::Stop));
    }

    #[test]
    fn control_event_isolate_carries_state() {
        let pause = ControlEvent::Isolate(true);
        let resume = ControlEvent::Isolate(false);

        if let ControlEvent::Isolate(state) = pause {
            assert!(state);
        } else {
            panic!("expected Isolate");
        }

        if let ControlEvent::Isolate(state) = resume {
            assert!(!state);
        } else {
            panic!("expected Isolate");
        }
    }

    #[derive(Debug)]
    #[allow(dead_code)]
    struct TestSeekPos(u64);
    impl SeekPosition for TestSeekPos {}

    #[test]
    fn control_event_seek_carries_position() {
        let pos = Arc::new(TestSeekPos(100));
        let event = ControlEvent::Seek(pos.clone());

        if let ControlEvent::Seek(p) = event {
            // Verify the Arc points to the same data
            assert!(Arc::ptr_eq(&p, &(pos as Arc<dyn SeekPosition>)));
        } else {
            panic!("expected Seek");
        }
    }

    // ========== SourceCaps tests ==========

    #[test]
    fn source_caps_default_all_false() {
        let caps = SourceCaps::default();
        assert!(!caps.ack);
        assert!(!caps.seek);
        assert!(!caps.parallel);
    }

    #[test]
    fn source_caps_can_be_customized() {
        let caps = SourceCaps {
            ack: true,
            seek: true,
            parallel: false,
        };
        assert!(caps.ack);
        assert!(caps.seek);
        assert!(!caps.parallel);
    }

    // ========== DataSource lifecycle tests ==========

    struct LifecycleTrackingSource {
        started: AtomicBool,
        closed: AtomicBool,
        receive_count: AtomicUsize,
    }

    impl LifecycleTrackingSource {
        fn new() -> Self {
            Self {
                started: AtomicBool::new(false),
                closed: AtomicBool::new(false),
                receive_count: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl DataSource for LifecycleTrackingSource {
        async fn receive(&mut self) -> SourceResult<SourceBatch> {
            self.receive_count.fetch_add(1, Ordering::SeqCst);
            Ok(vec![])
        }

        fn try_receive(&mut self) -> Option<SourceBatch> {
            None
        }

        fn identifier(&self) -> String {
            "lifecycle-tracker".into()
        }

        async fn start(&mut self, _ctrl_rx: CtrlRx) -> SourceResult<()> {
            self.started.store(true, Ordering::SeqCst);
            Ok(())
        }

        async fn close(&mut self) -> SourceResult<()> {
            self.closed.store(true, Ordering::SeqCst);
            Ok(())
        }
    }

    #[tokio::test]
    async fn datasource_lifecycle_start_receive_close() {
        let mut source = LifecycleTrackingSource::new();

        // Initially not started
        assert!(!source.started.load(Ordering::SeqCst));
        assert!(!source.closed.load(Ordering::SeqCst));

        // Start the source
        let (tx, rx) = async_broadcast::broadcast(16);
        drop(tx); // We won't send any events in this test
        source.start(rx).await.unwrap();
        assert!(source.started.load(Ordering::SeqCst));

        // Receive some batches
        for _ in 0..3 {
            let _ = source.receive().await.unwrap();
        }
        assert_eq!(source.receive_count.load(Ordering::SeqCst), 3);

        // Close the source
        source.close().await.unwrap();
        assert!(source.closed.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn datasource_close_is_idempotent() {
        let mut source = LifecycleTrackingSource::new();

        // Close can be called multiple times safely
        source.close().await.unwrap();
        source.close().await.unwrap();
        assert!(source.closed.load(Ordering::SeqCst));
    }

    // ========== DataSource with ControlEvent handling ==========

    struct ControlAwareSource {
        stopped: AtomicBool,
        isolated: AtomicBool,
    }

    impl ControlAwareSource {
        fn new() -> Self {
            Self {
                stopped: AtomicBool::new(false),
                isolated: AtomicBool::new(false),
            }
        }
    }

    #[async_trait]
    impl DataSource for ControlAwareSource {
        async fn receive(&mut self) -> SourceResult<SourceBatch> {
            if self.stopped.load(Ordering::SeqCst) {
                return Err(SourceReason::EOF.into());
            }
            if self.isolated.load(Ordering::SeqCst) {
                return Ok(vec![]); // Return empty when isolated
            }
            Ok(vec![])
        }

        fn try_receive(&mut self) -> Option<SourceBatch> {
            None
        }

        fn identifier(&self) -> String {
            "control-aware".into()
        }

        async fn start(&mut self, mut ctrl_rx: CtrlRx) -> SourceResult<()> {
            // Spawn a task to handle control events (simplified for test)
            let stopped = &self.stopped;
            let isolated = &self.isolated;

            // Process any immediately available events
            while let Ok(event) = ctrl_rx.try_recv() {
                match event {
                    ControlEvent::Stop => {
                        stopped.store(true, Ordering::SeqCst);
                    }
                    ControlEvent::Isolate(state) => {
                        isolated.store(state, Ordering::SeqCst);
                    }
                    ControlEvent::Seek(_) => {}
                }
            }
            Ok(())
        }
    }

    #[tokio::test]
    async fn datasource_responds_to_stop_event() {
        let mut source = ControlAwareSource::new();

        let (tx, rx) = async_broadcast::broadcast(16);

        // Send stop event before start
        tx.broadcast(ControlEvent::Stop).await.unwrap();

        source.start(rx).await.unwrap();

        // Source should be stopped
        assert!(source.stopped.load(Ordering::SeqCst));

        // receive should return EOF
        let result = source.receive().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn datasource_responds_to_isolate_event() {
        let mut source = ControlAwareSource::new();

        let (tx, rx) = async_broadcast::broadcast(16);

        // Send isolate event
        tx.broadcast(ControlEvent::Isolate(true)).await.unwrap();

        source.start(rx).await.unwrap();

        // Source should be isolated
        assert!(source.isolated.load(Ordering::SeqCst));
    }

    // ========== AckToken and SeekPosition tests ==========

    #[derive(Debug)]
    #[allow(dead_code)]
    struct TestAckToken(String);
    impl AckToken for TestAckToken {}

    #[tokio::test]
    async fn datasource_ack_unsupported_by_default() {
        let mut source = LifecycleTrackingSource::new();
        let token: Arc<dyn AckToken> = Arc::new(TestAckToken("test".into()));

        let result = source.ack(token).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn datasource_seek_unsupported_by_default() {
        let mut source = LifecycleTrackingSource::new();
        let pos: Arc<dyn SeekPosition> = Arc::new(TestSeekPos(0));

        let result = source.seek(pos).await;
        assert!(result.is_err());
    }
}
