use std::fmt::Display;

use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum OutFmt {
    #[serde(rename = "fmt")]
    Fmt(TextFmt),
}
impl Default for OutFmt {
    fn default() -> Self {
        OutFmt::Fmt(TextFmt::default())
    }
}
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize, Default, Copy)]
pub enum TextFmt {
    #[serde(rename = "json")]
    Json,
    #[serde(rename = "csv")]
    Csv,
    #[serde(rename = "show")]
    Show,
    #[serde(rename = "kv")]
    Kv,
    #[serde(rename = "raw")]
    #[default]
    Raw,
    #[serde(rename = "proto")]
    Proto,
    #[serde(rename = "proto-text")]
    ProtoText,
}

impl From<&str> for TextFmt {
    fn from(value: &str) -> Self {
        if value == "json" {
            TextFmt::Json
        } else if value == "csv" {
            TextFmt::Csv
        } else if value == "show" {
            TextFmt::Show
        } else if value == "kv" {
            TextFmt::Kv
        } else if value == "proto" {
            TextFmt::Proto
        } else if value == "proto-text" {
            TextFmt::ProtoText
        } else {
            TextFmt::Raw
        }
    }
}

impl Display for TextFmt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TextFmt::Json => write!(f, "json"),
            TextFmt::Csv => write!(f, "csv"),
            TextFmt::Show => write!(f, "show"),
            TextFmt::Kv => write!(f, "kv"),
            TextFmt::Raw => write!(f, "raw"),
            TextFmt::Proto => write!(f, "proto"),
            TextFmt::ProtoText => write!(f, "proto-text"),
        }
    }
}
