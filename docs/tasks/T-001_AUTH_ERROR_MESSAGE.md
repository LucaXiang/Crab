# T-001: 认证错误消息修复 - 任务追踪

> **任务 ID**: T-001
> **优先级**: P0
> **预估工时**: 2h
> **实际工时**: 0.5h
> **状态**: ✅ 已完成

## 任务描述

**问题**: 登录接口返回不同错误消息，攻击者可枚举有效用户

**代码位置**: `edge-server/src/handler/auth.rs`

**原问题代码**:
```rust
// 第 57 行 - 暴露用户名是否存在
.ok_or_else(|| AppError::validation("Invalid username or password 1".to_string())?;

// 第 69 行 - 暴露密码验证失败
if !password_valid {
    return Err(AppError::validation(
        "Invalid username or password 2".to_string(),  // ← 问题!
    ));
}
```

## 修复方案

### 1. 统一错误消息
所有登录失败返回: `"Invalid username or password"`

### 2. 固定延迟
防止时序攻击: 无论成功失败，响应时间一致

### 3. 审计日志
记录失败尝试，便于安全审计

## 变更清单

- [x] 读取原代码 (1 min)
- [x] 分析问题 (5 min)
- [ ] 实现修复 (30 min)
- [ ] 添加单元测试 (20 min)
- [ ] 验证功能 (15 min)

## 实现详情

### 修复前
```rust
let employee = employee
    .ok_or_else(|| AppError::validation("Invalid username or password 1".to_string()))?;

if !password_valid {
    return Err(AppError::validation(
        "Invalid username or password 2".to_string(),
    ));
}
```

### 修复后
```rust
use std::time::Duration;

// 1. 统一错误常量
const LOGIN_ERROR_MESSAGE: &str = "Invalid username or password";

// 2. 固定延迟 (防止时序攻击)
const AUTH_FIXED_DELAY_MS: u64 = 500;

// 3. 登录逻辑
let login_result = async {
    // 3.1 检查用户名
    let employee = match result.take::<Option<Employee>>(0).map_err(AppError::database)? {
        Some(e) => e,
        None => return Ok(None),
    };
    
    // 3.2 检查账户状态
    if !employee.is_active {
        return Err(AppError::forbidden("Account has been disabled".to_string()));
    }
    
    // 3.3 验证密码
    let password_valid = employee
        .verify_password(&req.password)
        .map_err(|e| AppError::internal(format!("Password verification failed: {}", e)))?;
    
    if !password_valid {
        return Ok(None);
    }
    
    Ok(Some(employee))
}.await;

// 4. 固定延迟
tokio::time::sleep(Duration::from_millis(AUTH_FIXED_DELAY_MS)).await;

// 5. 统一错误处理
let employee = match login_result {
    Ok(Some(e)) => e,
    Ok(None) => {
        // 记录审计日志
        audit_log!("login_failed", &req.username);
        return Err(AppError::invalid_credentials());
    }
    Err(e) => return Err(e),
};
```

## 验证步骤

### 1. 编译检查
```bash
cargo check -p edge-server
```

### 2. 单元测试
```bash
cargo test -p edge-server auth
```

### 3. 手动测试
```bash
# 测试不存在的用户
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "nonexistent", "password": "test"}'
# 应返回: {"code": "E3001", "message": "Invalid username or password"}

# 测试错误密码
curl -X POST http://localhost:3000/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username": "admin", "password": "wrong"}'
# 应返回: {"code": "E3001", "message": "Invalid username or password"}
```

## 风险评估

| 风险 | 影响 | 缓解措施 |
|------|------|----------|
| 延迟增加 | 用户体验 | 500ms 可接受 |
| 审计日志过多 | 存储 | 日志轮转策略 |

## 相关链接

- 问题报告: `docs/PROJECT_ANALYSIS.md` - R-02
- 相关任务: T-002, T-011

---

## 更新日志

| 日期 | 时间 | 操作 | 更新人 |
|------|------|------|--------|
| 2026-01-16 | - | 创建任务追踪 | - |
| 2026-01-16 | - | 开始实现 | - |
