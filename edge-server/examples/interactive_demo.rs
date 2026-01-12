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
    tracing_subscriber::fmt::init();

    println!("\n🦀 Interactive Edge Server Demo");
    println!("================================\n");

    // Create a temporary directory for this demo
    let temp_dir = "./temp_interactive_demo";
    std::fs::create_dir_all(temp_dir).ok();

    // 1. Start edge-server
    println!("1️⃣  Starting edge-server...");
    let config = Config {
        work_dir: temp_dir.to_string(),
        jwt: edge_server::server::auth::JwtConfig {
            secret: "demo-secret".to_string(),
            ..Default::default()
        },
        http_port: 3000,
        environment: "development".to_string(),
        message_tcp_port: 8081,
    };
    let state: ServerState = ServerState::initialize(&config).await;
    println!("✅ Edge-server started! (HTTP: 3000, TCP: 8081)\n");

    // 2. Start event receiver that listens to BOTH client and server messages
    println!("2️⃣  Starting event receiver...");
    let bus = state.get_message_bus();

    // Create two receivers: one for client messages, one for server broadcasts
    let mut client_rx = bus.subscribe_to_clients();
    let mut server_rx = bus.subscribe();

    tokio::spawn(async move {
        println!("📨 Event receiver started (listening to clients + server)\n");
        loop {
            tokio::select! {
                // Receive messages FROM CLIENTS (interactive_demo sends via publish)
                msg_result = client_rx.recv() => {
                    match msg_result {
                        Ok(msg) => {
                            match msg.event_type {
                                edge_server::message::EventType::OrderIntent => {
                                    println!("📝 [来自客户端] ORDER INTENT | 订单操作请求");
                                }
                                _ => {
                                    println!("📨 [来自客户端] {:?}", msg.event_type);
                                }
                            }
                            if let Ok(payload) = msg.parse_payload::<serde_json::Value>() {
                                println!("   Data: {}\n", payload);
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
                                edge_server::message::EventType::OrderSync => {
                                    println!("🔄 [服务端广播] ORDER SYNC | 订单状态同步");
                                }
                                edge_server::message::EventType::DataSync => {
                                    println!("💾 [服务端广播] DATA SYNC | 数据同步");
                                }
                                edge_server::message::EventType::Notification => {
                                    println!("📢 [服务端广播] NOTIFICATION | 系统通知");
                                }
                                edge_server::message::EventType::ServerCommand => {
                                    println!("🎮 [服务端广播] SERVER COMMAND | 服务器指令");
                                }
                                _ => {
                                    println!("📡 [服务端广播] {:?}", msg.event_type);
                                }
                            }
                            if let Ok(payload) = msg.parse_payload::<serde_json::Value>() {
                                println!("   Data: {}\n", payload);
                            }
                        }
                        Err(_) => break,
                    }
                }
            }
        }
    });

    println!("3️⃣  Receiver started!\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // 3. Interactive command line
    interactive_cli(state).await?;

    println!("\n👋 Demo complete!");
    println!("Cleaning up...");

    // Cleanup
    std::fs::remove_dir_all(temp_dir).ok();

    Ok(())
}

async fn interactive_cli(state: ServerState) -> Result<(), Box<dyn std::error::Error>> {
    // Create message client for simple send/recv interface
    let msg_client = MessageClient::memory(&state.get_message_bus());

    loop {
        print_menu();
        io::stdout().flush()?;

        let choice = get_input("Enter your choice (0-8): ");

        match choice.as_str() {
            "0" => {
                println!("\n👋 Goodbye!");
                break;
            }
            "1" => {
                // Add dish to table (client message via send)
                let table_id = get_input("Table ID (e.g., T01): ");
                let dish_name = get_input("Dish name: ");
                let quantity = get_input("Quantity: ").parse::<i32>().unwrap_or(1);

                let msg = BusMessage::order_intent(&serde_json::json!({
                    "action": "add_dish",
                    "table_id": table_id,
                    "dishes": [{
                        "name": dish_name,
                        "quantity": quantity
                    }],
                    "operator": "demo_user"
                }));
                msg_client.send(&msg).await?;
                println!("✅ Sent: Add dish to {}\n", table_id);
            }
            "2" => {
                // Payment request (client message via send)
                let table_id = get_input("Table ID: ");
                let amount = get_input("Amount (cents): ").parse::<i32>().unwrap_or(0);
                let method = get_input("Payment method (cash/card/wechat): ");

                let msg = BusMessage::order_intent(&serde_json::json!({
                    "action": "payment",
                    "table_id": table_id,
                    "amount": amount,
                    "method": method,
                    "operator": "demo_user"
                }));
                msg_client.send(&msg).await?;
                println!("✅ Sent: Payment for {}\n", table_id);
            }
            "3" => {
                // Checkout (client message via send)
                let table_id = get_input("Table ID: ");

                let msg = BusMessage::order_intent(&serde_json::json!({
                    "action": "checkout",
                    "table_id": table_id,
                    "operator": "demo_user"
                }));
                msg_client.send(&msg).await?;
                println!("✅ Sent: Checkout for {}\n", table_id);
            }
            "4" => {
                // Dish price update (server broadcast via publish)
                let dish_id = get_input("Dish ID: ");
                let new_price = get_input("New price (cents): ").parse::<i32>().unwrap_or(0);

                let msg = BusMessage::data_sync(
                    "dish_price",
                    serde_json::json!({
                        "dish_id": dish_id,
                        "new_price": new_price,
                        "updated_by": "demo_user"
                    }),
                );
                state.get_message_bus().publish(msg).await?;
                println!("✅ Sent: Price update for {}\n", dish_id);
            }
            "5" => {
                // Dish sold out (server broadcast via publish)
                let dish_id = get_input("Dish ID: ");

                let msg = BusMessage::data_sync(
                    "dish_sold_out",
                    serde_json::json!({
                        "dish_id": dish_id,
                        "available": false
                    }),
                );
                state.get_message_bus().publish(msg).await?;
                println!("✅ Sent: Marked {} as sold out\n", dish_id);
            }
            "6" => {
                // System notification (server broadcast via publish)
                let title = get_input("Notification title: ");
                let body = get_input("Notification body: ");

                let msg = BusMessage::notification(&title, &body);
                state.get_message_bus().publish(msg).await?;
                println!("✅ Sent: Notification\n");
            }
            "7" => {
                // Server command (server broadcast via publish)
                let command = get_input("Command (config_update/sync_dishes/restart): ");
                let key = get_input("Key (optional): ");

                let msg = BusMessage::server_command(
                    &command,
                    if key.is_empty() {
                        serde_json::json!({
                            "command": command,
                            "reason": "demo"
                        })
                    } else {
                        serde_json::json!({
                            "command": command,
                            "key": key,
                            "reason": "demo"
                        })
                    },
                );
                state.get_message_bus().publish(msg).await?;
                println!("✅ Sent: Server command: {}\n", command);
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
                            "order_intent" | "table_intent" => {
                                BusMessage::order_intent(&serde_json::json!({
                                    "action": value["action"].as_str().unwrap_or("custom"),
                                    "table_id": value["table_id"].as_str().unwrap_or("unknown"),
                                    "data": value["data"].clone(),
                                    "operator": value["operator"].as_str().unwrap_or("demo")
                                }))
                            }
                            "order_sync" | "table_sync" => {
                                BusMessage::order_sync(&serde_json::json!({
                                    "action": value["action"].as_str().unwrap_or("custom"),
                                    "table_id": value["table_id"].as_str().unwrap_or("unknown"),
                                    "status": value["status"].as_str().unwrap_or("updated"),
                                    "source": "demo",
                                    "data": value["data"].clone()
                                }))
                            }
                            "data_sync" => BusMessage::data_sync(
                                value["sync_type"].as_str().unwrap_or("custom"),
                                value["data"].clone(),
                            ),
                            "server_command" => BusMessage::server_command(
                                value["command"].as_str().unwrap_or("custom"),
                                value["data"].clone(),
                            ),
                            _ => BusMessage::notification("Custom", &json_str),
                        };
                        // Use send for client messages, publish for server broadcasts
                        if msg_type == "order_intent" || msg_type == "table_intent" {
                            msg_client.send(&msg).await?;
                        } else {
                            state.get_message_bus().publish(msg).await?;
                        }
                        println!("✅ Sent: Custom message\n");
                    }
                    Err(e) => {
                        println!("❌ Invalid JSON: {}\n", e);
                    }
                }
            }
            _ => {
                println!("❌ Invalid choice. Please try again.\n");
            }
        }

        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    }

    Ok(())
}

fn print_menu() {
    println!("📋 Select an action:");
    println!("  1. 📝 Add dish to table");
    println!("  2. 💰 Payment request");
    println!("  3. 🧾 Checkout");
    println!("  4. 💾 Update dish price");
    println!("  5. ❌ Mark dish sold out");
    println!("  6. 📢 System notification");
    println!("  7. 🎮 Server command");
    println!("  8. 🔧 Custom JSON");
    println!("  0. ❌ Exit");
    println!();
}

fn get_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}
