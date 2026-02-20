# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Crab Cert

PKI 证书管理库 - 三层 CA 层级 + 硬件绑定。

## 命令

```bash
cargo check -p crab-cert
cargo test -p crab-cert --lib
cargo run -p crab-cert --example mtls_demo
```

## 模块结构

```
src/
├── lib.rs        # 公开 API + write_secret_file (私钥文件 0o600 权限写入)
├── ca.rs         # CertificateAuthority - CA 证书生成
├── credential.rs # Credential - 实体证书 + 私钥
├── server.rs     # CertService - 证书服务
├── trust.rs      # 证书链验证
├── crypto.rs     # 加密/解密/签名
├── machine.rs    # 硬件 ID 生成
├── adapter.rs    # Rustls 适配器
├── profile.rs    # 证书配置文件
├── metadata.rs   # X.509 扩展元数据
└── signer.rs     # 签名器
```

## 三层 CA 层级

```
Root CA (crab-auth 持有)
    └── Tenant CA (每租户一个)
            └── Entity Cert (设备/服务器证书)
```

## 自定义 X.509 扩展

| OID | 字段 | 用途 |
|-----|------|------|
| `1.3.6.1.4.1.99999.1` | `tenant_id` | 租户标识 |
| `1.3.6.1.4.1.99999.2` | `device_id` | 硬件绑定 |
| `1.3.6.1.4.1.99999.5` | `client_name` | 客户端名称 |

## 核心类型

```rust
// 证书凭据
pub struct Credential {
    pub cert_pem: String,
    pub key_pem: String,
    pub ca_chain_pem: String,
}

// 证书服务
pub struct CertService {
    storage: CertStorage,
    ca: CertificateAuthority,
}
```

## 响应语言

使用中文回答。
