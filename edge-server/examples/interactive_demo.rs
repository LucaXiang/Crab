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
    let mut config = Config::with_overrides(temp_dir, 3000, 8081);
    config.environment = "development".to_string();
    config.jwt.secret = "demo-secret".to_string();

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
                let quantity = get_input("Quantity: ").parse::<u32>().unwrap_or(1);

                let payload = shared::message::OrderIntentPayload::add_dish(
                    shared::message::TableId::new_unchecked(table_id.clone()),
                    vec![shared::message::DishItem::simple(&dish_name, quantity)],
                    Some(shared::message::OperatorId::new("demo_user")),
                );
                let msg = BusMessage::order_intent(&payload);
                msg_client.send(&msg).await?;
                println!("✅ Sent: Add dish to {}\n", table_id);
            }
            "2" => {
                // Payment request (client message via send)
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
                    shared::message::OrderId::new_unchecked("ORD_DEMO"),
                    payment_method,
                    Some(shared::message::OperatorId::new("demo_user")),
                );
                let msg = BusMessage::order_intent(&payload);
                msg_client.send(&msg).await?;
                println!("✅ Sent: Payment for {}\n", table_id);
            }
            "3" => {
                // Checkout (client message via send)
                let table_id = get_input("Table ID: ");

                let payload = shared::message::OrderIntentPayload::checkout(
                    shared::message::TableId::new_unchecked(table_id.clone()),
                    shared::message::OrderId::new_unchecked("ORD_DEMO"),
                    shared::message::PaymentMethod::Cash,
                    Some(shared::message::OperatorId::new("demo_user")),
                );
                let msg = BusMessage::order_intent(&payload);
                msg_client.send(&msg).await?;
                println!("✅ Sent: Checkout for {}\n", table_id);
            }
            "4" => {
                // Dish price update (server broadcast via publish)
                let dish_id = get_input("Dish ID: ");
                let new_price = get_input("New price (cents): ").parse::<u64>().unwrap_or(0);

                let payload = shared::message::DataSyncPayload::DishPrice {
                    dish_id: shared::message::DishId::new(dish_id.clone()),
                    old_price: 0,
                    new_price,
                };
                let msg = BusMessage::data_sync(&payload);
                state.get_message_bus().publish(msg).await?;
                println!("✅ Sent: Price update for {}\n", dish_id);
            }
            "5" => {
                // Dish sold out (server broadcast via publish)
                let dish_id = get_input("Dish ID: ");

                let payload = shared::message::DataSyncPayload::DishSoldOut {
                    dish_id: shared::message::DishId::new(dish_id.clone()),
                    available: false,
                };
                let msg = BusMessage::data_sync(&payload);
                state.get_message_bus().publish(msg).await?;
                println!("✅ Sent: Marked {} as sold out\n", dish_id);
            }
            "6" => {
                // System notification (server broadcast via publish)
                let title = get_input("Notification title: ");
                let body = get_input("Notification body: ");

                let payload = shared::message::NotificationPayload::info(title, body);
                let msg = BusMessage::notification(&payload);
                state.get_message_bus().publish(msg).await?;
                println!("✅ Sent: Notification\n");
            }
            "7" => {
                // Server command (server broadcast via publish)
                let command_str = get_input("Command (ping/config_update/restart): ");
                let key = get_input("Key (for config_update, optional): ");

                let command = match command_str.to_lowercase().as_str() {
                    "ping" => shared::message::ServerCommand::Ping,
                    "restart" => shared::message::ServerCommand::Restart {
                        delay_seconds: 5,
                        reason: Some("demo".to_string()),
                    },
                    "config_update" | _ => shared::message::ServerCommand::ConfigUpdate {
                        key: if key.is_empty() {
                            "demo.key".to_string()
                        } else {
                            key
                        },
                        value: serde_json::json!("demo_value"),
                    },
                };

                let payload = shared::message::ServerCommandPayload { command };
                let msg = BusMessage::server_command(&payload);
                state.get_message_bus().publish(msg).await?;
                println!("✅ Sent: Server command: {}\n", command_str);
            }
            "8" => {
                // Custom notification (simplified)
                println!("Send custom notification:");
                let title = get_input("Title: ");
                let message = get_input("Message: ");
                let level = get_input("Level (info/warning/error): ");

                let notification_level = match level.to_lowercase().as_str() {
                    "warning" => shared::message::NotificationLevel::Warning,
                    "error" => shared::message::NotificationLevel::Error,
                    _ => shared::message::NotificationLevel::Info,
                };

                let payload = shared::message::NotificationPayload {
                    title,
                    message,
                    level: notification_level,
                    category: shared::message::NotificationCategory::System,
                    data: None,
                };

                let msg = BusMessage::notification(&payload);
                state.get_message_bus().publish(msg).await?;
                println!("✅ Sent: Custom notification\n");
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
