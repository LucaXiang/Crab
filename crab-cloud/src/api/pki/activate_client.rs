use crate::auth::tenant_auth;
use crate::db::{client_connections, p12, subscriptions, tenants};
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use crab_cert::{CertMetadata, CertProfile};
use shared::activation::{
    ActivationData, ActivationResponse, ActiveDevice, EntityType, QuotaInfo, SignedBinding,
    SubscriptionInfo,
};
use shared::error::ErrorCode;

use super::activate::{parse_plan_type, parse_subscription_status};

#[derive(serde::Deserialize)]
pub struct ActivateClientRequest {
    /// JWT session token (from /api/tenant/verify)
    pub token: String,
    pub device_id: String,
    pub replace_entity_id: Option<String>,
    /// 客户端名称 (POS, KDS, etc.)
    pub client_name: Option<String>,
}

pub async fn activate_client(
    State(state): State<AppState>,
    Json(req): Json<ActivateClientRequest>,
) -> Json<ActivationResponse> {
    let tenant_id = match tenant_auth::verify_token(&req.token, &state.jwt_secret) {
        Ok(claims) => claims.sub,
        Err(_) => {
            return Json(fail(ErrorCode::TokenExpired, "Invalid or expired token"));
        }
    };

    let tenant = match tenants::find_by_id(&state.pool, &tenant_id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return Json(fail(
                ErrorCode::TenantCredentialsInvalid,
                "Tenant not found",
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error finding tenant");
            return Json(fail(ErrorCode::InternalError, "Internal error"));
        }
    };

    // 获取最新订阅（不过滤 status）
    // 激活 = 证书签发 + 设备绑定，不应因订阅状态而拒绝。
    // 订阅检查由 edge-server 运行时处理。
    let sub = match subscriptions::get_latest_subscription(&state.pool, &tenant.id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Json(fail(
                ErrorCode::TenantNoSubscription,
                "No subscription found",
            ));
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error fetching subscription");
            return Json(fail(ErrorCode::InternalError, "Internal error"));
        }
    };

    let sub_status = parse_subscription_status(&sub.status);

    let plan = parse_plan_type(&sub.plan);
    let max_clients = sub.max_clients;

    let existing =
        match client_connections::find_by_device(&state.pool, &tenant.id, &req.device_id).await {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(error = %e, "Database error checking existing client");
                return Json(fail(ErrorCode::InternalError, "Internal error"));
            }
        };

    let (entity_id, is_reactivation) = if let Some(ref existing) = existing {
        if existing.status == "active" || existing.status == "deactivated" {
            (existing.entity_id.clone(), true)
        } else {
            (format!("client-{}", uuid::Uuid::new_v4()), false)
        }
    } else {
        (format!("client-{}", uuid::Uuid::new_v4()), false)
    };

    // === CA 操作（幂等，可在事务外执行）===

    let root_ca = match state.ca_store.get_or_create_root_ca().await {
        Ok(ca) => ca,
        Err(e) => {
            tracing::error!(error = %e, "Root CA error");
            return Json(fail(ErrorCode::AuthServerError, "Internal error"));
        }
    };

    let tenant_ca = match state
        .ca_store
        .get_or_create_tenant_ca(&tenant.id, &root_ca)
        .await
    {
        Ok(ca) => ca,
        Err(e) => {
            tracing::error!(error = %e, tenant_id = %tenant.id, "Tenant CA error");
            return Json(fail(ErrorCode::AuthServerError, "Internal error"));
        }
    };

    // Client 证书只需要 ClientAuth EKU（POS 终端作为 mTLS 客户端连接 edge-server）
    let profile = CertProfile::new_client(
        &entity_id,
        Some(tenant.id.clone()),
        Some(req.device_id.clone()),
        req.client_name.clone(),
    );

    let (entity_cert, entity_key) = match tenant_ca.issue_cert(&profile) {
        Ok(pair) => pair,
        Err(e) => {
            tracing::error!(error = %e, "Failed to issue certificate");
            return Json(fail(ErrorCode::AuthServerError, "Internal error"));
        }
    };

    let fingerprint = match CertMetadata::from_pem(&entity_cert) {
        Ok(meta) => meta.fingerprint_sha256,
        Err(e) => {
            tracing::error!(error = %e, "Certificate metadata error");
            return Json(fail(ErrorCode::AuthServerError, "Internal error"));
        }
    };

    let binding = SignedBinding::new(
        &entity_id,
        &tenant.id,
        &req.device_id,
        &fingerprint,
        EntityType::Client,
    );

    let signed_binding = match binding.sign(&tenant_ca.key_pem()) {
        Ok(b) => b,
        Err(e) => {
            tracing::error!(error = %e, "Failed to sign binding");
            return Json(fail(ErrorCode::AuthServerError, "Internal error"));
        }
    };

    let signature_valid_until = shared::util::now_millis() + 7 * 24 * 60 * 60 * 1000;
    let subscription_info = SubscriptionInfo {
        tenant_id: tenant.id.clone(),
        id: Some(sub.id.clone()),
        status: sub_status,
        plan,
        starts_at: shared::util::now_millis(),
        expires_at: sub.current_period_end,
        features: sub.features.clone(),
        max_stores: plan.max_stores() as u32,
        max_clients: max_clients as u32,
        cancel_at_period_end: sub.cancel_at_period_end,
        billing_interval: sub.billing_interval.clone(),
        signature_valid_until,
        signature: String::new(),
        last_checked_at: 0,
        p12: match p12::get_p12_info(&state.pool, &tenant.id).await {
            Ok(info) => Some(info),
            Err(e) => {
                tracing::warn!(error = %e, "Failed to query P12 info, defaulting to None");
                None
            }
        },
    };

    let signed_subscription = match subscription_info.sign(&tenant_ca.key_pem()) {
        Ok(s) => s,
        Err(e) => {
            tracing::error!(error = %e, "Failed to sign subscription");
            return Json(fail(ErrorCode::AuthServerError, "Internal error"));
        }
    };

    // === 配额检查 + 写入在事务内完成（advisory lock 防止并发超配额）===

    let mut tx = match state.pool.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            tracing::error!(error = %e, "Failed to begin transaction");
            return Json(fail(ErrorCode::InternalError, "Internal error"));
        }
    };

    if let Err(e) = client_connections::acquire_activation_lock(&mut tx, &tenant.id).await {
        tracing::error!(error = %e, "Failed to acquire client activation lock");
        return Json(fail(ErrorCode::InternalError, "Internal error"));
    }

    if !is_reactivation && max_clients > 0 {
        let active_count = match client_connections::count_active_in_tx(&mut tx, &tenant.id).await {
            Ok(n) => n,
            Err(e) => {
                tracing::error!(error = %e, "Database error counting active clients");
                return Json(fail(ErrorCode::InternalError, "Internal error"));
            }
        };

        if active_count >= max_clients as i64 {
            if let Some(ref replace_id) = req.replace_entity_id {
                let replace_target =
                    client_connections::find_by_entity(&state.pool, replace_id).await;
                match replace_target {
                    Ok(Some(target))
                        if target.tenant_id == tenant.id && target.status == "active" =>
                    {
                        if let Err(e) =
                            client_connections::mark_replaced_in_tx(&mut tx, replace_id, &entity_id)
                                .await
                        {
                            tracing::error!(error = %e, "Failed to mark client as replaced");
                            return Json(fail(
                                ErrorCode::InternalError,
                                "Failed to replace client",
                            ));
                        }
                    }
                    _ => {
                        return Json(fail(ErrorCode::ValidationFailed, "Invalid replace target"));
                    }
                }
            } else {
                let active_clients =
                    match client_connections::list_active(&state.pool, &tenant.id).await {
                        Ok(list) => list
                            .into_iter()
                            .map(|c| ActiveDevice {
                                entity_id: c.entity_id,
                                device_id: c.device_id,
                                activated_at: c.activated_at,
                                last_refreshed_at: c.last_refreshed_at,
                            })
                            .collect(),
                        Err(e) => {
                            tracing::error!(error = %e, "Database error listing active clients");
                            return Json(fail(ErrorCode::InternalError, "Internal error"));
                        }
                    };

                return Json(ActivationResponse {
                    success: false,
                    error: Some("client_limit_reached".to_string()),
                    error_code: Some(ErrorCode::ClientLimitReached),
                    data: None,
                    quota_info: Some(QuotaInfo {
                        max_slots: max_clients as u32,
                        active_count: active_count as u32,
                        active_devices: active_clients,
                    }),
                });
            }
        }
    }

    if let Err(e) = client_connections::insert_in_tx(
        &mut tx,
        &entity_id,
        &tenant.id,
        &req.device_id,
        &fingerprint,
    )
    .await
    {
        tracing::error!(error = %e, "Failed to write client connection record");
        return Json(fail(ErrorCode::InternalError, "Internal error"));
    }

    if let Err(e) = tx.commit().await {
        tracing::error!(error = %e, "Failed to commit client activation transaction");
        return Json(fail(ErrorCode::InternalError, "Internal error"));
    }

    tracing::info!(
        entity_id = %entity_id,
        tenant_id = %tenant.id,
        "Activated client"
    );

    Json(ActivationResponse {
        success: true,
        error: None,
        error_code: None,
        data: Some(ActivationData {
            entity_id,
            tenant_id: tenant.id,
            device_id: req.device_id,
            root_ca_cert: root_ca.cert_pem().to_string(),
            tenant_ca_cert: tenant_ca.cert_pem().to_string(),
            entity_cert,
            entity_key,
            binding: signed_binding,
            subscription: Some(signed_subscription),
        }),
        quota_info: None,
    })
}

fn fail(code: ErrorCode, error: &str) -> ActivationResponse {
    ActivationResponse {
        success: false,
        error: Some(error.to_string()),
        error_code: Some(code),
        data: None,
        quota_info: None,
    }
}
