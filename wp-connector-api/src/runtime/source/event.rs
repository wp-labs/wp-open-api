use std::net::IpAddr;
use std::sync::Arc;
use wp_parse_api::RawData;

use super::types::Tags;

/// Parse 侧预处理钩子
pub type EventPreHook = Arc<dyn Fn(&mut SourceEvent) + Send + Sync + 'static>;

#[derive(Clone)]
pub struct SourceEvent {
    pub event_id: u64,
    pub src_key: Arc<String>,
    pub payload: RawData,
    pub tags: Arc<Tags>,
    pub ups_ip: Option<IpAddr>,
    /// 可选：parse 线程在进入 WPL 前调用
    pub preproc: Option<EventPreHook>,
}

/// 一批源事件，便于批量传输；允许返回空 Vec 代表暂时无数据。
/// A batch of events for bulk delivery; empty Vec means "no data for now".
pub type SourceBatch = Vec<SourceEvent>;

impl SourceEvent {
    /// 构造一个最小帧
    pub fn new(event_id: u64, src_key: Arc<String>, payload: RawData, tags: Arc<Tags>) -> Self {
        Self {
            event_id,
            src_key,
            payload,
            tags,
            ups_ip: None,
            preproc: None,
        }
    }
}

impl std::fmt::Debug for SourceEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceEvent")
            .field("id", &self.event_id)
            .field("src_key", &self.src_key)
            .field(
                "payload",
                &match &self.payload {
                    RawData::String(s) => format!("String(len={})", s.len()),
                    RawData::Bytes(b) => format!("Bytes(len={})", b.len()),
                    RawData::ArcBytes(arc) => format!("ArcBytes(len={}, zcp ={})", arc.len(), true),
                },
            )
            .field("tags", &format!("{} tags", self.tags.len()))
            .field("ups_ip", &self.ups_ip)
            .finish()
    }
}
