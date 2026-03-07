//! Stripe integration via REST API (no SDK dependency)

use hmac::{Hmac, Mac};
use sha2::Sha256;

use shared::activation::PlanType;

/// Parse a legacy plan string (e.g. "basic_yearly") into PlanType,
/// stripping the billing interval suffix.
pub fn parse_plan_str(s: &str) -> PlanType {
    let base = s
        .strip_suffix("_yearly")
        .or_else(|| s.strip_suffix("_monthly"))
        .unwrap_or(s);
    PlanType::parse(base).unwrap_or(PlanType::Basic)
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

/// Cancel a Stripe subscription at period end
pub async fn cancel_subscription(
    secret_key: &str,
    subscription_id: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .post(format!(
            "https://api.stripe.com/v1/subscriptions/{subscription_id}"
        ))
        .basic_auth(secret_key, None::<&str>)
        .form(&[("cancel_at_period_end", "true")])
        .send()
        .await?
        .json()
        .await?;

    if resp.get("error").is_some() {
        return Err(format!("Stripe cancel_subscription failed: {resp}").into());
    }
    Ok(resp)
}

/// Resume a Stripe subscription (undo cancel_at_period_end)
pub async fn resume_subscription(
    secret_key: &str,
    subscription_id: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .post(format!(
            "https://api.stripe.com/v1/subscriptions/{subscription_id}"
        ))
        .basic_auth(secret_key, None::<&str>)
        .form(&[("cancel_at_period_end", "false")])
        .send()
        .await?
        .json()
        .await?;

    if resp.get("error").is_some() {
        return Err(format!("Stripe resume_subscription failed: {resp}").into());
    }
    Ok(resp)
}

/// Update a Stripe subscription's price (for plan changes)
pub async fn update_subscription_price(
    secret_key: &str,
    subscription_id: &str,
    subscription_item_id: &str,
    new_price_id: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .post(format!(
            "https://api.stripe.com/v1/subscriptions/{subscription_id}"
        ))
        .basic_auth(secret_key, None::<&str>)
        .form(&[
            ("items[0][id]", subscription_item_id),
            ("items[0][price]", new_price_id),
            ("cancel_at_period_end", "false"),
            ("proration_behavior", "always_invoice"),
        ])
        .send()
        .await?
        .json()
        .await?;

    if resp.get("error").is_some() {
        return Err(format!("Stripe update_subscription_price failed: {resp}").into());
    }
    Ok(resp)
}

/// Get a Stripe subscription
pub async fn get_subscription(
    secret_key: &str,
    subscription_id: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let resp: serde_json::Value = client
        .get(format!(
            "https://api.stripe.com/v1/subscriptions/{subscription_id}"
        ))
        .basic_auth(secret_key, None::<&str>)
        .send()
        .await?
        .json()
        .await?;

    if resp.get("error").is_some() {
        return Err(format!("Stripe get_subscription failed: {resp}").into());
    }
    Ok(resp)
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

    let payload_str = std::str::from_utf8(payload).map_err(|_| "Non-UTF8 webhook payload")?;
    let signed_payload = format!("{timestamp}.{payload_str}");
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
    if now - ts > 300 {
        return Err("Webhook timestamp too old");
    }
    if ts > now + 60 {
        return Err("Webhook timestamp in the future");
    }

    Ok(())
}
