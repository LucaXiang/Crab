//! Interactive Demo - Complete Edge Server Experience
//!
//! This example demonstrates:
//! 1. Starting edge-server
//! 2. Using MessageClient for simple send/recv interface
//! 3. Multiple event receivers (both client and server messages)
//! 4. Interactive command line to send messages
//!
//! Run: cargo run --example interactive_demo

use edge_server::Config;
use edge_server::server::ServerState;
use edge_server::{BusMessage, MessageClient};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Install default crypto provider to avoid panic
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();

    // Set default log level to info/debug if not set
    let env_filter = std::env::var("RUST_LOG")
        .unwrap_or_else(|_| "info,edge_server=debug,surrealdb=warn".to_string());

    tracing_subscriber::fmt().with_env_filter(env_filter).init();

    println!("\n🦀 Interactive Edge Server Demo");
    println!("================================\n");

    // Create a temporary directory for this demo
    let temp_dir = "./temp_interactive_demo";
    std::fs::create_dir_all(temp_dir).ok();

    // 1. Initialize edge-server state
    tracing::info!("1️⃣  Initializing edge-server state...");
    let mut config = Config::with_overrides(temp_dir, 3000, 8081);
    config.environment = "development".to_string();
    config.jwt.secret = "demo-secret".to_string();

    let state: ServerState = ServerState::initialize(&config).await;
    state.start_background_tasks().await;

    // Check activation status and print banner
    state.print_activation_banner().await;

    // Start TCP Server if activated and certs exist
    if let Ok(Some(tls_config)) = state.load_tls_config() {
        let bus = state.get_message_bus();
        let tcp_tls_config = tls_config.clone();
        tokio::spawn(async move {
            tracing::info!("Starting Message Bus TCP server...");
            if let Err(e) = bus.start_tcp_server(Some(tcp_tls_config)).await {
                tracing::error!("Message bus TCP server error: {}", e);
            }
        });
    } else {
        tracing::warn!("TCP Server not started (Not activated or missing certs)");
    }

    // 2. Start event receiver that listens to BOTH client and server messages
    tracing::info!("2️⃣  Starting event receiver...");
    let bus = state.get_message_bus();

    // Create two receivers: one for client messages, one for server broadcasts
    let mut client_rx = bus.subscribe_to_clients();
    let mut server_rx = bus.subscribe();

    tokio::spawn(async move {
        tracing::info!("📨 Event receiver started (listening to clients + server)");
        loop {
            tokio::select! {
                // Receive messages FROM CLIENTS (interactive_demo sends via publish)
                msg_result = client_rx.recv() => {
                    match msg_result {
                        Ok(msg) => {
                            {
                                tracing::info!("📨 [来自客户端] {:?}", msg.event_type);
                            }
                            if let Ok(payload) = msg.parse_payload::<serde_json::Value>() {
                                tracing::info!("   Data: {}", payload);
                            }
                        }
                        Err(_) => break,
                    }
                }
                // Receive broadcasts FROM SERVER (TCP clients would receive this)
                msg_result = server_rx.recv() => {
                    match msg_result {
                        Ok(msg) => {
                            match msg.event_type {
                                edge_server::message::EventType::Notification => {
                                    tracing::info!("📢 [服务端广播] NOTIFICATION | 系统通知");
                                }
                                edge_server::message::EventType::ServerCommand => {
                                    tracing::info!("🎮 [服务端广播] SERVER COMMAND | 服务器指令");
                                }
                            }
                            if let Ok(payload) = msg.parse_payload::<serde_json::Value>() {
                                tracing::info!("   Data: {}", payload);
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        }
    });

    tracing::info!("3️⃣  Receiver started!");

    // Give the receiver a moment to print its startup message
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 3. Interactive command line
    interactive_cli(state).await?;

    tracing::info!("👋 Demo complete!");
    tracing::info!("Cleaning up...");

    // Cleanup - DISABLED to allow persistence testing
    // std::fs::remove_dir_all(temp_dir).ok();

    Ok(())
}

async fn interactive_cli(state: ServerState) -> Result<(), Box<dyn std::error::Error>> {
    // Create message client for simple send/recv interface
    let msg_client = MessageClient::memory(&state.get_message_bus());

    loop {
        print_menu();
        io::stdout().flush()?;

        let choice = get_input("Enter your choice (0-4): ");

        match choice.as_str() {
            "0" => {
                println!("\n👋 Goodbye!");
                break;
            }
            "1" => {
                // Send Notification (client message via send)
                let title = get_input("Title: ");
                let message = get_input("Message: ");

                let payload = shared::message::NotificationPayload::info(&title, &message);
                let msg = BusMessage::notification(&payload);
                msg_client.send(&msg).await?;
                println!("✅ Sent: Notification '{}'\n", title);
            }
            "2" => {
                // Server Command (server broadcast via publish)
                let command_str = get_input("Command (ping/config_update/restart): ");
                let args_str = get_input("Args (json, optional): ");
                let args: serde_json::Value =
                    serde_json::from_str(&args_str).unwrap_or(serde_json::json!({}));

                let command = match command_str.to_lowercase().as_str() {
                    "ping" => shared::message::ServerCommand::Ping,
                    "restart" => shared::message::ServerCommand::Restart {
                        delay_seconds: args.get("delay").and_then(|v| v.as_u64()).unwrap_or(5)
                            as u32,
                        reason: args
                            .get("reason")
                            .and_then(|v| v.as_str())
                            .map(String::from),
                    },
                    "config_update" | _ => shared::message::ServerCommand::ConfigUpdate {
                        key: args
                            .get("key")
                            .and_then(|v| v.as_str())
                            .unwrap_or("demo.key")
                            .to_string(),
                        value: args
                            .get("value")
                            .cloned()
                            .unwrap_or(serde_json::json!("demo_value")),
                    },
                };

                let payload = shared::message::ServerCommandPayload { command };
                let msg = BusMessage::server_command(&payload);
                state.get_message_bus().publish(msg).await?;
                println!("✅ Sent: Server Command '{}'\n", command_str);
            }
            "3" => {
                // Custom JSON (raw message)
                let json_data = get_input("JSON Data: ");

                let payload = shared::message::NotificationPayload::info("Custom", &json_data);
                let msg = BusMessage::notification(&payload);
                msg_client.send(&msg).await?;
                println!("✅ Sent: Custom Data as Notification\n");
            }
            "4" => {
                // Activate Server (Real Auth Server)
                println!("\n🔐 Activating Server (Real Auth Server)...");

                let auth_url = get_input("Auth Server URL (default: http://localhost:3001): ");
                let auth_url = if auth_url.is_empty() {
                    "http://localhost:3001".to_string()
                } else {
                    auth_url
                };

                let mut username = get_input("Username (default: admin): ");
                if username.is_empty() {
                    username = "admin".to_string();
                }

                let mut password = get_input("Password (default: admin123): ");
                if password.is_empty() {
                    password = "admin123".to_string();
                }

                let mut tenant_id = get_input("Tenant ID (default: tenant-01): ");
                if tenant_id.is_empty() {
                    tenant_id = "tenant-01".to_string();
                }

                let mut edge_id = get_input("Edge Server ID (default: edge-01): ");
                if edge_id.is_empty() {
                    edge_id = "edge-01".to_string();
                }

                println!("   Connecting to Auth Server...");

                // Use the internal provisioning service
                let provisioning = state.provisioning_service(auth_url);

                match provisioning
                    .activate(&username, &password, &tenant_id, &edge_id)
                    .await
                {
                    Ok(_) => {
                        println!("\n✨ Activation Successful! ✨");
                        println!("Tenant: {}", tenant_id);
                        println!("Edge ID: {}", edge_id);
                        // The server state automatically reloads certificates, so we don't strictly need to restart for the demo to work
                        println!("\n✅ Server state updated with new certificates.");
                    }
                    Err(e) => {
                        println!("❌ Activation failed: {}", e);
                    }
                }
            }
            _ => println!("❌ Invalid choice, please try again.\n"),
        }
    }
    Ok(())
}

fn print_menu() {
    println!("\nAvailable Actions:");
    println!("1. Send Notification");
    println!("2. Server Command");
    println!("3. Custom JSON (wrapped in Notification)");
    println!("4. Activate Server (Real Auth Server)");
    println!("0. Exit");
}

fn get_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut buffer = String::new();
    io::stdin().read_line(&mut buffer).unwrap();
    buffer.trim().to_string()
}
