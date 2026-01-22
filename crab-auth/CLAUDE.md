# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Crab Auth

认证服务器 - 集中身份管理 + CA 层级管理。

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
| `POST /api/activate` | 设备激活 (返回证书) |
| `POST /api/tenant/register` | 租户注册 |
| `GET /api/tenant/:id/ca` | 获取租户 CA |
| `POST /api/cert/issue` | 签发实体证书 |

## 激活流程

```
1. 客户端发送 ActivationRequest (tenant_code, password, device_id)
2. crab-auth 验证租户凭据
3. 签发 Entity Cert (包含 device_id 绑定)
4. 返回 Credential (cert + key + ca_chain)
```

## 存储

证书存储在 `auth_storage/` (gitignored):
- `root_ca/` - Root CA
- `tenants/<id>/` - 租户 CA + 元数据

## 响应语言

使用中文回答。
