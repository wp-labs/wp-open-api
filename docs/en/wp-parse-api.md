# wp-parse-api Developer Guide

This guide explains the purpose of `wp-parse-api`, the public types it exports, and how to build parsers or pipeline processors with it.

## 1. Module Responsibilities

`wp-parse-api` is the minimal dependency set for WPL parsers. It provides:

- `RawData`: a unified representation for incoming payloads (string, `Bytes`, or zero-copy `Arc<Vec<u8>>`).
- `Parsable`: the trait every parser plug-in must implement.
- `PipeProcessor`: lightweight processors that run before/after parsers.
- `Wparse*` error types: the canonical error channel for plug-ins.

> **Compatibility note**: before 0.4.6, `RawData::ArcBytes` stored `Arc<[u8]>`. That layout is deprecated. To enable real zero-copy `into_bytes()`, the variant now carries `Arc<Vec<u8>>`. If you still have `Arc<[u8]>`, convert it into `Arc<Vec<u8>>` directly or temporarily call `RawData::from_arc_slice()` (which clones the data).

## 2. Core Types

### RawData

```rust
#[derive(Debug, Clone)]
pub enum RawData {
    String(String),
    Bytes(Bytes),
    ArcBytes(Arc<Vec<u8>>),
}
```

Key methods:

- `as_bytes()` – borrow the payload as `&[u8]`.
- `to_bytes()` – clone into `Bytes` (always copies).
- `into_bytes()` – consume `RawData` and, when `Arc::try_unwrap` succeeds, move the underlying `Vec<u8>` into `Bytes` without cloning.
- `is_zero_copy()` – tells whether the value is backed by `Arc<Vec<u8>>`.
- `len()` / `is_empty()` – convenience helpers.
- `from_arc_slice()` – compatibility helper that clones an `Arc<[u8]>` into the new representation.

### Parsable

```rust
pub trait Parsable: Send + Sync {
    fn parse(&self, data: &RawData, successful_others: usize) -> DataResult;
    fn name(&self) -> &str;
}
```

- `successful_others` is used by “race” setups where multiple parsers run in parallel and need visibility into the other parsers' results.
- `DataResult = Result<(DataRecord, RawData), WparseError>` – on success you return the parsed `DataRecord` plus any remaining raw payload for downstream processing or fallback parsers.

### PipeProcessor

```rust
pub trait PipeProcessor {
    fn process(&self, data: RawData) -> WparseResult<RawData>;
    fn name(&self) -> &'static str;
}
```

Useful when you want to chain pre/post steps such as Base64 decoding, trimming, escaping, etc.

### Error Types

- `WparseReason`: `thiserror` enum with variants `Plugin(String)`, `NotMatch`, `LineProc(String)`, `Uvs(UvsReason)`, ...
- `WparseError = StructError<WparseReason>`: wraps the reason and retains context stacks from `orion_error`.
- `WparseResult<T> = Result<T, WparseError>`.

Deprecated aliases `WplParse*` still exist for gradual migration.

## 3. Getting Started

### Implementing a Parser

```rust
use wp_parse_api::{Parsable, RawData, DataResult, WparseReason};
use wp_data_model::model::{DataField, DataRecord};

pub struct SimpleParser;

impl Parsable for SimpleParser {
    fn parse(&self, data: &RawData, _others: usize) -> DataResult {
        let text = std::str::from_utf8(data.as_bytes())
            .map_err(|e| WparseReason::Plugin(format!("invalid utf8: {}", e)).to_err())?;
        let mut record = DataRecord::default();
        record.append(DataField::from_chars("message", text));
        Ok((record, RawData::from_str("")))
    }

    fn name(&self) -> &str {
        "simple_parser"
    }
}
```

### Using PipeProcessor

```rust
use wp_parse_api::{PipeProcessor, RawData, WparseResult};

pub struct TrimProcessor;

impl PipeProcessor for TrimProcessor {
    fn process(&self, data: RawData) -> WparseResult<RawData> {
        match data {
            RawData::String(s) => Ok(RawData::String(s.trim().to_string())),
            other => Ok(other),
        }
    }

    fn name(&self) -> &'static str {
        "trim"
    }
}
```

## 4. Errors & Debugging

- Use `WparseReason::Plugin(String)` for business-level errors, `LineProc` to pinpoint processing stages.
- Take advantage of `StructError::context(..)` to add stack traces.
- Never panic inside plug-ins; always propagate `WparseError`.

## 5. Migration Tips

- Replace deprecated `WplParse*` aliases with the `Wparse*` types.
- Keep payloads zero-copy whenever possible. Prefer `RawData::ArcBytes(Arc<Vec<u8>>)` instead of cloning into `String`.

## 6. References

- `src/lib.rs`: public traits and exports.
- `src/error.rs`: detailed error definitions.
- `wp-model-core`: `DataRecord` and value types used in parser outputs.
