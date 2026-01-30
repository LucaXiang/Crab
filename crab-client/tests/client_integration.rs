//! Integration tests for crab-client.
//!
//! These tests verify the client API and credential management.

use crab_client::{CertManager, CrabClient, Credential, CredentialStorage};
use tempfile::TempDir;

// ============================================================================
// Credential Storage Tests
// ============================================================================

#[tokio::test]
async fn test_credential_storage() {
    let temp_dir = TempDir::new().unwrap();
    let storage = CredentialStorage::new(temp_dir.path(), "credential.json");

    // Test save and load
    let credential = Credential::new(
        "test-client".to_string(),
        "tenant-1".to_string(),
        "test-token".to_string(),
        None,
    );

    storage.save(&credential).unwrap();
    assert!(storage.exists());

    let loaded = storage.load().unwrap();
    assert_eq!(loaded.client_name, "test-client");
    assert_eq!(loaded.tenant_id, "tenant-1");
    assert_eq!(loaded.token(), "test-token");

    // Test delete
    storage.delete().unwrap();
    assert!(!storage.exists());
    assert!(storage.load().is_none());
}

#[tokio::test]
async fn test_credential_is_expired() {
    // Credential without expiry
    let cred1 = Credential::new("c", "t", "tok", None);
    assert!(!cred1.is_expired());
    assert!(cred1.is_valid());

    // Credential with future expiry
    let future = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        + 3600;
    let cred2 = Credential::new("c", "t", "tok", Some(future));
    assert!(!cred2.is_expired());
    assert!(cred2.is_valid());

    // Credential with past expiry
    let past = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
        - 3600;
    let cred3 = Credential::new("c", "t", "tok", Some(past));
    assert!(cred3.is_expired());
    assert!(!cred3.is_valid());
}

// ============================================================================
// CertManager Tests
// ============================================================================

#[tokio::test]
async fn test_cert_manager_creation() {
    let temp_dir = TempDir::new().unwrap();
    let manager = CertManager::new(temp_dir.path(), "test-client");

    assert!(!manager.has_credential());
    assert!(!manager.has_local_certificates());
    assert_eq!(manager.client_name(), "test-client");

    // Credential path should be: {base_path}/{client_name}/credential.json
    let expected_path = temp_dir.path().join("test-client").join("credential.json");
    assert_eq!(manager.credential_path(), &expected_path);
}

// ============================================================================
// Remote Client Builder Tests
// ============================================================================

#[tokio::test]
async fn test_remote_client_builder() {
    let temp_dir = TempDir::new().unwrap();

    // Build remote client
    let client = CrabClient::remote()
        .auth_server("https://auth.example.com")
        .cert_path(temp_dir.path())
        .client_name("pos-01")
        .build()
        .expect("Failed to build remote client");

    // Check initial state
    assert!(!client.has_cached_credentials());
    assert!(!client.is_connected());
    assert!(!client.is_authenticated());
    assert!(client.token().is_none());

    // Check client name
    assert_eq!(client.client_name(), Some("pos-01"));
}

#[tokio::test]
async fn test_remote_client_builder_missing_config() {
    // Missing auth_server
    let result = CrabClient::remote()
        .cert_path("./certs")
        .client_name("pos-01")
        .build();
    assert!(result.is_err());

    // Missing cert_path
    let result = CrabClient::remote()
        .auth_server("https://auth.example.com")
        .client_name("pos-01")
        .build();
    assert!(result.is_err());

    // Missing client_name
    let result = CrabClient::remote()
        .auth_server("https://auth.example.com")
        .cert_path("./certs")
        .build();
    assert!(result.is_err());
}

// ============================================================================
// Local Client Builder Tests (requires "in-process" feature)
// ============================================================================

#[cfg(feature = "in-process")]
mod local_tests {
    use super::*;
    use axum::Router;
    use shared::message::BusMessage;
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn test_local_client_builder() {
        // Create a simple router and sender for testing
        let router: Router = Router::new();
        let (sender, _) = broadcast::channel::<BusMessage>(16);

        // Build local client
        let client = CrabClient::local()
            .with_router(router)
            .with_message_channels(sender.clone(), sender)
            .build()
            .expect("Failed to build local client");

        // For Local mode, once router and sender are configured,
        // the client is considered "connected" (in-memory channels are always ready)
        assert!(client.is_connected());
        assert!(!client.is_authenticated());
        assert!(client.token().is_none());
    }

    #[tokio::test]
    async fn test_local_client_builder_missing_router() {
        let (sender, _) = broadcast::channel::<BusMessage>(16);

        // Missing router
        let result = CrabClient::local().with_message_channels(sender.clone(), sender).build();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_local_client_builder_missing_sender() {
        let router: Router = Router::new();

        // Missing message_sender
        let result = CrabClient::local().with_router(router).build();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_local_client_connect() {
        let router: Router = Router::new();
        let (sender, _) = broadcast::channel::<BusMessage>(16);

        let client = CrabClient::local()
            .with_router(router)
            .with_message_channels(sender.clone(), sender)
            .build()
            .unwrap();

        // Connect (should succeed since router and sender are configured)
        let client = client.connect().await.unwrap();

        // Now connected
        assert!(client.is_connected());
        assert!(!client.is_authenticated());
    }
}

// ============================================================================
// Client Status Tests
// ============================================================================

#[tokio::test]
async fn test_client_status() {
    let temp_dir = TempDir::new().unwrap();

    let client = CrabClient::remote()
        .auth_server("https://auth.example.com")
        .cert_path(temp_dir.path())
        .client_name("pos-01")
        .build()
        .unwrap();

    let status = client.status();
    assert!(!status.has_tenant_credential);
    assert!(!status.has_certificates);
    assert!(!status.is_connected);
    assert!(!status.is_authenticated);
}
