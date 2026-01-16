use crab_cert::{
    CaProfile, CertMetadata, CertProfile, CertificateAuthority, to_identity_pem, to_rustls_certs,
    to_rustls_key, verify_client_cert, verify_server_cert,
};
use std::sync::Once;

static INIT: Once = Once::new();

fn init_crypto() {
    INIT.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

#[test]
fn test_certificate_chain() {
    init_crypto();
    // 1. Create Root CA
    println!("Creating Root CA...");
    let mut root_profile = CaProfile::default();
    root_profile.common_name = "Crab Root CA".to_string();
    let root_ca = CertificateAuthority::new_root(root_profile).expect("Failed to create Root CA");

    // 2. Create Intermediate CA
    println!("Creating Intermediate CA...");
    let mut intermediate_profile = CaProfile::default();
    intermediate_profile.common_name = "Crab Intermediate CA".to_string();
    let intermediate_ca = CertificateAuthority::new_intermediate(intermediate_profile, &root_ca)
        .expect("Failed to create Intermediate CA");

    // 3. Issue Leaf Certificate from Intermediate
    println!("Issuing Leaf Certificate...");
    let leaf_profile = CertProfile::new_server(
        "leaf.local",
        vec!["leaf.local".to_string()],
        None,
        "leaf-device".to_string(),
    );
    let (leaf_cert_pem, _leaf_key_pem) = intermediate_ca
        .issue_cert(&leaf_profile)
        .expect("Failed to issue leaf cert");

    // 4. Verify Chain
    // To verify leaf signed by intermediate, we need to trust the Root.
    // The intermediate cert itself must be presented in the chain or trust store?
    // In typical TLS, the server sends [Leaf, Intermediate]. The client trusts Root.

    // Construct the chain PEM: Leaf + Intermediate
    let chain_pem = format!("{}\n{}", leaf_cert_pem, intermediate_ca.cert_pem());

    println!("Verifying Leaf Chain against Root...");
    verify_server_cert(&chain_pem, root_ca.cert_pem()).expect("Chain verification failed");

    println!("Chain verification passed!");
}

#[test]
fn test_ca_load() {
    init_crypto();
    // 1. Create and persist a Root CA
    let mut profile = CaProfile::default();
    profile.common_name = "Crab Loaded CA".to_string();
    let original_ca = CertificateAuthority::new_root(profile).expect("Failed to create Root CA");

    let cert_pem = original_ca.cert_pem();
    let key_pem = original_ca.key_pem();

    // 2. Load CA from PEM
    let loaded_ca = CertificateAuthority::load(cert_pem, &key_pem).expect("Failed to load CA");

    // 3. Verify loaded CA can issue certificates
    let server_profile = CertProfile::new_server(
        "loaded.local",
        vec!["loaded.local".to_string()],
        None,
        "loaded-device".to_string(),
    );
    let (server_cert, _) = loaded_ca
        .issue_cert(&server_profile)
        .expect("Failed to issue cert from loaded CA");

    // 4. Verify the issued certificate against the original CA cert
    verify_server_cert(&server_cert, cert_pem).expect("Verification failed");
}

#[test]
fn test_file_io() {
    use std::fs;

    let temp_dir = std::env::temp_dir().join("crab-cert-test");
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir).expect("Failed to clean temp dir");
    }
    fs::create_dir_all(&temp_dir).expect("Failed to create temp dir");

    // 1. Create and Save Root CA
    let mut profile = CaProfile::default();
    profile.common_name = "Crab File IO CA".to_string();
    let ca = CertificateAuthority::new_root(profile).expect("Failed to create Root CA");

    ca.save(&temp_dir, "root_ca").expect("Failed to save CA");

    let cert_path = temp_dir.join("root_ca.crt");
    let key_path = temp_dir.join("root_ca.key");
    assert!(cert_path.exists());
    assert!(key_path.exists());

    // 2. Load CA from file
    let loaded_ca = CertificateAuthority::load_from_file(&cert_path, &key_path)
        .expect("Failed to load CA from file");

    assert_eq!(ca.cert_pem(), loaded_ca.cert_pem());
    assert_eq!(ca.key_pem(), loaded_ca.key_pem());

    // 3. Issue cert and test metadata from file
    let server_profile = CertProfile::new_server(
        "file-io.local",
        vec!["file-io.local".into()],
        None,
        "file-io-device".to_string(),
    );
    let (cert_pem, _) = loaded_ca
        .issue_cert(&server_profile)
        .expect("Failed to issue cert");

    let cert_file_path = temp_dir.join("server.crt");
    fs::write(&cert_file_path, cert_pem).expect("Failed to write server cert");

    let metadata =
        CertMetadata::from_pem_file(&cert_file_path).expect("Failed to load metadata from file");
    assert_eq!(metadata.common_name.as_deref(), Some("file-io.local"));

    // Cleanup
    fs::remove_dir_all(&temp_dir).expect("Failed to cleanup temp dir");
}

#[test]
fn test_certificate_lifecycle() {
    init_crypto();
    println!("Creating Root CA...");
    let mut ca_profile = CaProfile::default();
    ca_profile.common_name = "Crab Test Root CA".to_string();
    let ca = CertificateAuthority::new_root(ca_profile.clone()).expect("Failed to create Root CA");

    // Verify CA PEM structure
    assert!(ca.cert_pem().contains("BEGIN CERTIFICATE"));
    assert!(ca.key_pem().contains("PRIVATE KEY"));

    // 2. Issue Server Certificate
    println!("Issuing Server Certificate...");
    let server_profile = CertProfile::new_server(
        "localhost",
        vec!["localhost".to_string(), "127.0.0.1".to_string()],
        None,
        "localhost-device".to_string(),
    );
    let (server_cert_pem, server_key_pem) = ca
        .issue_cert(&server_profile)
        .expect("Failed to issue server cert");

    // 3. Issue Client Certificate with Extensions
    println!("Issuing Client Certificate...");
    let tenant_id = "tenant-test-001";
    let device_id = "device-test-999";
    let client_profile = CertProfile::new_client(
        "client-device",
        Some(tenant_id.to_string()),
        Some(device_id.to_string()),
        Some("My Client Terminal".to_string()),
    );
    let (client_cert_pem, client_key_pem) = ca
        .issue_cert(&client_profile)
        .expect("Failed to issue client cert");

    // 4. Verify Metadata (Extensions and Fingerprint)
    println!("Verifying Metadata...");
    let metadata = CertMetadata::from_pem(&client_cert_pem).expect("Failed to parse metadata");

    assert_eq!(metadata.common_name.as_deref(), Some("client-device"));
    assert_eq!(metadata.tenant_id.as_deref(), Some(tenant_id));
    assert_eq!(metadata.device_id.as_deref(), Some(device_id));
    assert_eq!(metadata.client_name.as_deref(), Some("My Client Terminal"));

    // Verify serial number
    println!("Serial Number: {}", metadata.serial_number);
    assert!(!metadata.serial_number.is_empty());

    // Verify fingerprint presence
    println!("Fingerprint: {}", metadata.fingerprint_sha256);
    assert_eq!(metadata.fingerprint_sha256.len(), 64); // SHA256 hex length
    assert!(metadata.verify_fingerprint(&metadata.fingerprint_sha256));

    // 5. Verify Rustls Adapter
    println!("Verifying Rustls Adapter...");

    // Server Certs
    let rustls_certs = to_rustls_certs(&server_cert_pem).expect("Failed to convert server certs");
    assert_eq!(rustls_certs.len(), 1);
    let _rustls_key = to_rustls_key(&server_key_pem).expect("Failed to convert server key");

    // Identity PEM (for Reqwest)
    let identity_pem = to_identity_pem(&client_cert_pem, &client_key_pem);
    assert!(identity_pem.contains("CERTIFICATE"));
    assert!(identity_pem.contains("PRIVATE KEY"));

    // 6. Verify Server Certificate using helper
    println!("Verifying Server Certificate...");
    verify_server_cert(&server_cert_pem, ca.cert_pem())
        .expect("Server certificate verification failed");
    println!("Server certificate verification passed!");

    // 7. Verify Client Certificate using helper
    println!("Verifying Client Certificate...");
    verify_client_cert(&client_cert_pem, ca.cert_pem())
        .expect("Client certificate verification failed");
    println!("Client certificate verification passed!");

    println!("All tests passed!");
}
