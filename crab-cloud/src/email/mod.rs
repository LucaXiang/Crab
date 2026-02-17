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
