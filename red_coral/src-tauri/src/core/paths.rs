//! TenantPaths - 租户目录路径管理
//!
//! 集中管理租户数据目录的所有路径常量和构建函数。
//!
//! ## 目录结构
//!
//! ```text
//! {tenant-id}/
//! ├── auth/                    # [共用] 认证相关
//! │   └── session.json         # 员工会话缓存
//! │
//! ├── certs/                   # [Client 模式] 客户端证书 (CertManager 兼容)
//! │   ├── credential.json      # 客户端凭证 (CertManager 使用)
//! │   ├── entity.crt           # 客户端证书
//! │   ├── entity.key           # 客户端私钥
//! │   └── tenant_ca.crt        # Tenant CA
//! │
//! ├── cache/                   # [Client 模式] 本地缓存
//! │   └── images/              # 图片缓存 (只读)
//! │
//! └── server/                  # [Server 模式] 服务器数据 (edge-server work_dir)
//!     ├── credential.json      # 租户绑定凭证 (小写, 与edge-server一致)
//!     ├── certs/               # Edge Server 证书 (work_dir/certs/)
//!     │   ├── root_ca.pem      # Root CA
//!     │   ├── tenant_ca.pem    # Tenant CA
//!     │   ├── edge_cert.pem    # Edge Server 证书
//!     │   └── edge_key.pem     # Edge Server 私钥
//!     ├── data/
//!     │   ├── main.db           # SQLite 数据库
//!     │   ├── orders.redb      # 订单 Event Sourcing
//!     │   └── print.redb       # 打印队列
//!     └── images/              # 图片存储 (可写)
//!         └── {hash}.jpg
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

    /// 凭证文件: {tenant}/server/credential.json
    ///
    /// 注意：文件名必须与 edge-server 的 CREDENTIAL_FILE 常量一致 (小写 c)
    /// 位于 server 目录，因为 edge-server 从 work_dir 读取
    pub fn credential_file(&self) -> PathBuf {
        self.server_dir().join("credential.json")
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
    /// edge-server 从 work_dir/certs/ 读取证书，work_dir = {tenant}/server/
    pub fn server_certs_dir(&self) -> PathBuf {
        self.server_dir().join("certs")
    }

    // ============ 具体文件路径 ============

    /// SQLite 数据库路径: {tenant}/server/data/main.db
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

    // ============ 证书文件路径 (Client 模式, CertManager 兼容) ============

    /// 客户端凭证: {tenant}/certs/credential.json
    ///
    /// CertManager 使用此位置存储凭证
    pub fn client_credential(&self) -> PathBuf {
        self.certs_dir().join("credential.json")
    }

    /// 客户端证书: {tenant}/certs/entity.crt
    ///
    /// 文件名与 CertManager 保持一致
    pub fn client_cert(&self) -> PathBuf {
        self.certs_dir().join("entity.crt")
    }

    /// 客户端密钥: {tenant}/certs/entity.key
    ///
    /// 文件名与 CertManager 保持一致
    pub fn client_key(&self) -> PathBuf {
        self.certs_dir().join("entity.key")
    }

    /// 客户端 Tenant CA: {tenant}/certs/tenant_ca.crt
    ///
    /// 文件名与 CertManager 保持一致
    pub fn client_tenant_ca(&self) -> PathBuf {
        self.certs_dir().join("tenant_ca.crt")
    }

    /// Edge Server 证书: {tenant}/server/certs/edge_cert.pem
    ///
    /// 位于 server/certs/ 目录，edge-server 从 work_dir/certs/ 读取
    pub fn edge_cert(&self) -> PathBuf {
        self.server_certs_dir().join("edge_cert.pem")
    }

    /// Edge Server 密钥: {tenant}/server/certs/edge_key.pem
    pub fn edge_key(&self) -> PathBuf {
        self.server_certs_dir().join("edge_key.pem")
    }

    /// Server Root CA: {tenant}/server/certs/root_ca.pem
    pub fn server_root_ca(&self) -> PathBuf {
        self.server_certs_dir().join("root_ca.pem")
    }

    /// Server Tenant CA: {tenant}/server/certs/tenant_ca.pem
    pub fn server_tenant_ca(&self) -> PathBuf {
        self.server_certs_dir().join("tenant_ca.pem")
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

    // ============ 证书存在性检查 ============

    /// 检查 Client 模式证书是否存在
    ///
    /// 检查 entity.crt, entity.key, tenant_ca.crt 三个文件
    pub fn has_client_certificates(&self) -> bool {
        self.client_cert().exists()
            && self.client_key().exists()
            && self.client_tenant_ca().exists()
    }

    /// 检查 Server 模式证书是否存在
    ///
    /// 检查 edge_cert.pem, edge_key.pem, tenant_ca.pem 三个文件
    pub fn has_server_certificates(&self) -> bool {
        self.edge_cert().exists() && self.edge_key().exists() && self.server_tenant_ca().exists()
    }

    /// 检查 Server 模式凭证是否存在
    pub fn has_server_credential(&self) -> bool {
        self.credential_file().exists()
    }

    /// 检查 Server 模式是否已激活 (证书 + 凭证都存在)
    pub fn is_server_activated(&self) -> bool {
        self.has_server_certificates() && self.has_server_credential()
    }

    // ============ 证书删除 ============

    /// 删除 Server 模式证书和凭证
    pub fn delete_server_certs(&self) -> std::io::Result<()> {
        let certs_dir = self.server_certs_dir();
        if certs_dir.exists() {
            std::fs::remove_dir_all(&certs_dir)?;
        }
        let cred = self.credential_file();
        if cred.exists() {
            std::fs::remove_file(&cred)?;
        }
        Ok(())
    }

    /// 删除 Client 模式证书
    pub fn delete_client_certs(&self) -> std::io::Result<()> {
        let certs_dir = self.certs_dir();
        if certs_dir.exists() {
            std::fs::remove_dir_all(&certs_dir)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paths() {
        let paths = TenantPaths::new("/data/tenants/tenant-123");

        // 共用路径
        assert_eq!(
            paths.auth_dir(),
            PathBuf::from("/data/tenants/tenant-123/auth")
        );
        assert_eq!(
            paths.credential_file(),
            PathBuf::from("/data/tenants/tenant-123/server/credential.json")
        );
        assert_eq!(
            paths.session_file(),
            PathBuf::from("/data/tenants/tenant-123/auth/session.json")
        );

        // Client 证书 (CertManager 兼容文件名)
        assert_eq!(
            paths.certs_dir(),
            PathBuf::from("/data/tenants/tenant-123/certs")
        );
        assert_eq!(
            paths.client_credential(),
            PathBuf::from("/data/tenants/tenant-123/certs/credential.json")
        );
        assert_eq!(
            paths.client_cert(),
            PathBuf::from("/data/tenants/tenant-123/certs/entity.crt")
        );
        assert_eq!(
            paths.client_key(),
            PathBuf::from("/data/tenants/tenant-123/certs/entity.key")
        );
        assert_eq!(
            paths.client_tenant_ca(),
            PathBuf::from("/data/tenants/tenant-123/certs/tenant_ca.crt")
        );

        // Cache
        assert_eq!(
            paths.cache_images_dir(),
            PathBuf::from("/data/tenants/tenant-123/cache/images")
        );

        // Server 路径
        assert_eq!(
            paths.server_dir(),
            PathBuf::from("/data/tenants/tenant-123/server")
        );
        assert_eq!(
            paths.server_images_dir(),
            PathBuf::from("/data/tenants/tenant-123/server/images")
        );
        assert_eq!(
            paths.server_certs_dir(),
            PathBuf::from("/data/tenants/tenant-123/server/certs")
        );
        assert_eq!(
            paths.main_db_dir(),
            PathBuf::from("/data/tenants/tenant-123/server/data/main.db")
        );
        assert_eq!(
            paths.orders_db_file(),
            PathBuf::from("/data/tenants/tenant-123/server/data/orders.redb")
        );

        // Edge Server 证书 (在 server/certs/ 下)
        assert_eq!(
            paths.edge_cert(),
            PathBuf::from("/data/tenants/tenant-123/server/certs/edge_cert.pem")
        );
        assert_eq!(
            paths.edge_key(),
            PathBuf::from("/data/tenants/tenant-123/server/certs/edge_key.pem")
        );
        assert_eq!(
            paths.server_root_ca(),
            PathBuf::from("/data/tenants/tenant-123/server/certs/root_ca.pem")
        );
        assert_eq!(
            paths.server_tenant_ca(),
            PathBuf::from("/data/tenants/tenant-123/server/certs/tenant_ca.pem")
        );
    }
}
