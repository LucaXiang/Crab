//! Email service — Resend API for transactional emails

use reqwest::Client;

/// Resend email client wrapper
#[derive(Clone)]
pub struct EmailService {
    client: Client,
    api_key: String,
    from: String,
}

impl EmailService {
    pub fn new(api_key: String, from: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            from,
        }
    }

    async fn send(
        &self,
        to: &str,
        subject: &str,
        text: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let resp = self
            .client
            .post("https://api.resend.com/emails")
            .bearer_auth(&self.api_key)
            .json(&serde_json::json!({
                "from": self.from,
                "to": [to],
                "subject": subject,
                "text": text,
            }))
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("Resend API error {status}: {body}").into());
        }

        Ok(())
    }

    pub async fn send_verification_code(
        &self,
        to: &str,
        code: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let subject = "Tu código de verificación / Your verification code";
        let text = format!(
            "Tu código de verificación es: {code}\n\
             Válido durante 5 minutos.\n\n\
             Your verification code is: {code}\n\
             Valid for 5 minutes."
        );
        self.send(to, subject, &text).await?;
        tracing::info!(to = to, "Verification code sent");
        Ok(())
    }

    pub async fn send_password_reset_code(
        &self,
        to: &str,
        code: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let subject = "Restablecer contraseña / Reset your password";
        let text = format!(
            "Tu código para restablecer la contraseña es: {code}\n\
             Válido durante 5 minutos.\n\n\
             Your password reset code is: {code}\n\
             Valid for 5 minutes."
        );
        self.send(to, subject, &text).await?;
        tracing::info!(to = to, "Password reset code sent");
        Ok(())
    }

    pub async fn send_email_change_code(
        &self,
        to: &str,
        code: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let subject = "Confirmar cambio de correo / Confirm email change";
        let text = format!(
            "Tu código para confirmar el cambio de correo es: {code}\n\
             Válido durante 5 minutos.\n\n\
             Your email change confirmation code is: {code}\n\
             Valid for 5 minutes."
        );
        self.send(to, subject, &text).await?;
        tracing::info!(to = to, "Email change code sent");
        Ok(())
    }

    pub async fn send_subscription_activated(
        &self,
        to: &str,
        plan: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let subject = "Suscripción activada / Subscription activated";
        let text = format!(
            "Tu suscripción al plan \"{plan}\" ha sido activada.\n\
             ¡Gracias por elegir Red Coral!\n\n\
             Your \"{plan}\" subscription has been activated.\n\
             Thank you for choosing Red Coral!"
        );
        self.send(to, subject, &text).await?;
        tracing::info!(to = to, plan = plan, "Subscription activated email sent");
        Ok(())
    }

    pub async fn send_subscription_canceled(
        &self,
        to: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let subject = "Suscripción cancelada / Subscription canceled";
        let text = "Tu suscripción ha sido cancelada.\n\
             Si fue un error, puedes volver a suscribirte en cualquier momento.\n\n\
             Your subscription has been canceled.\n\
             If this was a mistake, you can resubscribe at any time.";
        self.send(to, subject, text).await?;
        tracing::info!(to = to, "Subscription canceled email sent");
        Ok(())
    }

    pub async fn send_payment_failed(
        &self,
        to: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let subject = "Pago fallido / Payment failed";
        let text = "No pudimos procesar tu pago.\n\
             Por favor actualiza tu método de pago para evitar la suspensión del servicio.\n\n\
             We were unable to process your payment.\n\
             Please update your payment method to avoid service suspension.";
        self.send(to, subject, text).await?;
        tracing::info!(to = to, "Payment failed email sent");
        Ok(())
    }

    pub async fn send_refund_processed(
        &self,
        to: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let subject = "Reembolso procesado / Refund processed";
        let text = "Tu reembolso ha sido procesado.\n\
             El monto será devuelto a tu método de pago original.\n\n\
             Your refund has been processed.\n\
             The amount will be returned to your original payment method.";
        self.send(to, subject, text).await?;
        tracing::info!(to = to, "Refund processed email sent");
        Ok(())
    }
}
