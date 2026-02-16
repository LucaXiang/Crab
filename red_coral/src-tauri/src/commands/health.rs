//! 健康检查命令

use std::sync::Arc;
use tauri::State;

use crate::core::bridge::ClientBridge;
use crate::core::response::ApiResponse;
use shared::app_state::{
    CertificateHealth, ComponentsHealth, DeviceInfo, HealthLevel, HealthStatus,
};

/// 获取系统健康状态
#[tauri::command]
pub async fn get_health_status(
    bridge: State<'_, Arc<ClientBridge>>,
) -> Result<ApiResponse<HealthStatus>, String> {
    let tenant_manager = bridge.tenant_manager().read().await;

    // 获取设备信息
    let device_id = crab_cert::generate_hardware_id();

    // 检查证书健康状态 (Server 模式使用 edge_cert)
    let certificate = if let Some(paths) = tenant_manager.current_paths() {
        if paths.has_server_certificates() {
            match std::fs::read_to_string(paths.edge_cert()) {
                Ok(cert_pem) => match crab_cert::CertMetadata::from_pem(&cert_pem) {
                    Ok(metadata) => {
                        let now = time::OffsetDateTime::now_utc();
                        let duration = metadata.not_after - now;
                        let days_remaining = duration.whole_days();
                        let status = if days_remaining < 0 {
                            HealthLevel::Critical
                        } else if days_remaining <= 30 {
                            HealthLevel::Warning
                        } else {
                            HealthLevel::Healthy
                        };
                        let expires_at_millis = metadata.not_after.unix_timestamp() * 1000
                            + metadata.not_after.millisecond() as i64;
                        CertificateHealth {
                            status,
                            expires_at: Some(expires_at_millis),
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

    // 释放 tenant_manager 锁
    drop(tenant_manager);

    // 获取订阅、网络、数据库健康状态
    let (subscription, network, database) = bridge.get_health_components().await;

    // 计算整体健康状态 (考虑所有组件)
    let overall = {
        let statuses = [
            &certificate.status,
            &subscription.status,
            &network.status,
            &database.status,
        ];

        if statuses.iter().any(|s| **s == HealthLevel::Critical) {
            HealthLevel::Critical
        } else if statuses.iter().any(|s| **s == HealthLevel::Warning) {
            HealthLevel::Warning
        } else if statuses.iter().all(|s| **s == HealthLevel::Unknown) {
            HealthLevel::Unknown
        } else {
            HealthLevel::Healthy
        }
    };

    let health = HealthStatus {
        overall,
        components: ComponentsHealth {
            certificate,
            subscription,
            network,
            database,
        },
        checked_at: shared::util::now_millis(),
        device_info: DeviceInfo {
            device_id: format!("{}...", &device_id[..8]),
            entity_id: None,
            tenant_id: bridge
                .tenant_manager()
                .read()
                .await
                .current_tenant_id()
                .map(|s| s.to_string()),
        },
    };

    Ok(ApiResponse::success(health))
}
