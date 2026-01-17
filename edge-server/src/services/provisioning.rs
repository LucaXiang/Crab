//! 供应服务
//!
//! 处理与认证服务器的交互，以进行供应和激活。
//! 使用 shared::activation 的标准激活响应格式。

use crate::core::ServerState;
use crate::services::tenant_binding::{TenantBinding, verify_cert_pair};
use crate::utils::AppError;
use serde_json::json;
use shared::activation::ActivationResponse;

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

    /// 执行完整的激活流程
    ///
    /// 1. 向 Auth Server 发送激活请求
    /// 2. Auth Server 返回签名的 ActivationData (包含证书 + SignedBinding)
    /// 3. 验证并保存证书
    /// 4. 构造 TenantBinding 并激活
    pub async fn activate(&self, username: &str, password: &str) -> Result<(), AppError> {
        // 1. 获取 Hardware ID (Device ID)
        let device_id = crab_cert::generate_hardware_id();
        tracing::info!("Using Device ID for activation: {}", device_id);

        // 2. 调用 /api/server/activate
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

        // 3. 解析激活响应
        let resp_data: ActivationResponse = resp
            .json()
            .await
            .map_err(|e| AppError::internal(format!("Invalid response: {}", e)))?;

        if !resp_data.success {
            let msg = resp_data.error.as_deref().unwrap_or("Unknown error");
            return Err(AppError::validation(format!("Activation failed: {}", msg)));
        }

        let data = resp_data
            .data
            .ok_or_else(|| AppError::validation("Missing activation data"))?;

        // 4. 验证证书链完整性
        tracing::info!("Verifying certificate chain: Root CA -> Tenant CA -> Edge Cert");
        self.state
            .cert_service()
            .verify_certificate_chain(&data.root_ca_cert, &data.tenant_ca_cert, &data.entity_cert)
            .await
            .map_err(|e| {
                AppError::validation(format!("Certificate chain verification failed: {}", e))
            })?;

        // 5. 验证证书 + 硬件绑定
        verify_cert_pair(&data.entity_cert, &data.tenant_ca_cert)
            .map_err(|e| AppError::validation(format!("Certificate verification failed: {}", e)))?;

        // 6. 验证 SignedBinding 签名
        data.binding
            .verify_signature(&data.tenant_ca_cert)
            .map_err(|e| AppError::validation(format!("Binding signature invalid: {}", e)))?;

        tracing::info!(
            "✅ Activation successful. Assigned ServerID: {}, TenantID: {}",
            data.entity_id,
            data.tenant_id
        );

        // 7. 保存证书
        self.state
            .save_certificates(
                &data.root_ca_cert,
                &data.tenant_ca_cert,
                &data.entity_cert,
                &data.entity_key,
            )
            .await
            .map_err(|e| AppError::internal(format!("Failed to save certificates: {}", e)))?;

        // 8. 构造 TenantBinding 并激活
        let credential = TenantBinding::from_signed(data.binding);

        // 通过 ActivationService 激活（保存 Credential.json 并通知）
        self.state.activation_service().activate(credential).await?;

        Ok(())
    }
}
