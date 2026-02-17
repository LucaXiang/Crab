use aws_sdk_sesv2::Client as SesClient;
use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message};

pub async fn send_verification_code(
    ses: &SesClient,
    from: &str,
    to: &str,
    code: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subject = Content::builder()
        .data("Tu código de verificación / Your verification code")
        .build()?;

    let body_text = format!(
        "Tu código de verificación es: {code}\n\
         Válido durante 5 minutos.\n\n\
         Your verification code is: {code}\n\
         Valid for 5 minutes."
    );

    let body = Body::builder()
        .text(Content::builder().data(body_text).build()?)
        .build();

    let message = Message::builder().subject(subject).body(body).build();

    ses.send_email()
        .from_email_address(from)
        .destination(Destination::builder().to_addresses(to).build())
        .content(EmailContent::builder().simple(message).build())
        .send()
        .await?;

    tracing::info!(to = to, "Verification code sent");
    Ok(())
}

pub async fn send_password_reset_code(
    ses: &SesClient,
    from: &str,
    to: &str,
    code: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subject = Content::builder()
        .data("Restablecer contraseña / Reset your password")
        .build()?;

    let body_text = format!(
        "Tu código para restablecer la contraseña es: {code}\n\
         Válido durante 5 minutos.\n\n\
         Your password reset code is: {code}\n\
         Valid for 5 minutes."
    );

    let body = Body::builder()
        .text(Content::builder().data(body_text).build()?)
        .build();

    let message = Message::builder().subject(subject).body(body).build();

    ses.send_email()
        .from_email_address(from)
        .destination(Destination::builder().to_addresses(to).build())
        .content(EmailContent::builder().simple(message).build())
        .send()
        .await?;

    tracing::info!(to = to, "Password reset code sent");
    Ok(())
}

pub async fn send_email_change_code(
    ses: &SesClient,
    from: &str,
    to: &str,
    code: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subject = Content::builder()
        .data("Confirmar cambio de correo / Confirm email change")
        .build()?;

    let body_text = format!(
        "Tu código para confirmar el cambio de correo es: {code}\n\
         Válido durante 5 minutos.\n\n\
         Your email change confirmation code is: {code}\n\
         Valid for 5 minutes."
    );

    let body = Body::builder()
        .text(Content::builder().data(body_text).build()?)
        .build();

    let message = Message::builder().subject(subject).body(body).build();

    ses.send_email()
        .from_email_address(from)
        .destination(Destination::builder().to_addresses(to).build())
        .content(EmailContent::builder().simple(message).build())
        .send()
        .await?;

    tracing::info!(to = to, "Email change code sent");
    Ok(())
}

pub async fn send_subscription_activated(
    ses: &SesClient,
    from: &str,
    to: &str,
    plan: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subject = Content::builder()
        .data("Suscripción activada / Subscription activated")
        .build()?;

    let body_text = format!(
        "Tu suscripción al plan \"{plan}\" ha sido activada.\n\
         ¡Gracias por elegir Red Coral!\n\n\
         Your \"{plan}\" subscription has been activated.\n\
         Thank you for choosing Red Coral!"
    );

    let body = Body::builder()
        .text(Content::builder().data(body_text).build()?)
        .build();

    let message = Message::builder().subject(subject).body(body).build();

    ses.send_email()
        .from_email_address(from)
        .destination(Destination::builder().to_addresses(to).build())
        .content(EmailContent::builder().simple(message).build())
        .send()
        .await?;

    tracing::info!(to = to, plan = plan, "Subscription activated email sent");
    Ok(())
}

pub async fn send_subscription_canceled(
    ses: &SesClient,
    from: &str,
    to: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subject = Content::builder()
        .data("Suscripción cancelada / Subscription canceled")
        .build()?;

    let body_text = "Tu suscripción ha sido cancelada.\n\
         Si fue un error, puedes volver a suscribirte en cualquier momento.\n\n\
         Your subscription has been canceled.\n\
         If this was a mistake, you can resubscribe at any time."
        .to_string();

    let body = Body::builder()
        .text(Content::builder().data(body_text).build()?)
        .build();

    let message = Message::builder().subject(subject).body(body).build();

    ses.send_email()
        .from_email_address(from)
        .destination(Destination::builder().to_addresses(to).build())
        .content(EmailContent::builder().simple(message).build())
        .send()
        .await?;

    tracing::info!(to = to, "Subscription canceled email sent");
    Ok(())
}

pub async fn send_payment_failed(
    ses: &SesClient,
    from: &str,
    to: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subject = Content::builder()
        .data("Pago fallido / Payment failed")
        .build()?;

    let body_text = "No pudimos procesar tu pago.\n\
         Por favor actualiza tu método de pago para evitar la suspensión del servicio.\n\n\
         We were unable to process your payment.\n\
         Please update your payment method to avoid service suspension."
        .to_string();

    let body = Body::builder()
        .text(Content::builder().data(body_text).build()?)
        .build();

    let message = Message::builder().subject(subject).body(body).build();

    ses.send_email()
        .from_email_address(from)
        .destination(Destination::builder().to_addresses(to).build())
        .content(EmailContent::builder().simple(message).build())
        .send()
        .await?;

    tracing::info!(to = to, "Payment failed email sent");
    Ok(())
}

#[allow(dead_code)]
pub async fn send_refund_processed(
    ses: &SesClient,
    from: &str,
    to: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let subject = Content::builder()
        .data("Reembolso procesado / Refund processed")
        .build()?;

    let body_text = "Tu reembolso ha sido procesado.\n\
         El monto será devuelto a tu método de pago original.\n\n\
         Your refund has been processed.\n\
         The amount will be returned to your original payment method."
        .to_string();

    let body = Body::builder()
        .text(Content::builder().data(body_text).build()?)
        .build();

    let message = Message::builder().subject(subject).body(body).build();

    ses.send_email()
        .from_email_address(from)
        .destination(Destination::builder().to_addresses(to).build())
        .content(EmailContent::builder().simple(message).build())
        .send()
        .await?;

    tracing::info!(to = to, "Refund processed email sent");
    Ok(())
}
