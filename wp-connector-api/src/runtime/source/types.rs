use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::borrow::Cow;
use std::sync::Arc;

use super::event::SourceBatch;
use crate::{SourceReason, SourceResult};

#[derive(Clone, Copy, Debug, Default)]
pub struct SourceCaps {
    pub ack: bool,
    pub seek: bool,
    pub parallel: bool,
}

/// AckToken：强类型 ack 标记的 trait，用于上游确认消费位置/状态
pub trait AckToken: Send + Sync + std::fmt::Debug {}

/// SeekPosition：强类型定位信息的 trait，用于外部请求数据源跳转位置
pub trait SeekPosition: Send + Sync + std::fmt::Debug {}

/// 强类型控制事件：用于向数据源传递外部控制能力（停止、隔离、定位等）
#[derive(Debug, Clone)]
pub enum ControlEvent {
    /// 立即停止当前源（应尽快结束 start/receive 循环并关闭资源）
    Stop,
    /// 是否隔离（true=进入隔离/暂停产出，false=恢复）
    Isolate(bool),
    /// 可选：外部定位（seek）请求
    Seek(Arc<dyn SeekPosition>),
}

/// 控制通道类型：基于 async_broadcast 的 Receiver，事件为 `ControlEvent`
pub type CtrlRx = async_broadcast::Receiver<ControlEvent>;

#[async_trait]
pub trait DataSource: Send + Sync {
    /// v2 接口：产出 `SourceBatch`（推荐批量>=1）；若返回空批次表示暂无可消费数据。
    /// V2 API: emit `SourceBatch` (recommended batch size >= 1); an empty batch means "no data yet".
    /// 调用方需根据 `SourceBatch` 中的每个事件推进 offset/ack；EOF/终止等通过 SourceError 约定表示。
    /// Caller must advance offset/ack for every event inside the batch; EOF/termination is signaled via
    /// `SourceError`.
    /// 要求：幂等；在 `start()` 成功后可反复调用，遇到 `EOF`/终止策略应尽快返回错误结束循环。
    /// Must stay idempotent and callable repeatedly once `start()` succeeds; return an error promptly for
    /// EOF/end-of-stream conditions.
    async fn receive(&mut self) -> SourceResult<SourceBatch>;

    /// 若实现者支持非阻塞拉取，可返回已就绪批次；否则返回 None。
    fn try_receive(&mut self) -> Option<SourceBatch>;

    /// 静态能力：告知实现是否**永久**支持非阻塞尝试。
    fn supports_try_receive(&self) -> bool {
        false
    }

    /// 动态能力：在运行期判断当前状态下是否可安全调用 `try_receive`。
    fn can_try_receive(&mut self) -> bool {
        self.supports_try_receive()
    }

    /// 数据源唯一标识，用于日志/统计等。
    fn identifier(&self) -> String;
    /// 零分配标识符获取（默认回退到分配型接口）。建议上层优先使用本方法以减少热路径分配。
    fn identifier_ref(&self) -> Cow<'_, str> {
        Cow::Owned(self.identifier())
    }

    fn caps(&self) -> SourceCaps {
        SourceCaps::default()
    }
    /// 启动源的外部生命周期管理（服务端式源可通过 `ctrl_rx` 接收 ControlEvent）。
    /// 约定：应幂等；重复调用若已启动应返回错误或忽略；必须在成功后方可 `receive()`/`recv()`。
    async fn start(&mut self, _ctrl_rx: CtrlRx) -> SourceResult<()> {
        Ok(())
    }
    /// 停止源并释放资源。约定：幂等；允许在未 `start()` 情况下调用（应安全返回）。
    async fn close(&mut self) -> SourceResult<()> {
        Ok(())
    }
    /// 消费确认。约定：仅当 `caps().ack == true` 的实现应返回 Ok；默认返回“不支持”。
    async fn ack(&mut self, _token: Arc<dyn AckToken>) -> SourceResult<()> {
        Err(SourceReason::SupplierError("ack unsupported".into()).into())
    }
    /// 外部定位。约定：仅当 `caps().seek == true` 的实现应返回 Ok；默认返回“不支持”。
    async fn seek(&mut self, _pos: Arc<dyn SeekPosition>) -> SourceResult<()> {
        Err(SourceReason::SupplierError("seek unsupported".into()).into())
    }
}

const INLINE_TAG_CAPACITY: usize = 16;

/// 标签集合（轻量、顺序无关）
#[derive(Clone, Default, Debug, Serialize, Deserialize)]
pub struct Tags {
    item: SmallVec<[(String, String); INLINE_TAG_CAPACITY]>,
}

impl Tags {
    pub fn new() -> Self {
        Self {
            item: SmallVec::new(),
        }
    }
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) {
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
    /// Convenience accessor used by existing engine/tests
    pub fn get(&self, k: &str) -> Option<&str> {
        self.item
            .binary_search_by(|(existing, _)| existing.as_str().cmp(k))
            .ok()
            .and_then(|idx| self.item.get(idx))
            .map(|(_, val)| val.as_str())
    }
    pub fn is_empty(&self) -> bool {
        self.item.is_empty()
    }
}

// 兼容 helpers
impl Tags {
    pub fn set_tag(&mut self, key: &str, value: String) {
        self.set(key.to_string(), value);
    }
    pub fn len(&self) -> usize {
        self.item.len()
    }
}

#[cfg(test)]
mod tests {
    use super::Tags;

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

        tags.set_tag("key", "value".to_string());
        assert!(!tags.is_empty());
        assert_eq!(tags.get("key"), Some("value"));
    }
}
