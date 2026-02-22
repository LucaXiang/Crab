//! Stripe integration via REST API (no SDK dependency)

use hmac::{Hmac, Mac};
use sha2::Sha256;

/// Plan -> quota mapping
pub struct PlanQuota {
    pub max_edge_servers: i32,
    pub max_clients: i32,
}

pub fn plan_quota(plan: &str) -> PlanQuota {
    match plan {
        "basic" | "basic_yearly" => PlanQuota {
            max_edge_servers: 1,
            max_clients: 5,
        },
        "pro" | "pro_yearly" => PlanQuota {
            max_edge_servers: 3,
            max_clients: 10,
        },
        "enterprise" => PlanQuota {
            max_edge_servers: 10,
            max_clients: 50,
        },
        _ => PlanQuota {
            max_edge_servers: 1,
            max_clients: 5,
        },
    }
}

/// Create a Stripe Customer
pub async fn create_customer(
    secret_key: &str,
    email: &str,
    tenant_id: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .post("https://api.stripe.com/v1/customers")
        .basic_auth(secret_key, None::<&str>)
        .form(&[("email", email), ("metadata[tenant_id]", tenant_id)])
        .send()
        .await?
        .json()
        .await?;

    resp["id"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| format!("Stripe create_customer failed: {resp}").into())
}

/// Create a Stripe Checkout Session (subscription mode)
pub async fn create_checkout_session(
    secret_key: &str,
    customer_id: &str,
    price_id: &str,
    plan: &str,
    success_url: &str,
    cancel_url: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .post("https://api.stripe.com/v1/checkout/sessions")
        .basic_auth(secret_key, None::<&str>)
        .form(&[
            ("customer", customer_id),
            ("mode", "subscription"),
            ("line_items[0][price]", price_id),
            ("line_items[0][quantity]", "1"),
            ("success_url", success_url),
            ("cancel_url", cancel_url),
            ("allow_promotion_codes", "true"),
            ("metadata[plan]", plan),
        ])
        .send()
        .await?
        .json()
        .await?;

    resp["url"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| format!("Stripe create_checkout failed: {resp}").into())
}

/// Create a Stripe Billing Portal session
pub async fn create_billing_portal_session(
    secret_key: &str,
    customer_id: &str,
    return_url: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .post("https://api.stripe.com/v1/billing_portal/sessions")
        .basic_auth(secret_key, None::<&str>)
        .form(&[("customer", customer_id), ("return_url", return_url)])
        .send()
        .await?
        .json()
        .await?;

    resp["url"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| format!("Stripe billing portal failed: {resp}").into())
}

/// Verify Stripe webhook signature (HMAC-SHA256)
pub fn verify_webhook_signature(
    payload: &[u8],
    sig_header: &str,
    secret: &str,
) -> Result<(), &'static str> {
    let mut timestamp = "";
    let mut signature = "";
    for part in sig_header.split(',') {
        if let Some(t) = part.strip_prefix("t=") {
            timestamp = t;
        } else if let Some(v) = part.strip_prefix("v1=") {
            signature = v;
        }
    }

    if timestamp.is_empty() || signature.is_empty() {
        return Err("Invalid Stripe-Signature header");
    }

    let signed_payload = format!("{timestamp}.{}", std::str::from_utf8(payload).unwrap_or(""));
    let mut mac =
        Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_err(|_| "HMAC key error")?;
    mac.update(signed_payload.as_bytes());

    // Decode hex signature and use constant-time comparison via hmac::verify_slice
    let sig_bytes = hex::decode(signature).map_err(|_| "Invalid signature hex")?;
    mac.verify_slice(&sig_bytes)
        .map_err(|_| "Webhook signature mismatch")?;

    // Reject events older than 5 minutes to prevent replay attacks
    let ts: i64 = timestamp.parse().map_err(|_| "Invalid timestamp")?;
    let now = chrono::Utc::now().timestamp();
    if (now - ts).abs() > 300 {
        return Err("Webhook timestamp too old");
    }

    Ok(())
}
