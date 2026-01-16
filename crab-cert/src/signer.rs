use crate::crypto;
use crate::error::{CertError, Result};
use async_trait::async_trait;

/// 硬件提供商类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderType {
    Software,
    Tpm,
    Android,
    SecureEnclave,
}

/// 安全签名器接口
///
/// 这个 trait 抽象了底层硬件（或软件模拟）的签名能力。
/// 关键点是：调用者永远无法获取私钥本身，只能请求“对数据进行签名”。
#[async_trait]
pub trait SecureSigner: Send + Sync {
    /// 获取公钥 (DER 格式)
    fn public_key(&self) -> Result<Vec<u8>>;

    /// 获取公钥 (PEM 格式)
    fn public_key_pem(&self) -> Result<String>;

    /// 核心能力：让硬件帮我签名
    /// 此时私钥在 TPM/TEE 内部，数据进去，签名出来
    async fn sign(&self, data: &[u8]) -> Result<Vec<u8>>;

    /// 硬件类型 (用于日志/调试)
    fn provider_type(&self) -> ProviderType;
}

/// 软件模拟的签名器 (用于开发和测试，以及没有 TPM 的环境)
/// 私钥以文件或内存形式存在，但在 trait 层面被封装起来。
pub struct SoftwareSigner {
    priv_key_pem: String,
    pub_key_pem: String,
    // 我们缓存公钥 DER 以提高性能
    pub_key_der: Vec<u8>,
}

impl SoftwareSigner {
    /// 从 PEM 字符串加载
    pub fn new(priv_key_pem: String, pub_key_pem: String) -> Result<Self> {
        // 验证一下私钥是否有效
        // 这里简单解析一下，确保格式正确
        let _ = crypto::to_rustls_key(&priv_key_pem)?;

        // 解析公钥 DER
        let certs = crypto::to_rustls_certs(&pub_key_pem)?;
        if certs.is_empty() {
            return Err(CertError::VerificationFailed(
                "No public key/cert found".into(),
            ));
        }
        let pub_key_der = certs[0].as_ref().to_vec();

        Ok(Self {
            priv_key_pem,
            pub_key_pem,
            pub_key_der,
        })
    }

    /// 从文件加载
    pub fn from_files(priv_path: &str, pub_path: &str) -> Result<Self> {
        let priv_pem = std::fs::read_to_string(priv_path).map_err(|e| {
            CertError::VerificationFailed(format!("Failed to read private key: {}", e))
        })?;
        let pub_pem = std::fs::read_to_string(pub_path).map_err(|e| {
            CertError::VerificationFailed(format!("Failed to read public key: {}", e))
        })?;
        Self::new(priv_pem, pub_pem)
    }
}

#[async_trait]
impl SecureSigner for SoftwareSigner {
    fn public_key(&self) -> Result<Vec<u8>> {
        Ok(self.pub_key_der.clone())
    }

    fn public_key_pem(&self) -> Result<String> {
        Ok(self.pub_key_pem.clone())
    }

    async fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
        // 调用底层的 crypto::sign
        crypto::sign(&self.priv_key_pem, data)
    }

    fn provider_type(&self) -> ProviderType {
        ProviderType::Software
    }
}
