//! Envelope encryption with AES-256-GCM
//!
//! Master key stored in AWS Secrets Manager (`crab/master-key`).
//! All sensitive data in PostgreSQL encrypted with this key.
//!
//! Format: base64(nonce_12bytes || ciphertext || tag_16bytes)

use aes_gcm::aead::Aead;
use aes_gcm::{Aes256Gcm, KeyInit, Nonce};
use base64::Engine;
use zeroize::Zeroize;

const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

/// Master encryption key (32 bytes for AES-256-GCM)
#[derive(Clone)]
pub struct MasterKey {
    key: [u8; KEY_LEN],
}

impl Drop for MasterKey {
    fn drop(&mut self) {
        self.key.zeroize();
    }
}

impl MasterKey {
    /// Load or create master key from AWS Secrets Manager
    pub async fn from_secrets_manager(
        sm: &aws_sdk_secretsmanager::Client,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let secret_name = "crab/master-key";

        match sm.get_secret_value().secret_id(secret_name).send().await {
            Ok(output) => {
                let b64 = output
                    .secret_string()
                    .ok_or("Master key secret has no string value")?;
                let bytes = base64::engine::general_purpose::STANDARD.decode(b64.trim())?;
                if bytes.len() != KEY_LEN {
                    return Err(format!(
                        "Master key wrong length: {} (expected {KEY_LEN})",
                        bytes.len()
                    )
                    .into());
                }
                let mut key = [0u8; KEY_LEN];
                key.copy_from_slice(&bytes);
                tracing::info!("Master key loaded from Secrets Manager");
                Ok(Self { key })
            }
            Err(err)
                if err
                    .as_service_error()
                    .is_some_and(|e| e.is_resource_not_found_exception()) =>
            {
                // Generate new master key
                let mut key = [0u8; KEY_LEN];
                rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut key);
                let b64 = base64::engine::general_purpose::STANDARD.encode(key);

                sm.create_secret()
                    .name(secret_name)
                    .secret_string(&b64)
                    .send()
                    .await?;

                tracing::info!("Master key created in Secrets Manager");
                Ok(Self { key })
            }
            Err(err) => Err(err.into()),
        }
    }

    /// Encrypt plaintext → base64(nonce || ciphertext || tag)
    pub fn encrypt(&self, plaintext: &[u8]) -> Result<String, &'static str> {
        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|_| "Invalid key")?;

        let mut nonce_bytes = [0u8; NONCE_LEN];
        rand::RngCore::fill_bytes(&mut rand::thread_rng(), &mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher
            .encrypt(nonce, plaintext)
            .map_err(|_| "Encryption failed")?;

        // nonce || ciphertext (includes tag)
        let mut result = Vec::with_capacity(NONCE_LEN + ciphertext.len());
        result.extend_from_slice(&nonce_bytes);
        result.extend_from_slice(&ciphertext);

        Ok(base64::engine::general_purpose::STANDARD.encode(&result))
    }

    /// Decrypt base64(nonce || ciphertext || tag) → plaintext
    pub fn decrypt(&self, encrypted_b64: &str) -> Result<Vec<u8>, &'static str> {
        let data = base64::engine::general_purpose::STANDARD
            .decode(encrypted_b64)
            .map_err(|_| "Invalid base64")?;

        if data.len() < NONCE_LEN + 16 {
            return Err("Ciphertext too short");
        }

        let cipher = Aes256Gcm::new_from_slice(&self.key).map_err(|_| "Invalid key")?;
        let nonce = Nonce::from_slice(&data[..NONCE_LEN]);
        let ciphertext = &data[NONCE_LEN..];

        cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| "Decryption failed (wrong key or tampered data)")
    }

    /// Encrypt a string → base64 blob
    pub fn encrypt_string(&self, plaintext: &str) -> Result<String, &'static str> {
        self.encrypt(plaintext.as_bytes())
    }

    /// Decrypt base64 blob → string
    pub fn decrypt_string(&self, encrypted_b64: &str) -> Result<String, &'static str> {
        let bytes = self.decrypt(encrypted_b64)?;
        String::from_utf8(bytes).map_err(|_| "Decrypted data is not valid UTF-8")
    }
}
