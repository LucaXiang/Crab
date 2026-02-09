use crate::db::{activations, subscriptions, tenants};
use crate::state::AppState;
use axum::Json;
use axum::extract::State;
use crab_cert::{CaProfile, CertMetadata, CertProfile, CertificateAuthority};
use shared::activation::{
    ActiveDevice, ActivationData, ActivationResponse, EntityType, PlanType, QuotaInfo,
    SignedBinding, SubscriptionInfo, SubscriptionStatus,
};
use std::sync::Arc;

#[derive(serde::Deserialize)]
pub struct ActivateRequest {
    pub username: String,
    pub password: String,
    pub device_id: String,
    pub replace_entity_id: Option<String>,
}

pub async fn activate(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ActivateRequest>,
) -> Json<ActivationResponse> {
    // 1. Authenticate
    let tenant = match tenants::authenticate(&state.db, &req.username, &req.password).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            return Json(fail("Invalid credentials"));
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error during authentication");
            return Json(fail("Internal error"));
        }
    };

    // 2. Check subscription
    let sub = match subscriptions::get_active_subscription(&state.db, &tenant.id).await {
        Ok(Some(s)) => s,
        Ok(None) => {
            return Json(fail("No active subscription"));
        }
        Err(e) => {
            tracing::error!(error = %e, "Database error fetching subscription");
            return Json(fail("Internal error"));
        }
    };

    // Check subscription status
    let sub_status = parse_subscription_status(&sub.status);
    if sub_status.is_blocked() {
        return Json(fail(&format!("Subscription {}", sub.status)));
    }

    let plan = parse_plan_type(&sub.plan);
    let max_edge_servers = sub.max_edge_servers;

    // 3. Check device quota
    let existing = match activations::find_by_device(&state.db, &tenant.id, &req.device_id).await {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error = %e, "Database error checking existing device");
            return Json(fail("Internal error"));
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

    // If not a re-activation, check quota
    if !is_reactivation {
        let active_count = match activations::count_active(&state.db, &tenant.id).await {
            Ok(n) => n,
            Err(e) => {
                tracing::error!(error = %e, "Database error counting active devices");
                return Json(fail("Internal error"));
            }
        };

        // max_edge_servers = 0 means unlimited
        if max_edge_servers > 0 && active_count >= max_edge_servers as i64 {
            // Quota full — do we have a replace request?
            if let Some(ref replace_id) = req.replace_entity_id {
                let replace_target =
                    activations::find_by_entity(&state.db, replace_id).await;
                match replace_target {
                    Ok(Some(target))
                        if target.tenant_id == tenant.id && target.status == "active" =>
                    {
                        if let Err(e) =
                            activations::mark_replaced(&state.db, replace_id, &entity_id).await
                        {
                            tracing::error!(error = %e, "Failed to mark device as replaced");
                            return Json(fail("Failed to replace device"));
                        }
                    }
                    _ => {
                        return Json(fail("Invalid replace target"));
                    }
                }
            } else {
                // Return quota info so frontend can show device list
                let active_devices = match activations::list_active(&state.db, &tenant.id).await {
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
                        return Json(fail("Internal error"));
                    }
                };

                return Json(ActivationResponse {
                    success: false,
                    error: Some("device_limit_reached".to_string()),
                    data: None,
                    quota_info: Some(QuotaInfo {
                        max_edge_servers: max_edge_servers as u32,
                        active_count: active_count as u32,
                        active_devices,
                    }),
                });
            }
        }
    }

    // 4. Issue certificate
    let root_ca = match state.auth_storage.get_or_create_root_ca() {
        Ok(ca) => ca,
        Err(e) => {
            tracing::error!(error = %e, "Root CA error");
            return Json(fail("Root CA error"));
        }
    };

    let tenant_dir = match state.auth_storage.get_tenant_dir(&tenant.id) {
        Ok(d) => d,
        Err(e) => {
            tracing::error!(error = %e, tenant_id = %tenant.id, "Failed to access tenant directory");
            return Json(fail("Storage error"));
        }
    };

    let tenant_ca_name = "tenant_ca";
    let tenant_ca =
        if tenant_dir.join(format!("{tenant_ca_name}.crt")).exists() {
            match CertificateAuthority::load_from_file(
                &tenant_dir.join(format!("{tenant_ca_name}.crt")),
                &tenant_dir.join(format!("{tenant_ca_name}.key")),
            ) {
                Ok(ca) => ca,
                Err(e) => {
                    return Json(fail(&format!("Failed to load Tenant CA: {e}")));
                }
            }
        } else {
            let profile =
                CaProfile::intermediate(&tenant.id, &format!("Tenant {}", tenant.id));
            let ca = match CertificateAuthority::new_intermediate(profile, &root_ca) {
                Ok(ca) => ca,
                Err(e) => {
                    return Json(fail(&format!("Failed to create Tenant CA: {e}")));
                }
            };
            if let Err(e) = ca.save(&tenant_dir, tenant_ca_name) {
                return Json(fail(&format!("Failed to save Tenant CA: {e}")));
            }
            ca
        };

    // Issue server cert
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
            return Json(fail(&format!("Failed to issue certificate: {e}")));
        }
    };

    // Certificate fingerprint
    let fingerprint = match CertMetadata::from_pem(&entity_cert) {
        Ok(meta) => meta.fingerprint_sha256,
        Err(e) => {
            return Json(fail(&format!("Certificate metadata error: {e}")));
        }
    };

    // 5. Create and sign binding
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
            return Json(fail(&format!("Failed to sign binding: {e}")));
        }
    };

    // 6. Build subscription info
    let signature_valid_until = shared::util::now_millis() + 7 * 24 * 60 * 60 * 1000;
    let subscription_info = SubscriptionInfo {
        tenant_id: tenant.id.clone(),
        id: Some(sub.id.clone()),
        status: sub_status,
        plan,
        starts_at: shared::util::now_millis(),
        expires_at: sub.current_period_end,
        features: sub.features.clone(),
        max_stores: max_edge_servers as u32,
        signature_valid_until,
        signature: String::new(),
        last_checked_at: 0,
    };

    let signed_subscription = match subscription_info.sign(&tenant_ca.key_pem()) {
        Ok(s) => s,
        Err(e) => {
            return Json(fail(&format!("Failed to sign subscription: {e}")));
        }
    };

    // 7. Write activation record
    if let Err(e) =
        activations::insert(&state.db, &entity_id, &tenant.id, &req.device_id, &fingerprint)
            .await
    {
        tracing::error!(error = %e, "Failed to write activation record");
        return Json(fail("Failed to save activation"));
    }

    tracing::info!(
        entity_id = %entity_id,
        tenant_id = %tenant.id,
        "Activated server"
    );

    // 8. Return response
    Json(ActivationResponse {
        success: true,
        error: None,
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

fn fail(error: &str) -> ActivationResponse {
    ActivationResponse {
        success: false,
        error: Some(error.to_string()),
        data: None,
        quota_info: None,
    }
}

fn parse_subscription_status(s: &str) -> SubscriptionStatus {
    match s {
        "active" => SubscriptionStatus::Active,
        "past_due" => SubscriptionStatus::PastDue,
        "canceled" => SubscriptionStatus::Canceled,
        "unpaid" => SubscriptionStatus::Unpaid,
        "expired" => SubscriptionStatus::Expired,
        _ => SubscriptionStatus::Inactive,
    }
}

fn parse_plan_type(s: &str) -> PlanType {
    match s {
        "pro" => PlanType::Pro,
        "enterprise" => PlanType::Enterprise,
        _ => PlanType::Basic,
    }
}
