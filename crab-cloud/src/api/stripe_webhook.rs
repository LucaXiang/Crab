//! Stripe webhook handler
//!
//! POST /stripe/webhook — handles Stripe events (raw body for signature verification)

use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};

use shared::cloud::TenantStatus;

use crate::state::AppState;
use crate::{db, email, stripe};

use chrono;
use sqlx;

/// Handle incoming Stripe webhook events
///
/// Must receive raw body (not JSON) for HMAC signature verification.
pub async fn handle_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    // 1. Get Stripe-Signature header
    let sig_header = match headers
        .get("stripe-signature")
        .and_then(|v| v.to_str().ok())
    {
        Some(s) => s,
        None => {
            tracing::warn!("Missing Stripe-Signature header");
            return StatusCode::BAD_REQUEST;
        }
    };

    // 2. Verify signature
    if let Err(e) =
        stripe::verify_webhook_signature(&body, sig_header, &state.stripe_webhook_secret)
    {
        tracing::warn!(error = e, "Webhook signature verification failed");
        return StatusCode::BAD_REQUEST;
    }

    // 3. Parse JSON event
    let event: serde_json::Value = match serde_json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(%e, "Failed to parse webhook JSON");
            return StatusCode::BAD_REQUEST;
        }
    };

    let event_type = event["type"].as_str().unwrap_or("");
    tracing::info!(event_type = event_type, "Received Stripe webhook");

    // 4. Idempotency: INSERT first, check rows_affected (eliminates TOCTOU race)
    let event_id = match event["id"].as_str() {
        Some(id) => id,
        None => {
            tracing::warn!("Webhook event missing id");
            return StatusCode::BAD_REQUEST;
        }
    };

    let now = chrono::Utc::now().timestamp_millis();
    let insert_result = sqlx::query(
        "INSERT INTO processed_webhook_events (event_id, event_type, processed_at)
         VALUES ($1, $2, $3) ON CONFLICT DO NOTHING",
    )
    .bind(event_id)
    .bind(event_type)
    .bind(now)
    .execute(&state.pool)
    .await;

    match insert_result {
        Ok(r) if r.rows_affected() == 0 => {
            tracing::info!(event_id = event_id, "Duplicate webhook event, skipping");
            return StatusCode::OK;
        }
        Err(e) => {
            tracing::error!(%e, "DB error recording webhook event");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
        Ok(_) => {} // New event, proceed
    }

    // 5. Handle event types
    match event_type {
        "checkout.session.completed" => handle_checkout_completed(&state, &event).await,
        "customer.subscription.updated" => handle_subscription_updated(&state, &event).await,
        "customer.subscription.deleted" => handle_subscription_deleted(&state, &event).await,
        "invoice.payment_failed" => handle_payment_failed(&state, &event).await,
        "charge.refunded" => handle_charge_refunded(&state, &event).await,
        "invoice.paid" => handle_invoice_paid(&state, &event).await,
        "invoice.payment_action_required" => handle_payment_action_required(&state, &event).await,
        _ => {
            tracing::debug!(event_type = event_type, "Unhandled webhook event type");
            StatusCode::OK
        }
    }
}

/// checkout.session.completed → create subscription + activate tenant
async fn handle_checkout_completed(state: &AppState, event: &serde_json::Value) -> StatusCode {
    let obj = match event.get("data").and_then(|d| d.get("object")) {
        Some(o) => o,
        None => return StatusCode::OK,
    };

    let customer_id = match obj["customer"].as_str() {
        Some(s) => s,
        None => {
            tracing::warn!("checkout.session.completed missing customer");
            return StatusCode::OK;
        }
    };

    let subscription_id = match obj["subscription"].as_str() {
        Some(s) => s,
        None => {
            tracing::warn!("checkout.session.completed missing subscription");
            return StatusCode::OK;
        }
    };

    // Find tenant by stripe_customer_id
    let tenant = match db::tenants::find_by_stripe_customer(&state.pool, customer_id).await {
        Ok(Some(t)) => t,
        Ok(None) => {
            tracing::warn!(customer_id = customer_id, "No tenant for Stripe customer");
            return StatusCode::OK;
        }
        Err(e) => {
            tracing::error!(%e, "DB error finding tenant by Stripe customer");
            return StatusCode::INTERNAL_SERVER_ERROR;
        }
    };

    // Determine plan from metadata or default to "basic"
    let plan = obj
        .get("metadata")
        .and_then(|m| m["plan"].as_str())
        .unwrap_or("basic");

    let quota = stripe::plan_quota(plan);
    let now = chrono::Utc::now().timestamp_millis();

    // Create subscription
    let sub = db::subscriptions::CreateSubscription {
        id: subscription_id,
        tenant_id: &tenant.id,
        plan,
        max_edge_servers: quota.max_edge_servers,
        max_clients: quota.max_clients,
        current_period_end: None, // set by subscription.updated events
        now,
    };
    if let Err(e) = db::subscriptions::create(&state.pool, &sub).await {
        tracing::error!(%e, "Failed to create subscription");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    // Activate tenant
    if let Err(e) =
        db::tenants::update_status(&state.pool, &tenant.id, TenantStatus::Active.as_db()).await
    {
        tracing::error!(%e, "Failed to activate tenant");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    tracing::info!(
        tenant_id = %tenant.id,
        subscription_id = subscription_id,
        plan = plan,
        "Tenant activated via Stripe checkout"
    );

    let _ =
        email::send_subscription_activated(&state.ses, &state.ses_from_email, &tenant.email, plan)
            .await;

    let detail = serde_json::json!({ "subscription_id": subscription_id, "plan": plan });
    let _ = crate::db::audit::log(
        &state.pool,
        &tenant.id,
        "subscription_activated",
        Some(&detail),
        None,
        now,
    )
    .await;

    StatusCode::OK
}

/// customer.subscription.updated → update subscription status/plan
async fn handle_subscription_updated(state: &AppState, event: &serde_json::Value) -> StatusCode {
    let obj = match event.get("data").and_then(|d| d.get("object")) {
        Some(o) => o,
        None => return StatusCode::OK,
    };

    let sub_id = match obj["id"].as_str() {
        Some(s) => s,
        None => return StatusCode::OK,
    };

    let status = obj["status"].as_str().unwrap_or("active");

    if let Err(e) = db::subscriptions::update_status(&state.pool, sub_id, status).await {
        tracing::error!(%e, "Failed to update subscription status");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    tracing::info!(
        subscription_id = sub_id,
        status = status,
        "Subscription updated"
    );
    StatusCode::OK
}

/// customer.subscription.deleted → cancel subscription + tenant
async fn handle_subscription_deleted(state: &AppState, event: &serde_json::Value) -> StatusCode {
    let obj = match event.get("data").and_then(|d| d.get("object")) {
        Some(o) => o,
        None => return StatusCode::OK,
    };

    let sub_id = match obj["id"].as_str() {
        Some(s) => s,
        None => return StatusCode::OK,
    };

    // Update subscription to canceled
    if let Err(e) = db::subscriptions::update_status(&state.pool, sub_id, "canceled").await {
        tracing::error!(%e, "Failed to cancel subscription");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    // Find and cancel tenant
    if let Ok(Some(tenant_id)) = db::subscriptions::find_tenant_by_sub_id(&state.pool, sub_id).await
    {
        let _ = db::tenants::update_status(&state.pool, &tenant_id, TenantStatus::Canceled.as_db())
            .await;
        tracing::info!(tenant_id = %tenant_id, "Tenant canceled (subscription deleted)");

        if let Ok(Some(tenant)) = db::tenants::find_by_id(&state.pool, &tenant_id).await {
            let _ =
                email::send_subscription_canceled(&state.ses, &state.ses_from_email, &tenant.email)
                    .await;
        }

        let detail = serde_json::json!({ "subscription_id": sub_id });
        let _ = crate::db::audit::log(
            &state.pool,
            &tenant_id,
            "subscription_canceled",
            Some(&detail),
            None,
            chrono::Utc::now().timestamp_millis(),
        )
        .await;
    }

    StatusCode::OK
}

/// invoice.payment_failed → suspend tenant
async fn handle_payment_failed(state: &AppState, event: &serde_json::Value) -> StatusCode {
    let obj = match event.get("data").and_then(|d| d.get("object")) {
        Some(o) => o,
        None => return StatusCode::OK,
    };

    let sub_id = match obj["subscription"].as_str() {
        Some(s) => s,
        None => return StatusCode::OK,
    };

    // Mark subscription past_due
    if let Err(e) = db::subscriptions::update_status(&state.pool, sub_id, "past_due").await {
        tracing::error!(%e, "Failed to update subscription to past_due");
        return StatusCode::INTERNAL_SERVER_ERROR;
    }

    // Suspend tenant
    if let Ok(Some(tenant_id)) = db::subscriptions::find_tenant_by_sub_id(&state.pool, sub_id).await
    {
        let _ =
            db::tenants::update_status(&state.pool, &tenant_id, TenantStatus::Suspended.as_db())
                .await;
        tracing::info!(tenant_id = %tenant_id, "Tenant suspended (payment failed)");

        if let Ok(Some(tenant)) = db::tenants::find_by_id(&state.pool, &tenant_id).await {
            let _ =
                email::send_payment_failed(&state.ses, &state.ses_from_email, &tenant.email).await;
        }
    }

    StatusCode::OK
}

/// charge.refunded → notify tenant
async fn handle_charge_refunded(state: &AppState, event: &serde_json::Value) -> StatusCode {
    let obj = match event.get("data").and_then(|d| d.get("object")) {
        Some(o) => o,
        None => return StatusCode::OK,
    };

    let customer_id = match obj["customer"].as_str() {
        Some(s) => s,
        None => return StatusCode::OK,
    };

    if let Ok(Some(tenant)) = db::tenants::find_by_stripe_customer(&state.pool, customer_id).await {
        let _ =
            email::send_refund_processed(&state.ses, &state.ses_from_email, &tenant.email).await;
        tracing::info!(tenant_id = %tenant.id, "Refund notification sent");
    }

    StatusCode::OK
}

/// invoice.paid → update current_period_end
async fn handle_invoice_paid(state: &AppState, event: &serde_json::Value) -> StatusCode {
    let obj = match event.get("data").and_then(|d| d.get("object")) {
        Some(o) => o,
        None => return StatusCode::OK,
    };

    let sub_id = match obj["subscription"].as_str() {
        Some(s) => s,
        None => return StatusCode::OK,
    };

    // Update current_period_end from invoice lines
    if let Some(period_end) = obj
        .get("lines")
        .and_then(|l| l.get("data"))
        .and_then(|d| d.as_array())
        .and_then(|a| a.first())
        .and_then(|line| line.get("period"))
        .and_then(|p| p["end"].as_i64())
    {
        let period_end_ms = period_end * 1000; // Stripe uses seconds
        let _ = sqlx::query("UPDATE subscriptions SET current_period_end = $1 WHERE id = $2")
            .bind(period_end_ms)
            .bind(sub_id)
            .execute(&state.pool)
            .await;
    }

    tracing::info!(subscription_id = sub_id, "Invoice paid, period updated");
    StatusCode::OK
}

/// invoice.payment_action_required → notify tenant about SCA/3DS
async fn handle_payment_action_required(state: &AppState, event: &serde_json::Value) -> StatusCode {
    let obj = match event.get("data").and_then(|d| d.get("object")) {
        Some(o) => o,
        None => return StatusCode::OK,
    };

    let customer_id = match obj["customer"].as_str() {
        Some(s) => s,
        None => return StatusCode::OK,
    };

    if let Ok(Some(tenant)) = db::tenants::find_by_stripe_customer(&state.pool, customer_id).await {
        let _ = email::send_payment_failed(&state.ses, &state.ses_from_email, &tenant.email).await;
        tracing::info!(tenant_id = %tenant.id, "Payment action required notification sent");
    }

    StatusCode::OK
}
