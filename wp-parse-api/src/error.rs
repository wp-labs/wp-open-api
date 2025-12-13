use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum DataErrKind {
    #[error("format error : {0}\n{1:?} ")]
    FormatError(String, Option<String>),
    #[error("not complete")]
    NotComplete,
    #[error("no parse data: {0}")]
    UnParse(String),

    #[error("less data")]
    LessData,
    #[error("empty data")]
    EmptyData,
    #[error("struct less : {0}")]
    LessStc(String),
    #[error("define less : {0}")]
    LessDef(String),
}

impl From<String> for DataErrKind {
    fn from(value: String) -> Self {
        DataErrKind::FormatError(value, None)
    }
}

use derive_more::From;
use orion_error::ErrorCode;
use orion_error::StructError;
use orion_error::ToStructError;
use orion_error::UvsDataFrom;
use orion_error::UvsReason;

#[derive(Error, Debug, Clone, PartialEq, Serialize, From)]
pub enum WparseReason {
    #[from(skip)]
    #[error("plugin >{0}")]
    Plugin(String),
    #[error("not match")]
    NotMatch,
    #[error("line proc > {0}")]
    LineProc(String),
    #[error("{0}")]
    Uvs(UvsReason),
}
impl ErrorCode for WparseReason {
    fn error_code(&self) -> i32 {
        500
    }
}

pub type WparseError = StructError<WparseReason>;

//universal_owe!(ParseEngineError, ParseOweUniversal);

impl From<DataErrKind> for WparseError {
    fn from(value: DataErrKind) -> Self {
        WparseReason::from_data(format!("{}", value), None).to_err()
    }
}
pub type WparseResult<T> = Result<T, WparseError>;

/// 兼容别名：保留历史命名，方便渐进迁移。
#[deprecated(note = "use `WparseReason` instead")]
pub type WplParseReason = WparseReason;

#[deprecated(note = "use `WparseError` instead")]
pub type WplParseError = WparseError;

#[deprecated(note = "use `WparseResult` instead")]
pub type WplParseResult<T> = WparseResult<T>;
