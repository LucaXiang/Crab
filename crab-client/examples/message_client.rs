//! Interactive Message Client Example with mTLS support
//!
//! Demonstrates an interactive MessageClient that can:
//! 1. Authenticate with Auth Server to get mTLS certificates
//! 2. Connect to Edge Server using mTLS
//! 3. Send/Receive messages
//!
//! Run: cargo run --example message_client

use crab_client::{BusMessage, MessageClient};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme};
use std::io::{self, Write};
use std::sync::Arc;
use webpki;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Install default crypto provider (ring)
    rustls::crypto::ring::default_provider()
        .install_default()
        .expect("Failed to install rustls crypto provider");

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("\n🦀 Interactive Message Client (mTLS)");
    println!("=====================================\n");

    let auth_url = get_input_with_default("Auth Server URL", "http://localhost:3001");
    let edge_addr = get_input_with_default("Edge Server Address", "127.0.0.1:8082");

    // 1. Authenticate
    println!("\n🔑 Authentication required");
    let username = get_input("Username: ");
    let password = get_input("Password: ");

    println!("Connecting to Auth Server...");
    let http_client = reqwest::Client::new();

    let login_res = http_client
        .post(format!("{}/api/auth/login", auth_url))
        .json(&serde_json::json!({
            "username": username,
            "password": password
        }))
        .send()
        .await?;

    if !login_res.status().is_success() {
        return Err(format!("Login failed: {}", login_res.text().await?).into());
    }

    let login_data: serde_json::Value = login_res.json().await?;
    let token = login_data["token"]
        .as_str()
        .ok_or("No token in login response")?
        .to_string();

    println!("✅ Login successful! Token received.");

    // 2. Request Certificate
    println!("\n📜 Requesting Client Certificate...");
    let tenant_id = get_input_with_default("Tenant ID", "tenant-123");
    let common_name = get_input_with_default("Common Name", "pos-device-1");

    let issue_res = http_client
        .post(format!("{}/api/cert/issue", auth_url))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "tenant_id": tenant_id,
            "common_name": common_name,
            "is_server": false
        }))
        .send()
        .await?;

    if !issue_res.status().is_success() {
        return Err(format!("Cert issuance failed: {}", issue_res.text().await?).into());
    }

    let cert_data: serde_json::Value = issue_res.json().await?;

    let cert_pem = cert_data["cert"].as_str().ok_or("No cert received")?;
    let key_pem = cert_data["key"].as_str().ok_or("No key received")?;
    let tenant_ca_pem = cert_data["tenant_ca_cert"]
        .as_str()
        .ok_or("No tenant CA received")?;

    println!("✅ Certificate received!");

    // 3. Configure mTLS
    println!("\n🔐 Configuring mTLS...");

    // Load Client Cert/Key
    let mut cert_reader = std::io::Cursor::new(cert_pem);
    let certs: Vec<CertificateDer> =
        rustls_pemfile::certs(&mut cert_reader).collect::<Result<_, _>>()?;

    let mut key_reader = std::io::Cursor::new(key_pem);
    let key = rustls_pemfile::private_key(&mut key_reader)?.ok_or("No private key found")?;

    // Load Tenant CA as Root
    let mut roots = RootCertStore::empty();
    let mut ca_reader = std::io::Cursor::new(tenant_ca_pem);
    for cert in rustls_pemfile::certs(&mut ca_reader) {
        roots.add(cert?)?;
    }

    // Custom Verifier that skips hostname check
    // This allows connecting via IP (e.g. 192.168.1.x) while still verifying the chain against Tenant CA.
    let verifier = Arc::new(SkipHostnameVerifier::new(Arc::new(roots)));

    let tls_config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(verifier)
        .with_client_auth_cert(certs, key)?;

    println!("✅ mTLS Configuration ready.");

    // 4. Connect to Edge Server
    println!("\n📡 Connecting to Edge Server at {} (mTLS)...", edge_addr);

    let client = MessageClient::connect_tls(
        &edge_addr,
        "edge-server", // This matches the Server Cert CN (though ignored by verifier)
        tls_config,
    )
    .await?;

    println!("✅ Connected successfully to Edge Server via mTLS!\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // ... Interactive loop (same as before) ...
    interactive_loop(client).await
}

async fn interactive_loop(client: MessageClient) -> Result<(), Box<dyn std::error::Error>> {
    // Spawn a task to receive and display messages
    let recv_client = client.clone();
    tokio::spawn(async move {
        loop {
            match recv_client.recv().await {
                Ok(msg) => {
                    print_received_message(&msg);
                }
                Err(e) => {
                    eprintln!("\n❌ Connection error: {}", e);
                    break;
                }
            }
        }
    });

    // Interactive menu loop
    loop {
        print_menu();
        io::stdout().flush()?;

        let choice = get_input("Enter choice (0-8): ");

        match choice.as_str() {
            "0" => {
                println!("\n👋 Goodbye!");
                break;
            }
            "1" => {
                // Add dish to table
                let table_id = get_input("Table ID (e.g., T01): ");
                let dish_name = get_input("Dish name: ");
                let quantity = get_input("Quantity: ").parse::<u32>().unwrap_or(1);

                let payload = shared::message::OrderIntentPayload::add_dish(
                    shared::message::TableId::new_unchecked(table_id),
                    vec![shared::message::DishItem::simple(&dish_name, quantity)],
                    Some(shared::message::OperatorId::new("client_user")),
                );
                let msg = BusMessage::order_intent(&payload);
                client.send(&msg).await?;
            }
            "2" => {
                // Payment request
                let table_id = get_input("Table ID: ");
                let _amount = get_input("Amount (cents): ").parse::<u64>().unwrap_or(0);
                let method = get_input("Payment method (cash/card/wechat/alipay): ");

                let payment_method = match method.to_lowercase().as_str() {
                    "cash" => shared::message::PaymentMethod::Cash,
                    "card" => shared::message::PaymentMethod::Card,
                    "wechat" => shared::message::PaymentMethod::Wechat,
                    "alipay" => shared::message::PaymentMethod::Alipay,
                    _ => shared::message::PaymentMethod::Cash,
                };

                let payload = shared::message::OrderIntentPayload::checkout(
                    shared::message::TableId::new_unchecked(table_id.clone()),
                    shared::message::OrderId::new_unchecked("ORD_CLIENT"),
                    payment_method,
                    Some(shared::message::OperatorId::new("client_user")),
                );
                let msg = BusMessage::order_intent(&payload);
                client.send(&msg).await?;
            }
            "3" => {
                // Checkout
                let table_id = get_input("Table ID: ");

                let payload = shared::message::OrderIntentPayload::checkout(
                    shared::message::TableId::new_unchecked(table_id),
                    shared::message::OrderId::new_unchecked("ORD_CLIENT"),
                    shared::message::PaymentMethod::Cash,
                    Some(shared::message::OperatorId::new("client_user")),
                );
                let msg = BusMessage::order_intent(&payload);
                client.send(&msg).await?;
            }
            "4" => {
                // Dish price update
                let dish_id = get_input("Dish ID: ");
                let new_price = get_input("New price (cents): ").parse::<u64>().unwrap_or(0);

                let payload = shared::message::DataSyncPayload::DishPrice {
                    dish_id: shared::message::DishId::new(dish_id),
                    old_price: 0,
                    new_price,
                };
                let msg = BusMessage::data_sync(&payload);
                client.send(&msg).await?;
            }
            "5" => {
                // Dish sold out
                let dish_id = get_input("Dish ID: ");

                let payload = shared::message::DataSyncPayload::DishSoldOut {
                    dish_id: shared::message::DishId::new(dish_id),
                    available: false,
                };
                let msg = BusMessage::data_sync(&payload);
                client.send(&msg).await?;
            }
            "6" => {
                // System notification
                let title = get_input("Notification title: ");
                let body = get_input("Notification body: ");

                let payload = shared::message::NotificationPayload::info(title, body);
                let msg = BusMessage::notification(&payload);
                client.send(&msg).await?;
            }
            "7" => {
                // Server command
                let command_str = get_input("Command (ping/config_update/restart): ");

                let command = match command_str.to_lowercase().as_str() {
                    "ping" => shared::message::ServerCommand::Ping,
                    "restart" => shared::message::ServerCommand::Restart {
                        delay_seconds: 5,
                        reason: Some("client".to_string()),
                    },
                    "config_update" | _ => shared::message::ServerCommand::ConfigUpdate {
                        key: "client.config".to_string(),
                        value: serde_json::json!("client_value"),
                    },
                };

                let payload = shared::message::ServerCommandPayload { command };
                let msg = BusMessage::server_command(&payload);
                client.send(&msg).await?;
            }
            "8" => {
                // Custom JSON
                println!("Enter custom JSON payload:");
                let json_str = get_input("JSON: ");

                match serde_json::from_str::<serde_json::Value>(&json_str) {
                    Ok(value) => {
                        let msg_type = value
                            .get("type")
                            .and_then(|v| v.as_str())
                            .unwrap_or("notification");
                        let msg = match msg_type {
                            "order_intent" => {
                                let table_id = value
                                    .get("table_id")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("T01");

                                let payload = shared::message::OrderIntentPayload::add_dish(
                                    shared::message::TableId::new_unchecked(table_id),
                                    vec![shared::message::DishItem::simple("custom_dish", 1)],
                                    Some(shared::message::OperatorId::new("client_user")),
                                );
                                BusMessage::order_intent(&payload)
                            }
                            _ => {
                                let payload = shared::message::NotificationPayload::info(
                                    "Custom".to_string(),
                                    json_str,
                                );
                                BusMessage::notification(&payload)
                            }
                        };
                        client.send(&msg).await?;
                    }
                    Err(e) => println!("❌ Invalid JSON: {}", e),
                }
            }
            _ => println!("❌ Invalid choice"),
        }
    }

    Ok(())
}

fn print_received_message(msg: &BusMessage) {
    println!("\n📨 Received: [{}]", msg.event_type);

    match msg.parse_payload::<serde_json::Value>() {
        Ok(payload) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&payload).unwrap_or_default()
            );
        }
        Err(_) => {
            println!("(Raw payload: {} bytes)", msg.payload.len());
        }
    }
    print!("\n> "); // Restore prompt
    let _ = io::stdout().flush();
}

fn print_menu() {
    println!("\nAvailable Actions:");
    println!("1. Add Dish (OrderIntent)");
    println!("2. Payment (OrderIntent)");
    println!("3. Checkout (OrderIntent)");
    println!("4. Update Dish Price (DataSync)");
    println!("5. Set Dish Sold Out (DataSync)");
    println!("6. Send Notification");
    println!("7. Server Command");
    println!("8. Custom JSON");
    println!("0. Exit");
}

fn get_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn get_input_with_default(prompt: &str, default: &str) -> String {
    print!("{} [{}]: ", prompt, default);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    let input = input.trim();
    if input.is_empty() {
        default.to_string()
    } else {
        input.to_string()
    }
}

#[derive(Debug)]
struct SkipHostnameVerifier {
    roots: Arc<RootCertStore>,
    inner: Arc<dyn ServerCertVerifier>,
}

impl SkipHostnameVerifier {
    fn new(roots: Arc<RootCertStore>) -> Self {
        let inner = rustls::client::WebPkiServerVerifier::builder(roots.clone())
            .build()
            .unwrap();
        Self { roots, inner }
    }
}

impl ServerCertVerifier for SkipHostnameVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        let cert = webpki::EndEntityCert::try_from(end_entity).map_err(|e| {
            rustls::Error::InvalidCertificate(rustls::CertificateError::Other(rustls::OtherError(
                Arc::new(e),
            )))
        })?;

        // Use verify_for_usage instead of verify_is_valid_tls_server_cert
        // Pass anchors directly from RootCertStore (Vec<TrustAnchor>)
        cert.verify_for_usage(
            &webpki::ALL_VERIFICATION_ALGS,
            &self.roots.roots,
            intermediates,
            now,
            webpki::KeyUsage::server_auth(),
            None, // No CRLs
            None, // No revocation policy?
        )
        .map_err(|e| {
            rustls::Error::InvalidCertificate(rustls::CertificateError::Other(rustls::OtherError(
                Arc::new(e),
            )))
        })?;

        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        self.inner.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.inner.supported_verify_schemes()
    }
}
