//! CloudService — HTTP client + WebSocket connector for crab-cloud

use reqwest::Client;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use shared::activation::SignedBinding;
use shared::cloud::{CloudSyncBatch, CloudSyncResponse};
use std::error::Error as StdError;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio_tungstenite::tungstenite;
use tokio_tungstenite::{Connector, MaybeTlsStream};

use crate::utils::AppError;

pub type WsStream = tokio_tungstenite::WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;

/// HTTP + WebSocket client for crab-cloud communication
pub struct CloudService {
    /// HTTP client for fallback sync
    client: Client,
    cloud_url: String,
    edge_id: String,
    /// Certs directory path for building rustls config on WS connect
    certs_dir: PathBuf,
}

impl CloudService {
    /// Create a new CloudService with mTLS configuration
    pub fn new(cloud_url: String, edge_id: String, certs_dir: &Path) -> Result<Self, AppError> {
        let edge_cert_pem = std::fs::read(certs_dir.join("server.pem"))
            .map_err(|e| AppError::internal(format!("Failed to read edge cert: {e}")))?;
        let edge_key_pem = std::fs::read(certs_dir.join("server.key.pem"))
            .map_err(|e| AppError::internal(format!("Failed to read edge key: {e}")))?;
        let tenant_ca_pem = std::fs::read(certs_dir.join("tenant_ca.pem"))
            .map_err(|e| AppError::internal(format!("Failed to read tenant CA: {e}")))?;
        let root_ca_pem = std::fs::read(certs_dir.join("root_ca.pem"))
            .map_err(|e| AppError::internal(format!("Failed to read root CA: {e}")))?;

        tracing::info!(
            cloud_url = %cloud_url,
            "CloudService: loading mTLS certificates"
        );

        // Build identity: edge_cert + tenant_ca (full chain) + key
        let identity_pem = [
            edge_cert_pem,
            b"\n".to_vec(),
            tenant_ca_pem,
            b"\n".to_vec(),
            edge_key_pem,
        ]
        .concat();
        let identity = reqwest::Identity::from_pem(&identity_pem)
            .map_err(|e| AppError::internal(format!("Failed to create identity: {e}")))?;

        // Root CA for verifying crab-cloud's server certificate
        let root_cert = reqwest::Certificate::from_pem(&root_ca_pem)
            .map_err(|e| AppError::internal(format!("Failed to parse root CA: {e}")))?;

        let client = Client::builder()
            .identity(identity)
            .add_root_certificate(root_cert)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| AppError::internal(format!("Failed to build HTTP client: {e}")))?;

        Ok(Self {
            client,
            cloud_url,
            edge_id,
            certs_dir: certs_dir.to_path_buf(),
        })
    }

    /// Connect WebSocket with mTLS to crab-cloud
    pub async fn connect_ws(&self, binding: &SignedBinding) -> Result<WsStream, AppError> {
        let tls_config = self.build_rustls_config()?;
        let connector = Connector::Rustls(Arc::new(tls_config));

        let binding_json = serde_json::to_string(binding)
            .map_err(|e| AppError::internal(format!("Failed to serialize binding: {e}")))?;

        // Convert https:// URL to wss://
        let ws_url = self
            .cloud_url
            .replace("https://", "wss://")
            .replace("http://", "ws://");
        let url = format!("{ws_url}/api/edge/ws");

        // Extract host from URL for the Host header (required by WebSocket protocol)
        let host = url
            .split("://")
            .nth(1)
            .and_then(|s| s.split('/').next())
            .unwrap_or("localhost");

        let request = tungstenite::http::Request::builder()
            .uri(&url)
            .header("Host", host)
            .header("X-Signed-Binding", &binding_json)
            .header("Connection", "Upgrade")
            .header("Upgrade", "websocket")
            .header("Sec-WebSocket-Version", "13")
            .header(
                "Sec-WebSocket-Key",
                tungstenite::handshake::client::generate_key(),
            )
            .body(())
            .map_err(|e| AppError::internal(format!("Failed to build WS request: {e}")))?;

        let (ws_stream, _response) = tokio_tungstenite::connect_async_tls_with_config(
            request,
            None,
            false,
            Some(connector),
        )
        .await
        .map_err(|e| {
            // Distinguish auth failures (401/403) from transient errors so the
            // caller can apply a longer backoff for permanent auth issues.
            if let tungstenite::Error::Http(ref resp) = e {
                let status = resp.status().as_u16();
                if status == 401 || status == 403 {
                    return AppError::with_message(
                        shared::error::ErrorCode::NotAuthenticated,
                        format!("Cloud rejected connection (HTTP {status}): authentication failed"),
                    );
                }
            }
            AppError::internal(format!("WebSocket connection failed: {e}"))
        })?;

        tracing::info!(url = %url, "WebSocket connected to crab-cloud");
        Ok(ws_stream)
    }

    /// Build rustls ClientConfig with mTLS certs
    fn build_rustls_config(&self) -> Result<rustls::ClientConfig, AppError> {
        let edge_cert_pem = std::fs::read(self.certs_dir.join("server.pem"))
            .map_err(|e| AppError::internal(format!("Read edge cert: {e}")))?;
        let edge_key_pem = std::fs::read(self.certs_dir.join("server.key.pem"))
            .map_err(|e| AppError::internal(format!("Read edge key: {e}")))?;
        let tenant_ca_pem = std::fs::read(self.certs_dir.join("tenant_ca.pem"))
            .map_err(|e| AppError::internal(format!("Read tenant CA: {e}")))?;
        let root_ca_pem = std::fs::read(self.certs_dir.join("root_ca.pem"))
            .map_err(|e| AppError::internal(format!("Read root CA: {e}")))?;

        // Parse certs
        let mut edge_certs: Vec<CertificateDer<'static>> = Vec::new();
        let mut cursor = std::io::Cursor::new(&edge_cert_pem);
        for cert in rustls_pemfile::certs(&mut cursor) {
            edge_certs.push(cert.map_err(|e| AppError::internal(format!("Parse edge cert: {e}")))?);
        }

        let mut tenant_ca_certs: Vec<CertificateDer<'static>> = Vec::new();
        let mut cursor = std::io::Cursor::new(&tenant_ca_pem);
        for cert in rustls_pemfile::certs(&mut cursor) {
            tenant_ca_certs
                .push(cert.map_err(|e| AppError::internal(format!("Parse tenant CA: {e}")))?);
        }

        // Full client cert chain: edge_cert + tenant_ca
        let mut cert_chain = edge_certs;
        cert_chain.extend(tenant_ca_certs);

        // Parse private key
        let mut cursor = std::io::Cursor::new(&edge_key_pem);
        let key: PrivateKeyDer<'static> = rustls_pemfile::private_key(&mut cursor)
            .map_err(|e| AppError::internal(format!("Parse edge key: {e}")))?
            .ok_or_else(|| AppError::internal("No private key found in server.key.pem"))?;

        // Root store with root CA
        let mut root_store = rustls::RootCertStore::empty();
        let mut cursor = std::io::Cursor::new(&root_ca_pem);
        for cert in rustls_pemfile::certs(&mut cursor) {
            let cert = cert.map_err(|e| AppError::internal(format!("Parse root CA cert: {e}")))?;
            root_store
                .add(cert)
                .map_err(|e| AppError::internal(format!("Add root CA: {e}")))?;
        }

        let config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_client_auth_cert(cert_chain, key)
            .map_err(|e| AppError::internal(format!("Build TLS config: {e}")))?;

        Ok(config)
    }

    /// Push a sync batch to crab-cloud via HTTP (fallback)
    pub async fn push_batch(
        &self,
        batch: CloudSyncBatch,
        binding: &SignedBinding,
    ) -> Result<CloudSyncResponse, AppError> {
        let binding_json = serde_json::to_string(binding)
            .map_err(|e| AppError::internal(format!("Failed to serialize binding: {e}")))?;

        let url = format!("{}/api/edge/sync", self.cloud_url);

        let response = self
            .client
            .post(&url)
            .header("X-Signed-Binding", &binding_json)
            .json(&batch)
            .send()
            .await
            .map_err(|e| {
                let mut msg = format!("Cloud sync request failed: {e}");
                let mut source: Option<&dyn StdError> = StdError::source(&e);
                while let Some(s) = source {
                    msg.push_str(&format!(" → {s}"));
                    source = s.source();
                }
                AppError::internal(msg)
            })?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(AppError::internal(format!(
                "Cloud sync failed with status {status}: {body}"
            )));
        }

        let sync_response: CloudSyncResponse = response
            .json()
            .await
            .map_err(|e| AppError::internal(format!("Failed to parse sync response: {e}")))?;

        Ok(sync_response)
    }

    pub fn edge_id(&self) -> &str {
        &self.edge_id
    }
}
