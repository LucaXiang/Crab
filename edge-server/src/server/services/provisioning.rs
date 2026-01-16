//! 供应服务
//!
//! 处理与认证服务器的交互，以进行供应和激活。

use crate::common::AppError;
use crate::server::ServerState;
use crate::server::credential::{Credential, verify_cert_pair};
use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct ActivationResponse {
    success: bool,
    error: Option<String>,
    server_id: Option<String>,
    tenant_id: Option<String>,
    cert: Option<String>,
    key: Option<String>,
    #[serde(rename = "tenant_ca_cert")]
    tenant_ca_pem: Option<String>,
}

pub struct ProvisioningService {
    state: ServerState,
    auth_url: String,
    client: reqwest::Client,
}

impl ProvisioningService {
    pub fn new(state: ServerState, auth_url: String) -> Self {
        Self {
            state,
            auth_url,
            client: reqwest::Client::new(),
        }
    }

    /// 执行完整的激活流程：登录并激活 -> 颁发证书 -> 保存 -> 本地 Credential.json 激活
    pub async fn activate(&self, username: &str, password: &str) -> Result<(), AppError> {
        // 1. 获取 Hardware ID (Device ID)
        // 这是物理设备的唯一标识，必须上传给 Auth Server 绑定
        let device_id = crab_cert::generate_hardware_id();
        tracing::info!("Using Device ID for activation: {}", device_id);

        // 2. 调用 /api/server/activate
        // 该接口只需用户名密码，服务器会自动分配 ServerID 和 TenantID
        let resp = self
            .client
            .post(format!("{}/api/server/activate", self.auth_url))
            .json(&json!({
                "username": username,
                "password": password,
                "device_id": device_id,
            }))
            .send()
            .await
            .map_err(|e| AppError::internal(format!("Auth server connection failed: {}", e)))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::internal(format!(
                "Activation failed: {} - {}",
                status, text
            )));
        }

        let resp_data: ActivationResponse = resp
            .json()
            .await
            .map_err(|e| AppError::internal(format!("Invalid response: {}", e)))?;

        if !resp_data.success {
            let msg = resp_data.error.as_deref().unwrap_or("Unknown error");
            return Err(AppError::validation(format!("Activation failed: {}", msg)));
        }

        // 3. 解析响应
        let server_id = resp_data
            .server_id
            .ok_or_else(|| AppError::validation("Missing server_id"))?;
        let tenant_id = resp_data
            .tenant_id
            .ok_or_else(|| AppError::validation("Missing tenant_id"))?;
        let cert_pem = resp_data
            .cert
            .ok_or_else(|| AppError::validation("Missing cert"))?;
        let key_pem = resp_data
            .key
            .ok_or_else(|| AppError::validation("Missing key"))?;
        let tenant_ca_pem = resp_data
            .tenant_ca_pem
            .ok_or_else(|| AppError::validation("Missing tenant_ca_cert"))?;

        tracing::info!(
            "Activation successful. Assigned ServerID: {}, TenantID: {}",
            server_id,
            tenant_id
        );

        // 4. 验证证书链
        // 这里只是验证，不生成 Credential 对象，因为我们需要构造完整的 Credential
        verify_cert_pair(&cert_pem, &tenant_ca_pem)
            .map_err(|e| AppError::validation(format!("Certificate verification failed: {}", e)))?;

        // 我们需要获取指纹。
        let metadata = crab_cert::CertMetadata::from_pem(&cert_pem)
            .map_err(|e| AppError::validation(format!("Failed to parse cert metadata: {}", e)))?;

        // 检查 Server ID 是否匹配
        let cn = metadata.common_name.as_deref().unwrap_or("");
        if cn != server_id {
            return Err(AppError::validation(format!(
                "Server ID mismatch: expected {}, got CN={}",
                server_id, cn
            )));
        }

        // 5. 保存证书
        self.state
            .save_certificates(&tenant_ca_pem, &cert_pem, &key_pem)
            .await
            .map_err(|e| AppError::internal(format!("Failed to save certificates: {}", e)))?;

        // 6. 构造并保存 Credential
        let credential = Credential::new(
            tenant_id.to_string(),
            server_id.to_string(),
            Some(device_id),
            metadata.fingerprint_sha256,
        );

        // 通过 ActivationService 激活（保存 Credential.json 并通知）
        self.state.activation_service().activate(credential).await?;

        Ok(())
    }
}
