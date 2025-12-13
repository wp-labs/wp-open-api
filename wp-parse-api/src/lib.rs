//! Minimal Plugin API for WPL parsers.
//! This crate defines core types for plugin development while maintaining
//! compatibility with the wp-lang ecosystem.

use std::fmt::{Display, Formatter};
use std::sync::Arc;

use bytes::Bytes;
use wp_data_model::model::DataRecord;

mod error;
pub use error::{WparseError, WparseReason, WparseResult};
#[allow(deprecated)]
pub use error::{WplParseError, WplParseReason, WplParseResult};
// Re-export necessary types from wp-lang that we still need

/// Result type for plugin parsing operations.
///
/// On success, returns a tuple of `(DataRecord, remaining_raw)`.
/// On failure, returns a WparseError (旧名称 `WplParseError` 仍可用，但已弃用)。
pub type DataResult = Result<(DataRecord, RawData), WparseError>;

#[derive(Debug, Clone)]
pub enum RawData {
    String(String),
    Bytes(Bytes),
    ArcBytes(Arc<Vec<u8>>),
}

impl RawData {
    pub fn from_string<T: Into<String>>(value: T) -> RawData {
        RawData::String(value.into())
    }

    pub fn from_arc_bytes(data: Arc<Vec<u8>>) -> Self {
        RawData::ArcBytes(data)
    }

    /// 辅助构造：从 `Arc<[u8]>` 构建。该接口用于兼容旧版（0.4.6 之前）`ArcBytes` 表示，
    /// 会额外复制一次数据，建议尽快迁移到 `Arc<Vec<u8>>`。
    pub fn from_arc_slice(data: Arc<[u8]>) -> Self {
        RawData::ArcBytes(Arc::new(data.as_ref().to_vec()))
    }

    // 统一的数据访问接口
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            RawData::String(s) => s.as_bytes(),
            RawData::Bytes(b) => b.as_ref(),
            RawData::ArcBytes(arc) => arc.as_slice(),
        }
    }

    // 向后兼容的 Bytes 转换（仅在需要时，始终复制）
    pub fn to_bytes(&self) -> Bytes {
        match self {
            RawData::String(s) => Bytes::copy_from_slice(s.as_bytes()),
            RawData::Bytes(b) => b.clone(),
            RawData::ArcBytes(arc) => Bytes::copy_from_slice(arc.as_slice()),
        }
    }

    /// 按需取得 Bytes，消耗自身以在 `String`/`Bytes` 分支复用缓冲区。
    pub fn into_bytes(self) -> Bytes {
        match self {
            RawData::String(s) => Bytes::from(s),
            RawData::Bytes(b) => b,
            RawData::ArcBytes(arc) => match Arc::try_unwrap(arc) {
                Ok(vec) => Bytes::from(vec),
                Err(shared) => Bytes::copy_from_slice(shared.as_slice()),
            },
        }
    }

    // 零拷贝检测
    pub fn is_zero_copy(&self) -> bool {
        matches!(self, RawData::ArcBytes(_))
    }

    pub fn len(&self) -> usize {
        self.as_bytes().len()
    }

    pub fn is_empty(&self) -> bool {
        match self {
            RawData::String(value) => value.is_empty(),
            RawData::Bytes(value) => value.is_empty(),
            RawData::ArcBytes(arc) => arc.is_empty(),
        }
    }
}

impl Display for RawData {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            RawData::String(value) => f.write_str(value),
            // 安全转换：尽量显示为 UTF-8；不可解码时使用替代字符
            RawData::Bytes(value) => f.write_str(&String::from_utf8_lossy(value)),
            RawData::ArcBytes(arc) => f.write_str(&String::from_utf8_lossy(arc.as_slice())),
        }
    }
}

/// Trait for pipeline data processing operations.
///
/// This trait defines the interface for components that process RawData
/// within a data pipeline, transforming it from one format to another
/// (e.g., base64 decoding, hex decoding, string unescaping, etc.).
///
/// Pipeline processors are executed in sequence as part of a data processing
/// pipeline, with the output of one processor becoming the input of the next.
pub trait PipeProcessor {
    /// Process the input data and return the transformed result.
    ///
    /// # Arguments
    /// * `data` - The input data to be processed
    ///
    /// # Returns
    /// The processed data in the appropriate output format
    fn process(&self, data: RawData) -> WparseResult<RawData>;

    /// Get the name/identifier of this pipeline processor.
    ///
    /// # Returns
    /// A string slice representing the processor name
    fn name(&self) -> &'static str;
}

pub type PipeHold = Arc<dyn PipeProcessor + Send + Sync>;

#[cfg(test)]
mod tests {
    use super::RawData;
    use bytes::Bytes;
    use std::sync::Arc;

    #[test]
    fn rawdata_as_bytes_and_len_cover_all_variants() {
        let text = RawData::from_string("hello");
        assert_eq!(text.as_bytes(), b"hello");
        assert_eq!(text.len(), 5);
        assert!(!text.is_zero_copy());

        let bytes = RawData::Bytes(Bytes::from_static(b"bin"));
        assert_eq!(bytes.as_bytes(), b"bin");
        assert_eq!(bytes.len(), 3);
        assert!(bytes.to_bytes().eq(&Bytes::from_static(b"bin")));
        assert!(!bytes.is_zero_copy());

        let arc = Arc::new(vec![1u8, 2, 3, 4]);
        let arc_raw = RawData::from_arc_bytes(arc.clone());
        assert_eq!(arc_raw.as_bytes(), arc.as_slice());
        assert_eq!(arc_raw.len(), 4);
        assert!(arc_raw.is_zero_copy());
        let bytes_from_arc = arc_raw.to_bytes();
        assert_eq!(bytes_from_arc.as_ref(), &[1, 2, 3, 4]);

        let owned = RawData::from_arc_bytes(Arc::new(vec![5u8, 6, 7]));
        let converted = owned.into_bytes();
        assert_eq!(converted.as_ref(), &[5, 6, 7]);
    }

    #[test]
    fn rawdata_is_empty_handles_all_variants() {
        assert!(RawData::from_string("").is_empty());
        assert!(RawData::Bytes(Bytes::new()).is_empty());
        assert!(RawData::from_arc_bytes(Arc::new(vec![])).is_empty());
        assert!(!RawData::from_string("x").is_empty());
    }
}
