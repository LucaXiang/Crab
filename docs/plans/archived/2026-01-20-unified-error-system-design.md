# 统一错误系统设计

## 概述
为 Crab 项目设计统一的、可维护的、支持国际化的错误系统，覆盖三层架构：
- Edge-Server (Rust)
- Tauri (Rust)
- React 前端 (TypeScript)

## 设计决策

| 决策项 | 选择 |
|-------|------|
| 错误码格式 | 纯数字 (如 4001) |
| 分段规则 | 按模块分段 (0xxx-9xxx) |
| 详细程度 | code + message + details |
| message 用途 | 给开发者 (英文)，前端查 i18n |
| 单一来源 | shared crate |
| i18n key | 数字码作为 key |
| HTTP 映射 | 按分类默认 + 特殊覆盖 |

## 架构图

```
shared/src/error/          (Single Source of Truth)
  ├── codes.rs             错误码枚举
  ├── types.rs             AppError, ApiResponse
  ├── http.rs              HTTP 状态码映射
  └── category.rs          错误分类
        │
        ▼ (build.rs 生成)
red_coral/src/generated/error-codes.ts
        │
        ▼
red_coral/src/i18n/locales/{en-US,zh-CN}.json
```

## 错误码分段

| 范围 | 分类 | 说明 |
|-----|------|-----|
| 0xxx | General | 通用/验证错误 |
| 1xxx | Auth | 认证错误 |
| 2xxx | Permission | 授权错误 |
| 3xxx | Tenant | 租户/激活错误 |
| 4xxx | Order | 订单错误 |
| 5xxx | Payment | 支付错误 |
| 6xxx | Product | 商品/分类/规格错误 |
| 7xxx | Table | 桌台/区域错误 |
| 8xxx | Employee | 员工/角色错误 |
| 9xxx | System | 系统/基础设施错误 |

## 完整错误码清单

### 0xxx: 通用
- 0 Success - 成功
- 1 Unknown - 未知错误
- 2 ValidationFailed - 验证失败
- 3 NotFound - 资源不存在
- 4 AlreadyExists - 资源已存在
- 5 InvalidRequest - 无效请求
- 6 InvalidFormat - 格式错误
- 7 RequiredField - 必填字段缺失
- 8 ValueOutOfRange - 值超出范围

### 1xxx: 认证
- 1001 NotAuthenticated - 未登录
- 1002 InvalidCredentials - 用户名或密码错误
- 1003 TokenExpired - Token 已过期
- 1004 TokenInvalid - Token 无效
- 1005 SessionExpired - 会话过期
- 1006 AccountLocked - 账户已锁定
- 1007 AccountDisabled - 账户已禁用

### 2xxx: 授权
- 2001 PermissionDenied - 无权限
- 2002 RoleRequired - 需要特定角色
- 2003 AdminRequired - 需要管理员权限
- 2004 CannotModifyAdmin - 不能修改管理员
- 2005 CannotDeleteAdmin - 不能删除管理员

### 3xxx: 租户/激活
- 3001 TenantNotSelected - 未选择租户
- 3002 TenantNotFound - 租户不存在
- 3003 ActivationFailed - 激活失败
- 3004 CertificateInvalid - 证书无效
- 3005 LicenseExpired - 许可证过期

### 4xxx: 订单
- 4001 OrderNotFound - 订单不存在
- 4002 OrderAlreadyPaid - 订单已支付
- 4003 OrderAlreadyCompleted - 订单已完成
- 4004 OrderAlreadyVoided - 订单已作废
- 4005 OrderHasPayments - 订单有支付记录
- 4006 OrderItemNotFound - 订单项不存在
- 4007 OrderEmpty - 订单为空

### 5xxx: 支付
- 5001 PaymentFailed - 支付失败
- 5002 PaymentInsufficientAmount - 支付金额不足
- 5003 PaymentInvalidMethod - 无效支付方式
- 5004 PaymentAlreadyRefunded - 已退款
- 5005 PaymentRefundExceedsAmount - 退款超额

### 6xxx: 商品/分类/规格
- 6001 ProductNotFound - 商品不存在
- 6002 ProductInvalidPrice - 商品价格无效
- 6003 ProductOutOfStock - 商品缺货
- 6101 CategoryNotFound - 分类不存在
- 6102 CategoryHasProducts - 分类下有商品
- 6103 CategoryNameExists - 分类名已存在
- 6201 SpecNotFound - 规格不存在
- 6301 AttributeNotFound - 属性不存在
- 6302 AttributeBindFailed - 属性绑定失败

### 7xxx: 桌台/区域
- 7001 TableNotFound - 桌台不存在
- 7002 TableOccupied - 桌台占用中
- 7003 TableAlreadyEmpty - 桌台已空
- 7101 ZoneNotFound - 区域不存在
- 7102 ZoneHasTables - 区域下有桌台
- 7103 ZoneNameExists - 区域名已存在

### 8xxx: 员工/角色
- 8001 EmployeeNotFound - 员工不存在
- 8002 EmployeeUsernameExists - 用户名已存在
- 8003 EmployeeCannotDeleteSelf - 不能删除自己
- 8101 RoleNotFound - 角色不存在
- 8102 RoleNameExists - 角色名已存在
- 8103 RoleInUse - 角色使用中

### 9xxx: 系统
- 9001 InternalError - 内部错误
- 9002 DatabaseError - 数据库错误
- 9003 NetworkError - 网络错误
- 9004 TimeoutError - 超时
- 9005 ConfigError - 配置错误
- 9101 BridgeNotInitialized - Bridge 未初始化
- 9102 BridgeNotConnected - Bridge 未连接
- 9103 BridgeConnectionFailed - Bridge 连接失败
- 9201 PrinterNotAvailable - 打印机不可用
- 9202 PrintFailed - 打印失败

## HTTP 状态码映射

| 分类 | 默认状态码 | 特殊情况 |
|-----|----------|---------|
| 0xxx General | 400 | NotFound→404, AlreadyExists→409 |
| 1xxx Auth | 401 | - |
| 2xxx Permission | 403 | - |
| 3xxx Tenant | 403 | - |
| 4xxx Order | 400 | OrderNotFound→404, AlreadyPaid→409 |
| 5xxx Payment | 400 | InsufficientAmount→402 |
| 6xxx Product | 400 | NotFound→404 |
| 7xxx Table | 400 | NotFound→404 |
| 8xxx Employee | 400 | NotFound→404 |
| 9xxx System | 500 | - |

## API 响应格式

```json
{
  "code": 4001,
  "message": "Order not found",
  "data": null,
  "details": { "order_id": "abc123" }
}
```

- `code`: 数字错误码，0 或 null 表示成功
- `message`: 英文开发者消息
- `data`: 响应数据
- `details`: 上下文详情，用于 i18n 插值

## 前端 i18n 结构

```json
{
  "errors": {
    "0": "成功",
    "1": "未知错误",
    "1001": "请先登录",
    "4001": "订单不存在",
    "5002": "支付金额不足，还差 {{remaining}}"
  }
}
```

## 实现步骤

### Phase 1: shared crate
1. 创建 `shared/src/error/` 模块
2. 定义 ErrorCode 枚举
3. 实现 AppError, ApiResponse 类型
4. 实现 HTTP 状态码映射
5. 添加 build.rs 生成 TypeScript

### Phase 2: Edge-Server 迁移
1. 替换现有 AppError 为新版本
2. 更新所有 API handler
3. 保持向后兼容（code 字段格式）

### Phase 3: Tauri 迁移
1. 移除 error_codes.rs，使用 shared
2. 更新 ApiResponse 使用新格式
3. 更新所有 commands

### Phase 4: 前端迁移
1. 使用生成的 error-codes.ts
2. 补全 i18n 翻译
3. 更新错误处理工具函数
4. 移除旧的 ErrorCode 枚举

## 文件变更清单

### 新建
- `shared/src/error/mod.rs`
- `shared/src/error/codes.rs`
- `shared/src/error/types.rs`
- `shared/src/error/http.rs`
- `shared/src/error/category.rs`
- `shared/build.rs`
- `red_coral/src/generated/error-codes.ts`

### 修改
- `shared/Cargo.toml` - 添加 build.rs
- `edge-server/src/utils/error.rs` - 迁移到新系统
- `red_coral/src-tauri/src/core/error_codes.rs` - 删除，使用 shared
- `red_coral/src-tauri/src/core/response.rs` - 使用新 ApiResponse
- `red_coral/src/core/domain/types/api/error.ts` - 删除，使用生成文件
- `red_coral/src/infrastructure/i18n/locales/*.json` - 补全错误翻译
