// crab-client/tests/client_integration.rs
// 集成测试

use crab_client::{CrabClient, RemoteMode, LocalMode, Credential, CredentialStorage, CertManager};
use tempfile::TempDir;

#[tokio::test]
async fn test_credential_storage() {
    let temp_dir = TempDir::new().unwrap();
    let storage = CredentialStorage::new(temp_dir.path(), "test-client");

    // Test save and load
    let credential = Credential::new(
        "test-client".to_string(),
        "test-token".to_string(),
        None,
        "tenant-1".to_string(),
    );

    storage.save(&credential).unwrap();
    assert!(storage.exists());

    let loaded = storage.load().unwrap();
    assert_eq!(loaded.client_name, "test-client");
    assert_eq!(loaded.token, "test-token");
    assert_eq!(loaded.tenant_id, "tenant-1");

    // Test delete
    storage.delete().unwrap();
    assert!(!storage.exists());
    assert!(storage.load().is_none());
}

#[tokio::test]
async fn test_credential_is_expired() {
    // Credential without expiry
    let cred1 = Credential::new("test".to_string(), "token".to_string(), None, "tenant".to_string());
    assert!(!cred1.is_expired());

    // Credential with future expiry
    let future = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() + 3600;
    let cred2 = Credential::new("test".to_string(), "token".to_string(), Some(future), "tenant".to_string());
    assert!(!cred2.is_expired());

    // Credential with past expiry
    let past = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() - 3600;
    let cred3 = Credential::new("test".to_string(), "token".to_string(), Some(past), "tenant".to_string());
    assert!(cred3.is_expired());
}

#[tokio::test]
async fn test_cert_manager_creation() {
    let temp_dir = TempDir::new().unwrap();
    let manager = CertManager::new(temp_dir.path(), "test-client");

    assert!(!manager.has_credential());
    // CertManager appends client_name, so path is: base/client_name/credential.json
    let expected_path = temp_dir.path().join("test-client").join("credential.json");
    assert_eq!(manager.credential_path(), &expected_path);
}

#[tokio::test]
async fn test_remote_client_creation() {
    let client = CrabClient::<RemoteMode>::new("http://localhost:8080");
    assert!(!client.is_logged_in());
    assert!(client.token().is_none());
}

#[tokio::test]
async fn test_local_client_creation() {
    let client = CrabClient::<LocalMode>::new("http://localhost:8080");
    assert!(!client.is_logged_in());
    assert!(client.token().is_none());
}

#[tokio::test]
async fn test_token_access() {
    let client = CrabClient::<RemoteMode>::new("http://localhost:8080");
    assert!(client.token().is_none());
    assert!(!client.is_logged_in());
}
