//! Generate production certs for crab-cloud mTLS server
//! - Root CA (stored in Secrets Manager format)
//! - Server cert for sync.redcoral.app

use crab_cert::{CaProfile, CertProfile, CertificateAuthority, KeyType};
use std::fs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = "deploy/ec2/certs";
    fs::create_dir_all(out_dir)?;

    // 1. Generate Root CA
    println!("Generating Root CA...");
    let root_ca = CertificateAuthority::new_root(CaProfile {
        common_name: "Crab Root CA".to_string(),
        organization: "Crab Inc.".to_string(),
        validity_days: 365 * 20,
        path_len: Some(1),
        key_type: KeyType::P256,
    })?;

    // Save root_ca.pem (public cert only - for client verification)
    fs::write(format!("{out_dir}/root_ca.pem"), root_ca.cert_pem())?;
    println!("  root_ca.pem written");

    // Output Secrets Manager JSON for crab-auth/root-ca
    let sm_json = serde_json::json!({
        "cert_pem": root_ca.cert_pem(),
        "key_pem": root_ca.key_pem(),
    });
    fs::write(
        format!("{out_dir}/root_ca_secret.json"),
        sm_json.to_string(),
    )?;
    println!("  root_ca_secret.json written (for Secrets Manager)");

    // 2. Generate server cert for sync.redcoral.app
    println!("Generating server cert for sync.redcoral.app...");
    let server_profile = CertProfile {
        common_name: "sync.redcoral.app".to_string(),
        organization: "Crab Cloud".to_string(),
        sans: vec![
            "sync.redcoral.app".to_string(),
            "cloud.redcoral.app".to_string(),
        ],
        validity_days: 365 * 2, // 2 years
        is_client: false,
        is_server: true,
        tenant_id: None,
        device_id: None,
        client_name: None,
        key_type: KeyType::P256,
    };

    let (cert_pem, key_pem) = root_ca.issue_cert(&server_profile)?;
    fs::write(format!("{out_dir}/server.pem"), &cert_pem)?;
    fs::write(format!("{out_dir}/server.key"), &key_pem)?;
    println!("  server.pem + server.key written");

    println!("\nDone! Files in {out_dir}/");
    println!("Next steps:");
    println!("  1. Upload root_ca_secret.json to Secrets Manager as crab-auth/root-ca");
    println!("  2. SCP root_ca.pem, server.pem, server.key to EC2:/opt/crab/certs/");

    Ok(())
}
