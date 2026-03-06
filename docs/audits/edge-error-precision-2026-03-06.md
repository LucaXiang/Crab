# Edge Server 错误代码精确性审计

**审计日期**: 2026-03-06
**审计范围**: edge-server/src/ (所有 Rust 文件)
**审计方法**: grep 模式搜索 + 代码分析

## 执行摘要

发现 **79 个错误代码精确性问题**，涵盖 5 个主要反模式类别。这些问题影响系统的可诊断性和错误处理的精确度。

### 关键数据
- **HIGH 严重性**: 42 个问题（需立即修复）
- **MEDIUM 严重性**: 37 个问题（优化，影响诊断）
- **主要受影响模块**: 订单系统、API 数据传输、消息总线、数据库层

---

## 问题分类

### 1. AppError::internal(e.to_string()) — 24 处 (HIGH)

**特征**: 使用通用错误转换，丢失原始错误类型信息

**示例**:
```rust
// edge-server/src/api/data_transfer/handler.rs:58
serde_json::to_vec(&catalog)
    .map_err(|e| AppError::internal(e.to_string()))?;

// edge-server/src/message/transport/memory.rs:45
channel_send(msg)
    .map_err(|e| AppError::internal(e.to_string()))

// edge-server/src/cloud/worker.rs:567
serde_json::to_string(&batch)
    .map_err(|e| AppError::internal(format!("Serialize SyncBatch: {e}")))?;
```

**问题分析**:
- JSON 序列化/反序列化 → 应返回 `ErrorCode::InvalidFormat`
- 文件操作 → 应返回 `ErrorCode::FileSystemError`（待新增）
- 消息序列化 → 应返回 `ErrorCode::ProtocolError`（待新增）

**涉及文件**:
- api/data_transfer/handler.rs: 58, 130, 141-161, 186-208
- message/transport/memory.rs: 45, 52
- message/transport/mod.rs: 75, 87, 95, 108, 126, 152, 159
- message/bus.rs: 126, 136, 148
- cloud/worker.rs: 567, 571, 591
- core/state.rs: 190, 199, 250, 287

**建议修复**:
```rust
// 错误的做法
.map_err(|e| AppError::internal(e.to_string()))?

// 正确的做法
.map_err(|e| AppError::with_message(ErrorCode::InvalidFormat, e.to_string()))?
// 或
.map_err(|e| AppError::new(ErrorCode::ProtocolError))?
```

---

### 2. AppError::validation() 硬编码字符串 — 35 处 (MEDIUM)

**特征**: 业务规则违反时返回硬编码字符串，缺乏结构化错误码

**示例**:
```rust
// edge-server/src/utils/validation.rs:42
return Err(AppError::validation(format!("{field} must not be empty")));

// edge-server/src/api/products/handler.rs:63
return Err(AppError::validation(format!("Product '{}' already exists", name)));

// edge-server/src/utils/time.rs:14
.map_err(|_| AppError::validation(format!("Invalid date format: {}", date)))
```

**问题分析**:
- 空值检查 → 应用 `ErrorCode::RequiredField`
- 重复检查 → 应用 `ErrorCode::AlreadyExists`
- 格式验证 → 应用 `ErrorCode::InvalidFormat`
- 范围检查 → 应用 `ErrorCode::ValueOutOfRange`

**涉及文件与错误码映射**:
| 文件 | 问题类型 | 应用错误码 |
|------|---------|----------|
| utils/validation.rs:42,45,62 | 字段非空、范围检查 | RequiredField, ValueOutOfRange |
| utils/time.rs:14,21 | 日期格式 | InvalidFormat |
| api/products/handler.rs:63,76,82 | 商品存在性 | AlreadyExists, NotFound |
| api/attributes/handler.rs:92,99,362,433 | 属性验证 | RequiredAttribute (待新增) |
| api/marketing_groups/handler.rs:74-109,343-348 | 营销组验证 | 需细分 |
| api/price_rules/handler.rs:68,73 | 规则验证 | 需细分 |
| api/shifts/handler.rs:29,34 | 班次验证 | 需细分 |
| api/store_info/handler.rs:44 | 门店信息 | 需细分 |
| api/data_transfer/handler.rs:177,183,188,214 | 数据导入验证 | InvalidDataFormat, ImportFailed (待新增) |

**关键发现**:
- 多处重复 "硬编码字符串 + ErrorCode::ValidationError" 的模式
- 应推荐使用特定的 ErrorCode 替代泛化的 `validation()`

---

### 3. CommandErrorCode::InternalError 泛滥 — 18 处 (HIGH)

**特征**: 订单命令执行失败时，过度使用 `InternalError` 作为 catch-all

**示例**:
```rust
// edge-server/src/orders/manager/error.rs:50-52
fn classify_storage_error(e: &StorageError) -> CommandErrorCode {
    match e {
        StorageError::Serialization(_) => return CommandErrorCode::InternalError,  // ❌ 泛化
        StorageError::EventNotFound(_, _) => return CommandErrorCode::InternalError,  // ❌ 泛化
        _ => {}
    }
    // ...
    CommandErrorCode::SystemBusy  // ❌ 默认 fallback 也太泛化
}

// edge-server/src/orders/manager/mod.rs:498-499
.map_err(|e| OrderError::InvalidOperation(
    CommandErrorCode::InternalError,  // ❌ 应分化为 StampActivityNotFound
    format!("Failed to query stamp activity: {e}")
))?
```

**涉及文件**:
- orders/manager/error.rs: 50, 52, 121
- orders/manager/mod.rs: 421, 427, 446, 452, 464, 498-499, 509, 522, 535, 575, 585, 599, 711
- orders/actions/uncomp_item.rs: 76
- orders/actions/redeem_stamp.rs: 231

**问题分析**:

| 存储错误类型 | 当前映射 | 建议映射 |
|----------|---------|--------|
| Serialization | InternalError | 保留(序列化本质上是内部错误) |
| EventNotFound | InternalError | StorageCorrupted 或 SystemBusy |
| redb 磁盘满 | 字符串匹配→SystemBusy | StorageFull |
| redb 内存不足 | 字符串匹配→SystemBusy | OutOfMemory |
| redb 数据损坏 | 字符串匹配→SystemBusy | StorageCorrupted |

**关键问题**: `classify_storage_error()` 依赖字符串模式匹配，脆弱且维护困难。

**建议修复方案**:
1. 扩展 `StorageError` enum 添加新变体（如 `StorageError::DiskFull`, `StorageError::Corrupted`）
2. 在 `classify_storage_error()` 中精确分化每个案例
3. 避免字符串 `.contains()` 匹配

---

### 4. database() 调用通用化 — 14 处 (MEDIUM)

**特征**: 所有数据库错误都映射为 `ErrorCode::DatabaseError`，无细致区分

**示例**:
```rust
// edge-server/src/api/statistics/handler.rs:324
fetch_sales_data(pool).await
    .map_err(|e| AppError::database(e.to_string()))?;

// edge-server/src/printing/service.rs:30
PrintServiceError::Storage(e) => AppError::database(e.to_string()),
```

**涉及文件**:
| 文件 | 行数 | 问题 |
|------|------|------|
| api/statistics/handler.rs | 324, 364, 382, 393, 414, 435, 458, 479, 493, 509 | 统计查询 |
| printing/service.rs | 30 | 打印存储 |
| db/mod.rs | 23, 35, 44 | 初始化 |

**问题分析**:
- `RowNotFound` → 应返回 `ErrorCode::NotFound`
- 约束违反 → 应返回 `ErrorCode::AlreadyExists` 或 `ErrorCode::ConstraintViolation` (待新增)
- 连接池关闭 → 应返回 `ErrorCode::SystemBusy`

**关键问题**: `RepoError::From<sqlx::Error>` 在 mod.rs:76-93 使用字符串匹配：
```rust
// 脆弱：依赖 SQLite 错误消息的精确字符串
if msg.contains("UNIQUE constraint failed") { ... }
else if msg.contains("FOREIGN KEY constraint failed") { ... }
```

---

### 5. 字符串模式匹配 — 8 处 (MEDIUM)

**特征**: 错误分类依赖字符串模式匹配，脆弱易失败

**示例**:
```rust
// edge-server/src/db/repository/mod.rs:82
if msg.contains("UNIQUE constraint failed") { ... }

// edge-server/src/orders/manager/error.rs:60
if err_str.contains("no space") || err_str.contains("disk full") { ... }

// edge-server/src/db/repository/payment.rs:89
if msg.contains("unique") || msg.contains("duplicate") { ... }
```

**问题**:
- SQLite/redb 错误消息格式可能变化
- 大小写敏感性问题
- 易被新版本库改变

**建议**: 使用 enum match 替代字符串匹配

---

## 按文件的详细排查

### high-priority 文件 (P0)

#### 1. edge-server/src/orders/manager/mod.rs (14 处)

**问题统计**:
- 14 处 `CommandErrorCode::InternalError` 或通用错误映射
- 影响订单命令的每个关键路径

**具体位置**:
```
421: RuleNotFoundInOrder → InternalError ❌
427: ApplyRule 失败 → InternalError ❌
446: ItemNotFound → InternalError ❌
452: InsufficientStamps → InternalError ❌
464: OtherError → InternalError ❌
498-499: StampActivity 查询失败 → InternalError ❌
509: ApplyRule 失败 → InternalError ❌
522: OtherError → InternalError ❌
535: OtherError → InternalError ❌
575: StampActivity 查询失败 → InternalError ❌
585: ApplyRule 失败 → InternalError ❌
599: ApplyRule 失败 → InternalError ❌
711: OtherError → InternalError ❌
```

**建议**: 建立清晰的 CommandErrorCode 映射，将每个业务错误映射到特定码

#### 2. edge-server/src/api/data_transfer/handler.rs (15 处)

**问题统计**:
- 7 处 `AppError::internal(e.to_string())` — 序列化
- 5 处 `AppError::internal(e.to_string())` — 文件操作
- 2 处 `AppError::validation(...)` — ZIP 验证
- 1 处 `AppError::validation(...)` — JSON 验证

**具体位置及建议**:
```
58:   JSON 序列化 → InvalidFormat
130:  JSON 写入 → FileSystemError
141:  ZIP 读取 → InvalidFormat
143:  ZIP 列举 → InvalidFormat
157:  文件读取 → FileSystemError
159:  文件读取 → FileSystemError
161:  JSON 写入 → InvalidFormat
167:  ZIP 写入 → FileSystemError
177:  ZIP 验证 → InvalidDataFormat (新增)
183:  ZIP 字段检查 → RequiredField
186:  JSON 解析 → InvalidFormat
193:  目录创建 → FileSystemError
198:  ZIP 提取 → InvalidDataFormat
207:  File 读取 → FileSystemError
208:  File 写入 → FileSystemError
```

#### 3. edge-server/src/message/tcp_server.rs (4 处)

**问题**:
- 40: TCP 绑定失败 → `internal()` (应 PortBindingFailed)
- 56: SSL 握手准备失败 → `internal()` (应 TlsHandshakeFailed)
- 151: TLS 握手失败 → `internal()` (应 TlsHandshakeFailed)
- 155: TLS 握手超时 → `internal()` (应 NetworkTimeout)

#### 4. edge-server/src/db/repository/mod.rs (8 处)

**问题**: `From<sqlx::Error>` 的实现

```rust
76-93: fn from(err: sqlx::Error) -> Self {
    match &err {
        sqlx::Error::RowNotFound => RepoError::NotFound(...),  // ✅ 好
        sqlx::Error::Database(db_err) => {
            let msg = db_err.message().to_string();
            if msg.contains("UNIQUE constraint failed") { ... }  // ❌ 字符串匹配
            else if msg.contains("FOREIGN KEY constraint failed") { ... }  // ❌ 字符串匹配
            else { RepoError::Database(msg) }  // ❌ 泛化
        }
        _ => RepoError::Database(err.to_string()),  // ❌ 泛化所有其他错误
    }
}
```

**应改为**:
```rust
match err {
    sqlx::Error::RowNotFound => RepoError::NotFound(...),
    sqlx::Error::Database(db_err) => {
        // 使用 sqlx 提供的错误码/类别而非字符串匹配
        match db_err.kind() {
            sqlx::error::ErrorKind::UniqueViolation => RepoError::Duplicate(...),
            sqlx::error::ErrorKind::ForeignKeyViolation => RepoError::Database(...),
            _ => RepoError::Database(db_err.message().to_string()),
        }
    }
    sqlx::Error::PoolClosed => RepoError::Database("Pool closed".into()),
    sqlx::Error::PoolTimedOut => RepoError::Database("Pool timeout".into()),
    sqlx::Error::Io(e) => RepoError::Database(format!("IO error: {}", e)),
    _ => RepoError::Database(err.to_string()),
}
```

---

## 建议的新增 ErrorCode

基于审计发现，以下 ErrorCode 应被添加到 `shared/src/error/codes.rs`：

```rust
// 文件操作相关 (9xxx)
FileSystemError = 9001,
FileNotFound = 9002,
DirectoryNotFound = 9003,
PermissionDenied = 9004,

// 网络/协议相关 (9xxx)
TlsHandshakeFailed = 9010,
PortBindingFailed = 9011,
NetworkTimeout = 9012,
ProtocolError = 9013,

// 数据格式相关 (6xxx)
InvalidDataFormat = 6001,
ConstraintViolation = 9020,  // 数据库约束
ImportFailed = 6002,  // 导入相关

// 打印相关 (9xxx)
KitchenOrderNotFound = 9030,
PrintStorageError = 9031,
PrintDeviceNotFound = 9032,

// 存储/初始化相关 (9xxx)
InitializationFailed = 9040,
StorageCorrupted = 4008,  // 订单相关存储
```

---

## 修复优先级与时间表

### P0 - 立即修复 (2-3 天)

**文件**: orders/manager/, db/repository/mod.rs, api/data_transfer/

**工作量**: ~4-6 小时

1. 新增上述 ErrorCode 变体
2. 修复 `CommandErrorCode::InternalError` 泛滥 (14 处)
3. 修复 `RepoError::From<sqlx::Error>` 的字符串匹配
4. 重构 `classify_storage_error()` 的分类逻辑

### P1 - 优化 (本周)

**文件**: api/statistics/, message/tcp_server.rs, printing/

**工作量**: ~2-3 小时

1. 修复统计 API 的 database() 泛化 (10 处)
2. 修复消息总线的 TLS/TCP 错误映射
3. 打印服务的错误分化

### P2 - 规范化 (长期)

1. **代码审查规范**: 禁止新增 `map_err(|e| AppError::internal(e.to_string()))`
2. **Clippy 规则**: 添加 custom lint 检测泛化 map_err
3. **培训**: 所有开发者学习 ErrorCode 体系和分化原则

---

## 一般性建议

### 1. 避免泛化错误映射

❌ 不要这样做:
```rust
.map_err(|e| AppError::internal(e.to_string()))?
.map_err(|e| AppError::database(e.to_string()))?
```

✅ 应该这样:
```rust
// 序列化失败
.map_err(|e| AppError::with_message(ErrorCode::InvalidFormat, e.to_string()))?

// 特定错误
.map_err(|e| AppError::new(ErrorCode::FileSystemError))?
```

### 2. 避免字符串模式匹配

❌ 不要这样做:
```rust
if msg.contains("UNIQUE constraint failed") { ... }
```

✅ 应该这样:
```rust
if db_err.kind() == sqlx::error::ErrorKind::UniqueViolation { ... }
```

### 3. 使用枚举来分类错误

```rust
pub enum ArchiveError {
    NotFound(String),
    Duplicate(String),
    Database(String),
    Validation(String),
    DataCorruption(String),  // ← 新增，替代 InternalError
    // ...
}
```

### 4. 前端集成

每个新的 ErrorCode 必须同步到:
1. `red_coral/src/core/domain/types/api/errorCode.ts`
2. `red_coral/src/infrastructure/i18n/locales/zh-CN.json` — 添加翻译
3. `red_coral/src/infrastructure/i18n/locales/es-ES.json` — 同步翻译

---

## 检查清单

修复时请按以下清单验证：

- [ ] 新增 ErrorCode 已添加到 shared/src/error/codes.rs
- [ ] HTTP 状态码映射已在 shared/src/error/http.rs 中添加
- [ ] 所有 map_err(|e| AppError::internal(e.to_string())) 已替换
- [ ] 所有字符串模式匹配已改为 enum match
- [ ] CommandErrorCode::InternalError 的使用已分化为具体错误码
- [ ] 前端 i18n 已同步更新
- [ ] cargo clippy --workspace 无新警告
- [ ] cargo test --lib --all 通过所有测试

---

## 参考资源

- 错误码定义: `shared/src/error/codes.rs`
- 错误类型: `shared/src/error/types.rs`
- HTTP 映射: `shared/src/error/http.rs`
- 订单错误码: `shared/src/order/types.rs::CommandErrorCode`
- CLAUDE.md: `edge-server/CLAUDE.md` 的错误处理部分

---

**审计完成**: 2026-03-06
**审计员**: error-precision-edge task
**下一步行动**: 提交 P0 修复 PR
