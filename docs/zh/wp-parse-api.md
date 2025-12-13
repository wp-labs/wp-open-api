# wp-parse-api 开发指南

本文档介绍 `wp-parse-api` 的核心职责、对外暴露的类型以及常见的开发场景。

## 1. 模块职责

`wp-parse-api` 是 WPL 解析插件的最小依赖集合，提供以下能力：

- `RawData`：统一的原始数据表示（字符串、`Bytes`、零拷贝 `Arc<Vec<u8>>`）。
- **兼容性提示**：0.4.6 之前 `ArcBytes` 内部为 `Arc<[u8]>`。该方案已淘汰，为了支持真正的零拷贝 `into_bytes()`，现在统一为 `Arc<Vec<u8>>`。若仍有旧代码提供 `Arc<[u8]>`，请改为构造 `Arc<Vec<u8>>`，或暂时使用 `RawData::from_arc_slice()`（但会复制一次数据）。
- `Parsable` trait：插件需要实现的解析接口。
- `PipeProcessor` trait：流水线式前后处理器。
- `Wparse*` 错误类型：插件返回的标准错误封装。

## 2. 核心类型

### RawData

```rust
#[derive(Debug, Clone)]
pub enum RawData {
    String(String),
    Bytes(Bytes),
    ArcBytes(Arc<Vec<u8>>),
}
```

常用方法：

- `as_bytes()`：以只读切片方式获取数据。
- `to_bytes()`：必要时复制成 `Bytes`（保留当前数据但会复制）。
- `into_bytes()`：消耗自身以获得 `Bytes`，`ArcBytes` 在独占时可零拷贝转换。
- `is_zero_copy()`：判断数据是否零拷贝。
- `len()/is_empty()`：长度状态。
- `from_arc_slice()`：从 `Arc<[u8]>` 构建（内部会复制一份 Vec），便于旧接口平滑迁移到 `Arc<Vec<u8>>`。

### Parsable

```rust
pub trait Parsable: Send + Sync {
    fn parse(&self, data: &RawData, successful_others: usize) -> DataResult;
    fn name(&self) -> &str;
}
```

- `successful_others` 用于竞速型解析（多 Parser 并行时了解其他解析器状态）。
- `DataResult = Result<(DataRecord, RawData), WparseError>`：成功返回解析出的 `DataRecord` 及剩余原始数据片段（可继续喂给后续 Parser 或回退逻辑）。

### PipeProcessor

```rust
pub trait PipeProcessor {
    fn process(&self, data: RawData) -> WparseResult<RawData>;
    fn name(&self) -> &'static str;
}
```

适用于在 Parser 前后串联若干加工步骤（例如 Base64 解码、字符串清洗）。

### 错误类型

- `WparseReason`：`thiserror` 枚举，包含 `Plugin(String)`、`NotMatch`、`LineProc(String)`、`Uvs(UvsReason)` 等变体。
- `WparseError = StructError<WparseReason>`：统一错误包装，可与 `orion_error` 生态互通。
- `WparseResult<T> = Result<T, WparseError>`：通用结果别名。

旧名称 `WplParse*` 仍提供 `#[deprecated]` 别名供渐进迁移。

## 3. 快速开始

### 实现 Parser

```rust
use wp_parse_api::{Parsable, RawData, DataResult, WparseReason};

pub struct SimpleParser;

impl Parsable for SimpleParser {
    fn parse(&self, data: &RawData, _others: usize) -> DataResult {
        let text = std::str::from_utf8(data.as_bytes())
            .map_err(|e| WparseReason::Plugin(format!("invalid utf8: {}", e)).to_err())?;
        let record = wp_data_model::model::Record::<wp_data_model::model::DataField>::default();
        // 构造 DataRecord ...
        Ok((record, RawData::from_str("")))
    }

    fn name(&self) -> &str {
        "simple_parser"
    }
}
```

### 使用 PipeProcessor

```rust
use wp_parse_api::{PipeProcessor, RawData, WparseResult};

pub struct TrimProcessor;

impl PipeProcessor for TrimProcessor {
    fn process(&self, data: RawData) -> WparseResult<RawData> {
        match data {
            RawData::String(s) => Ok(RawData::String(s.trim().to_string())),
            _ => Ok(data),
        }
    }

    fn name(&self) -> &'static str {
        "trim"
    }
}
```

## 4. 错误与调试

- 使用 `WparseReason::Plugin(String)` 表达业务错误，`LineProc` 表达具体处理阶段。
- 借助 `StructError::context` 可追加多层定位信息。
- 遵循“不要 panic”准则，所有错误通过 `WparseError` 返回。

## 5. 迁移提醒

- 请逐步将旧的 `WplParse*` 类型替换为 `Wparse*`。  
- `RawData` 统一了 `String`/`Bytes`，在跨模块传递时尽量保持零拷贝（`ArcBytes`）。

## 6. 参考

- `src/lib.rs`： trait 定义与 re-export。  
- `src/error.rs`：错误类型详情。  
- `wp-model-core`：`DataRecord` 定义，Parser 输出需要依赖该 crate。
