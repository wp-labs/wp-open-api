use derive_more::From;
use orion_error::StructError;
use orion_error::{ErrorCode, UvsReason};
use serde::Serialize;
use std::sync::mpsc::SendError;
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Serialize, From)]
pub enum SinkReason {
    #[error("sink unavailable {0}")]
    Sink(String),
    #[error("set mock error")]
    Mock,
    #[error("stg ctrl error")]
    StgCtrl,
    #[error("{0}")]
    Uvs(UvsReason),
}
impl ErrorCode for SinkReason {
    fn error_code(&self) -> i32 {
        255
    }
}

pub type SinkError = StructError<SinkReason>;

pub trait ReasonSummary {
    fn summary(&self) -> String;
}

impl<T> From<SendError<T>> for SinkReason
where
    T: ReasonSummary,
{
    fn from(err: SendError<T>) -> Self {
        SinkReason::Sink(format!("send error: {}", err.0.summary()))
    }
}

impl From<anyhow::Error> for SinkReason {
    fn from(e: anyhow::Error) -> Self {
        SinkReason::Sink(format!("{}", e))
    }
}

pub type SinkResult<T> = Result<T, SinkError>;

impl SinkReason {
    pub fn sink<S: Into<String>>(msg: S) -> Self {
        SinkReason::Sink(msg.into())
    }
}

pub trait SinkErrorOwe<T> {
    fn owe_sink<S: Into<String>>(self, msg: S) -> Result<T, StructError<SinkReason>>;
}

impl<T, E> SinkErrorOwe<T> for Result<T, E>
where
    E: std::fmt::Display,
{
    fn owe_sink<S: Into<String>>(self, msg: S) -> Result<T, StructError<SinkReason>> {
        match self {
            Ok(v) => Ok(v),
            Err(e) => {
                Err(StructError::from(SinkReason::Sink(msg.into())).with_detail(e.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[derive(Clone)]
    struct Summary(&'static str);

    impl ReasonSummary for Summary {
        fn summary(&self) -> String {
            self.0.into()
        }
    }

    #[test]
    fn sink_reason_from_send_error_uses_inner_summary() {
        let (tx, rx) = mpsc::channel();
        drop(rx);
        let err = tx.send(Summary("queue overflow")).unwrap_err();
        let reason = SinkReason::from(err);
        match reason {
            SinkReason::Sink(msg) => assert!(msg.contains("queue overflow")),
            other => panic!("unexpected reason: {other:?}"),
        }
    }

    #[test]
    fn sink_error_owe_wraps_displayable_error() {
        let failing: Result<(), &str> = Err("io timeout");
        let err = failing.owe_sink("flush failed").unwrap_err();
        match err.reason() {
            SinkReason::Sink(msg) => assert_eq!(msg, "flush failed"),
            other => panic!("unexpected reason: {other:?}"),
        }
        let detail = err.detail();
        assert_eq!(detail.as_ref().map(|s| s.as_str()), Some("io timeout"));
    }
}
