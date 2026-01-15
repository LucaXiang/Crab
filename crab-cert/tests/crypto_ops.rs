use crab_cert::{
    CaProfile, CertProfile, CertificateAuthority, KeyType, decrypt, encrypt, sign, verify,
};

#[test]
fn test_rsa_crypto_ops() {
    // 1. Create RSA Root CA
    let mut profile = CaProfile::default();
    profile.common_name = "RSA Root CA".to_string();
    profile.key_type = KeyType::Rsa2048;

    let ca = CertificateAuthority::new_root(profile).expect("Failed to create RSA CA");

    // 2. Issue RSA Certificate
    let mut server_profile = CertProfile::new_server(
        "rsa.local",
        vec!["rsa.local".into()],
        None,
        "rsa-device".to_string(),
    );
    server_profile.key_type = KeyType::Rsa2048;

    let (cert_pem, key_pem) = ca
        .issue_cert(&server_profile)
        .expect("Failed to issue RSA cert");

    let data = b"Hello Crab Security!";

    // 3. Test Signing
    let signature = sign(&key_pem, data).expect("Signing failed");
    verify(&cert_pem, data, &signature).expect("Verification failed");

    // 4. Test Encryption
    let ciphertext = encrypt(&cert_pem, data).expect("Encryption failed");
    let decrypted = decrypt(&key_pem, &ciphertext).expect("Decryption failed");

    assert_eq!(data, &decrypted[..]);
}

#[test]
fn test_ecdsa_crypto_ops() {
    // 1. Create ECDSA Root CA (Default)
    let profile = CaProfile::default(); // Defaults to P-256

    let ca = CertificateAuthority::new_root(profile).expect("Failed to create ECDSA CA");

    // 2. Issue ECDSA Certificate
    let server_profile = CertProfile::new_server(
        "ecdsa.local",
        vec!["ecdsa.local".into()],
        None,
        "ecdsa-device".to_string(),
    );
    // Defaults to P-256

    let (cert_pem, key_pem) = ca
        .issue_cert(&server_profile)
        .expect("Failed to issue ECDSA cert");

    let data = b"Hello Crab ECDSA!";

    // 3. Test Signing
    let signature = sign(&key_pem, data).expect("Signing failed");
    verify(&cert_pem, data, &signature).expect("Verification failed");

    // 4. Test Encryption (Should Fail)
    let result = encrypt(&cert_pem, data);
    assert!(result.is_err(), "Encryption should fail for ECDSA");
}
