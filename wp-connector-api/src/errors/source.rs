use derive_more::From;
use orion_error::{ErrorCode, StructError, UvsReason};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, Clone, PartialEq, Serialize, From)]
pub enum SourceReason {
    #[error("not data")]
    NotData,
    #[error("eof")]
    EOF,
    #[error("supplier error : {0}")]
    SupplierError(String),
    #[from(skip)]
    #[error("disconnected: {0}")]
    Disconnect(String),
    #[from(skip)]
    #[error("{0}")]
    Other(String),
    #[error("{0}")]
    Uvs(UvsReason),
}

impl ErrorCode for SourceReason {
    fn error_code(&self) -> i32 {
        255
    }
}

pub type SourceError = StructError<SourceReason>;
pub type SourceResult<T> = Result<T, StructError<SourceReason>>;
