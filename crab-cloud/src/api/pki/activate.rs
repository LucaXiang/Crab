use crate::auth::tenant_auth;
use crate::db::{activations, p12, subscriptions, sync_store, tenants};
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use crab_cert::{CertMetadata, CertProfile};
use shared::activation::{
    ActivationData, ActivationResponse, ActiveDevice, EntityType, PlanType, QuotaInfo,
    SignedBinding, SubscriptionInfo, SubscriptionStatus,
};
use shared::error::ErrorCode;

#[derive(serde::Deserialize)]
pub struct ActivateRequest {
    /// JWT session token (from /api/tenant/verify)
    pub token: String,
    pub device_id: String,
    pub replace_entity_id: Option<String>,
}

pub async fn activate(
    State(state): State<AppState>,
    Json(req): Json<ActivateRequest>,
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
    // 订阅检查由 edge-server 运行时的 is_subscription_blocked() 处理，
    // 前端展示 SubscriptionBlockedScreen。
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
    let max_edge_servers = sub.max_edge_servers;

    let existing = match activations::find_by_device(&state.pool, &tenant.id, &req.device_id).await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error = %e, "Database error checking existing device");
            return Json(fail(ErrorCode::InternalError, "Internal error"));
        }
    };

    let (entity_id, is_reactivation) = if let Some(ref existing) = existing {
        if existing.status == "active" || existing.status == "deactivated" {
            (existing.entity_id.clone(), true)
        } else {
            (format!("edge-server-{}", uuid::Uuid::new_v4()), false)
        }
    } else {
        (format!("edge-server-{}", uuid::Uuid::new_v4()), false)
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

    // Edge-server 证书需要同时具备 Server + Client EKU:
    // - Server: 作为 MessageBus TLS 服务端（POS 客户端连接）
    // - Client: 作为 mTLS 客户端（向 cloud 同步数据）
    let mut profile = CertProfile::new_server(
        &entity_id,
        vec![entity_id.clone(), "localhost".to_string()],
        Some(tenant.id.clone()),
        req.device_id.clone(),
    );
    profile.is_client = true;

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
        EntityType::Server,
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
        max_clients: sub.max_clients as u32,
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

    if let Err(e) = activations::acquire_activation_lock(&mut tx, &tenant.id).await {
        tracing::error!(error = %e, "Failed to acquire activation lock");
        return Json(fail(ErrorCode::InternalError, "Internal error"));
    }

    if !is_reactivation && max_edge_servers > 0 {
        let active_count = match activations::count_active_in_tx(&mut tx, &tenant.id).await {
            Ok(n) => n,
            Err(e) => {
                tracing::error!(error = %e, "Database error counting active devices");
                return Json(fail(ErrorCode::InternalError, "Internal error"));
            }
        };

        if active_count >= max_edge_servers as i64 {
            if let Some(ref replace_id) = req.replace_entity_id {
                let replace_target = activations::find_by_entity(&state.pool, replace_id).await;
                match replace_target {
                    Ok(Some(target))
                        if target.tenant_id == tenant.id && target.status == "active" =>
                    {
                        if let Err(e) =
                            activations::mark_replaced_in_tx(&mut tx, replace_id, &entity_id).await
                        {
                            tracing::error!(error = %e, "Failed to mark device as replaced");
                            return Json(fail(
                                ErrorCode::InternalError,
                                "Failed to replace device",
                            ));
                        }
                    }
                    _ => {
                        return Json(fail(ErrorCode::ValidationFailed, "Invalid replace target"));
                    }
                }
            } else {
                let active_devices = match activations::list_active(&state.pool, &tenant.id).await {
                    Ok(list) => list
                        .into_iter()
                        .map(|a| ActiveDevice {
                            entity_id: a.entity_id,
                            device_id: a.device_id,
                            activated_at: a.activated_at,
                            last_refreshed_at: a.last_refreshed_at,
                        })
                        .collect(),
                    Err(e) => {
                        tracing::error!(error = %e, "Database error listing active devices");
                        return Json(fail(ErrorCode::InternalError, "Internal error"));
                    }
                };

                return Json(ActivationResponse {
                    success: false,
                    error: Some("device_limit_reached".to_string()),
                    error_code: Some(ErrorCode::DeviceLimitReached),
                    data: None,
                    quota_info: Some(QuotaInfo {
                        max_slots: max_edge_servers as u32,
                        active_count: active_count as u32,
                        active_devices,
                    }),
                });
            }
        }
    }

    if let Err(e) = activations::insert_in_tx(
        &mut tx,
        &entity_id,
        &tenant.id,
        &req.device_id,
        &fingerprint,
    )
    .await
    {
        tracing::error!(error = %e, "Failed to write activation record");
        return Json(fail(ErrorCode::InternalError, "Internal error"));
    }

    if let Err(e) = tx.commit().await {
        tracing::error!(error = %e, "Failed to commit activation transaction");
        return Json(fail(ErrorCode::InternalError, "Internal error"));
    }

    // Query store_number for this edge-server
    let store_number = match sync_store::get_store_number(&state.pool, &entity_id, &tenant.id).await
    {
        Ok(n) => n,
        Err(e) => {
            tracing::error!(error = %e, "Failed to get store_number");
            return Json(fail(ErrorCode::InternalError, "Internal error"));
        }
    };

    tracing::info!(
        entity_id = %entity_id,
        tenant_id = %tenant.id,
        store_number = store_number,
        "Activated server"
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
            store_number,
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

pub fn parse_subscription_status(s: &str) -> SubscriptionStatus {
    match s {
        "active" => SubscriptionStatus::Active,
        "past_due" => SubscriptionStatus::PastDue,
        "canceled" => SubscriptionStatus::Canceled,
        "unpaid" => SubscriptionStatus::Unpaid,
        "expired" => SubscriptionStatus::Expired,
        _ => SubscriptionStatus::Inactive,
    }
}

pub fn parse_plan_type(s: &str) -> PlanType {
    match s {
        "pro" => PlanType::Pro,
        "enterprise" => PlanType::Enterprise,
        _ => PlanType::Basic,
    }
}
