# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Crab Auth

认证服务器 — 集中身份管理 + CA 层级管理 + 订阅状态校验。

## 命令

```bash
cargo check -p crab-auth
cargo run -p crab-auth     # 启动服务器 (Port 3001)
```

## 模块结构

```
src/
├── main.rs   # 入口
├── api.rs    # HTTP API 路由
└── state.rs  # AppState
```

## API 端点

| 端点 | 用途 |
|------|------|
| `POST /api/activate` | 设备激活 (验证租户 + 签发证书 + 返回订阅信息) |
| `POST /api/tenant/register` | 租户注册 |
| `GET /api/tenant/:id/ca` | 获取租户 CA |
| `POST /api/cert/issue` | 签发实体证书 |

## 激活流程

```
1. 客户端发送 ActivationRequest (tenant_code, password, device_id)
2. crab-auth 验证租户凭据
3. 检查订阅状态 (SubscriptionInfo)
4. 签发 Entity Cert (包含 device_id 绑定)
5. 生成 SignedBinding (硬件绑定 + 签名)
6. 返回: Credential + SignedBinding + SubscriptionInfo
```

## 订阅系统

**SubscriptionStatus**: Inactive → Active → PastDue → Expired / Canceled / Unpaid

**PlanType**:
| Plan | max_stores | 说明 |
|------|------------|------|
| Basic | 1 | 单店 |
| Pro | 3 | 连锁 |
| Enterprise | 无限 | 企业 |

**SignedBinding**:
- 硬件绑定: device_id + fingerprint
- 时钟篡改检测: 后退 ≤1小时，前进 ≤30天
- 签名验证: 防止绑定信息被篡改

## 存储

证书存储在 `auth_storage/` (gitignored):
- `root_ca/` - Root CA
- `tenants/<id>/` - 租户 CA + 元数据 + 订阅信息

## 响应语言

使用中文回答。
