# CRAB-CERT MODULE

**Generated:** 2026-01-16T18:07:10Z
**Reason:** PKI证书管理，3000行代码，三级CA体系

## OVERVIEW
PKI证书管理库，三级CA体系（Root→Tenant→Entity），硬件绑定安全

## WHERE TO LOOK
| 任务 | 位置 | 备注 |
|------|------|------|
| CA管理 | src/lib.rs | 证书颁发和验证 |
| 证书生成 | src/cert.rs | X.509证书创建 |
| 硬件绑定 | src/credential.rs | device_id绑定 |
| 存储管理 | src/storage.rs | 证书持久化 |

## 证书架构
```
Root CA → Tenant CA → Entity Certificates
```

## 核心功能
- **三级证书链**: Root CA签发Tenant CA，Tenant CA签发实体证书
- **自定义扩展**: OID 1.3.6.1.4.1.99999.* (tenant_id/device_id/client_name)
- **硬件绑定**: 证书device_id与机器硬件ID匹配
- **离线支持**: 本地证书缓存，断网仍可验证

## 安全特性
- **FIPS合规**: 使用aws-lc-rs加密后端
- **硬件安全**: 支持TPM 2.0/Secure Enclave
- **证书验证**: 三层验证链（TLS握手+身份验证+硬件绑定）

## CONVENTIONS
- **证书存储**: `auth_storage/`目录（gitignore）
- **密钥保护**: 私钥本地加密存储
- **验证顺序**: CA链→身份→硬件绑定

## ANTI-PATTERNS
- ❌ 过期证书使用：需检查有效期
- ❌ 明文私钥：必须加密存储
- ❌ 硬件ID伪造：需加强验证机制

## 示例使用
```bash
# mTLS演示
cargo run -p crab-cert --example mtls_demo
```

## 注意事项
- **证书更新**: 定期轮换，长期证书风险
- **私钥管理**: 严禁硬编码，使用环境变量
- **硬件检测**: machine_id生成逻辑需验证