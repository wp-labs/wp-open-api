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
