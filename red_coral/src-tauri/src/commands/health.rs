//! 健康检查命令

use std::sync::Arc;
use tauri::State;
use tokio::sync::RwLock;

use crate::core::bridge::ClientBridge;
use crate::core::response::ApiResponse;
use shared::app_state::{
    CertificateHealth, ComponentsHealth, DatabaseHealth, DeviceInfo, HealthLevel, HealthStatus,
    NetworkHealth, SubscriptionHealth,
};
use time::OffsetDateTime;

/// 获取系统健康状态
#[tauri::command]
pub async fn get_health_status(
    bridge: State<'_, Arc<RwLock<ClientBridge>>>,
) -> Result<ApiResponse<HealthStatus>, String> {
    let bridge = bridge.read().await;
    let tenant_manager = bridge.tenant_manager().read().await;

    // 获取设备信息
    let device_id = crab_cert::generate_hardware_id();
    let tenant_id = tenant_manager.current_tenant_id().map(|s| s.to_string());

    // 检查证书健康状态
    let certificate = if let Some(cm) = tenant_manager.current_cert_manager() {
        if cm.has_local_certificates() {
            match cm.load_local_certificates() {
                Ok((cert_pem, _, _)) => match crab_cert::CertMetadata::from_pem(&cert_pem) {
                    Ok(metadata) => {
                        let now = OffsetDateTime::now_utc();
                        let duration = metadata.not_after - now;
                        let days_remaining = duration.whole_days();
                        let status = if days_remaining < 0 {
                            HealthLevel::Critical
                        } else if days_remaining <= 30 {
                            HealthLevel::Warning
                        } else {
                            HealthLevel::Healthy
                        };
                        CertificateHealth {
                            status,
                            expires_at: Some(metadata.not_after.to_string()),
                            days_remaining: Some(days_remaining),
                            fingerprint: Some(metadata.fingerprint_sha256.clone()),
                            issuer: metadata.common_name.clone(),
                        }
                    }
                    Err(_) => CertificateHealth {
                        status: HealthLevel::Critical,
                        expires_at: None,
                        days_remaining: None,
                        fingerprint: None,
                        issuer: None,
                    },
                },
                Err(_) => CertificateHealth {
                    status: HealthLevel::Critical,
                    expires_at: None,
                    days_remaining: None,
                    fingerprint: None,
                    issuer: None,
                },
            }
        } else {
            CertificateHealth {
                status: HealthLevel::Unknown,
                expires_at: None,
                days_remaining: None,
                fingerprint: None,
                issuer: None,
            }
        }
    } else {
        CertificateHealth {
            status: HealthLevel::Unknown,
            expires_at: None,
            days_remaining: None,
            fingerprint: None,
            issuer: None,
        }
    };

    // 检查订阅健康状态 (简化)
    let subscription = SubscriptionHealth {
        status: HealthLevel::Unknown,
        plan: None,
        subscription_status: None,
        signature_valid_until: None,
        needs_refresh: false,
    };

    // 检查网络健康状态 (简化)
    let network = NetworkHealth {
        status: HealthLevel::Unknown,
        auth_server_reachable: false,
        last_connected_at: None,
    };

    // 检查数据库健康状态 (简化)
    let database = DatabaseHealth {
        status: HealthLevel::Healthy,
        size_bytes: None,
        last_write_at: None,
    };

    // 计算整体健康状态
    let overall = if certificate.status == HealthLevel::Critical {
        HealthLevel::Critical
    } else if certificate.status == HealthLevel::Warning {
        HealthLevel::Warning
    } else {
        HealthLevel::Healthy
    };

    let health = HealthStatus {
        overall,
        components: ComponentsHealth {
            certificate,
            subscription,
            network,
            database,
        },
        checked_at: chrono::Utc::now().to_rfc3339(),
        device_info: DeviceInfo {
            device_id: format!("{}...", &device_id[..8]),
            entity_id: None,
            tenant_id,
        },
    };

    Ok(ApiResponse::success(health))
}
