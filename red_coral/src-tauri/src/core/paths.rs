//! TenantPaths - 租户目录路径管理
//!
//! 集中管理租户数据目录的所有路径常量和构建函数。
//!
//! ## 目录结构
//!
//! ```text
//! {tenant-id}/
//! ├── auth/                    # [共用] 认证相关
//! │   ├── credential.json      # 登录凭证
//! │   └── session.json         # 当前会话 + 会话缓存
//! │
//! ├── certs/                   # [共用] 客户端证书 (mTLS)
//! │   ├── root_ca.pem
//! │   ├── tenant_ca.pem
//! │   ├── cert.pem
//! │   └── key.pem
//! │
//! ├── cache/                   # [Client 模式] 本地缓存
//! │   └── images/              # 图片缓存 (只读)
//! │
//! └── server/                  # [Server 模式] 服务器数据
//!     ├── data/
//!     │   ├── main.db/         # SurrealDB (RocksDB 目录)
//!     │   ├── orders.redb      # 订单 Event Sourcing
//!     │   └── print.redb       # 打印队列
//!     ├── images/              # 图片存储 (可写)
//!     │   └── {hash}.jpg
//!     └── certs/               # 边缘服务器证书
//!         ├── edge_cert.pem
//!         └── edge_key.pem
//! ```

use std::path::{Path, PathBuf};

/// 租户路径管理器
///
/// 提供租户数据目录下所有路径的统一访问。
#[derive(Debug, Clone)]
pub struct TenantPaths {
    /// 租户目录根路径
    base: PathBuf,
}

impl TenantPaths {
    /// 创建新的 TenantPaths
    pub fn new(tenant_path: impl Into<PathBuf>) -> Self {
        Self {
            base: tenant_path.into(),
        }
    }

    /// 获取租户目录根路径
    pub fn base(&self) -> &Path {
        &self.base
    }

    // ============ 共用路径 ============

    /// 认证目录: {tenant}/auth/
    pub fn auth_dir(&self) -> PathBuf {
        self.base.join("auth")
    }

    /// 凭证文件: {tenant}/auth/Credential.json
    ///
    /// 注意：文件名必须与 edge-server 的 CREDENTIAL_FILE 常量一致 (大写 C)
    pub fn credential_file(&self) -> PathBuf {
        self.auth_dir().join("Credential.json")
    }

    /// 会话缓存文件: {tenant}/auth/session.json
    ///
    /// 包含员工会话缓存和当前活动会话
    pub fn session_file(&self) -> PathBuf {
        self.auth_dir().join("session.json")
    }

    /// 客户端证书目录: {tenant}/certs/
    ///
    /// 用于 mTLS 客户端身份验证
    pub fn certs_dir(&self) -> PathBuf {
        self.base.join("certs")
    }

    // ============ Client 模式路径 ============

    /// 缓存目录: {tenant}/cache/
    pub fn cache_dir(&self) -> PathBuf {
        self.base.join("cache")
    }

    /// 图片缓存目录: {tenant}/cache/images/
    pub fn cache_images_dir(&self) -> PathBuf {
        self.cache_dir().join("images")
    }

    // ============ Server 模式路径 ============

    /// 服务器数据目录: {tenant}/server/
    pub fn server_dir(&self) -> PathBuf {
        self.base.join("server")
    }

    /// 服务器数据存储目录: {tenant}/server/data/
    pub fn server_data_dir(&self) -> PathBuf {
        self.server_dir().join("data")
    }

    /// 服务器图片目录: {tenant}/server/images/
    pub fn server_images_dir(&self) -> PathBuf {
        self.server_dir().join("images")
    }

    /// 服务器证书目录: {tenant}/server/certs/
    ///
    /// 存放 Edge Server 的证书 (edge_cert.pem, edge_key.pem)
    pub fn server_certs_dir(&self) -> PathBuf {
        self.server_dir().join("certs")
    }

    // ============ 具体文件路径 ============

    /// SurrealDB 数据目录: {tenant}/server/data/main.db/
    pub fn main_db_dir(&self) -> PathBuf {
        self.server_data_dir().join("main.db")
    }

    /// 订单 Event Sourcing 数据库: {tenant}/server/data/orders.redb
    pub fn orders_db_file(&self) -> PathBuf {
        self.server_data_dir().join("orders.redb")
    }

    /// 打印队列数据库: {tenant}/server/data/print.redb
    pub fn print_db_file(&self) -> PathBuf {
        self.server_data_dir().join("print.redb")
    }

    // ============ 证书文件路径 ============

    /// Root CA 证书: {tenant}/certs/root_ca.pem
    pub fn root_ca_cert(&self) -> PathBuf {
        self.certs_dir().join("root_ca.pem")
    }

    /// Tenant CA 证书: {tenant}/certs/tenant_ca.pem
    pub fn tenant_ca_cert(&self) -> PathBuf {
        self.certs_dir().join("tenant_ca.pem")
    }

    /// 客户端证书: {tenant}/certs/cert.pem
    pub fn client_cert(&self) -> PathBuf {
        self.certs_dir().join("cert.pem")
    }

    /// 客户端密钥: {tenant}/certs/key.pem
    pub fn client_key(&self) -> PathBuf {
        self.certs_dir().join("key.pem")
    }

    /// Edge Server 证书: {tenant}/server/certs/edge_cert.pem
    pub fn edge_cert(&self) -> PathBuf {
        self.server_certs_dir().join("edge_cert.pem")
    }

    /// Edge Server 密钥: {tenant}/server/certs/edge_key.pem
    pub fn edge_key(&self) -> PathBuf {
        self.server_certs_dir().join("edge_key.pem")
    }

    // ============ 目录创建辅助 ============

    /// 确保共用目录存在
    pub fn ensure_common_dirs(&self) -> std::io::Result<()> {
        std::fs::create_dir_all(self.auth_dir())?;
        std::fs::create_dir_all(self.certs_dir())?;
        Ok(())
    }

    /// 确保 Client 模式目录存在
    pub fn ensure_client_dirs(&self) -> std::io::Result<()> {
        self.ensure_common_dirs()?;
        std::fs::create_dir_all(self.cache_images_dir())?;
        Ok(())
    }

    /// 确保 Server 模式目录存在
    pub fn ensure_server_dirs(&self) -> std::io::Result<()> {
        self.ensure_common_dirs()?;
        std::fs::create_dir_all(self.server_data_dir())?;
        std::fs::create_dir_all(self.server_images_dir())?;
        std::fs::create_dir_all(self.server_certs_dir())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths() {
        let paths = TenantPaths::new("/data/tenants/tenant-123");

        assert_eq!(paths.auth_dir(), PathBuf::from("/data/tenants/tenant-123/auth"));
        assert_eq!(paths.credential_file(), PathBuf::from("/data/tenants/tenant-123/auth/Credential.json"));
        assert_eq!(paths.session_file(), PathBuf::from("/data/tenants/tenant-123/auth/session.json"));
        assert_eq!(paths.certs_dir(), PathBuf::from("/data/tenants/tenant-123/certs"));
        assert_eq!(paths.cache_images_dir(), PathBuf::from("/data/tenants/tenant-123/cache/images"));
        assert_eq!(paths.server_dir(), PathBuf::from("/data/tenants/tenant-123/server"));
        assert_eq!(paths.server_images_dir(), PathBuf::from("/data/tenants/tenant-123/server/images"));
        assert_eq!(paths.main_db_dir(), PathBuf::from("/data/tenants/tenant-123/server/data/main.db"));
        assert_eq!(paths.orders_db_file(), PathBuf::from("/data/tenants/tenant-123/server/data/orders.redb"));
        assert_eq!(paths.edge_cert(), PathBuf::from("/data/tenants/tenant-123/server/certs/edge_cert.pem"));
    }
}
