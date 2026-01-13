//! Interactive Message Client Example
//!
//! Demonstrates an interactive MessageClient that can:
//! 1. Subscribe and display messages from the edge server
//! 2. Send messages via interactive menu
//!
//! Run: cargo run --example message_client

use crab_client::{BusMessage, EventType, MessageClient};
use std::io::{self, Write};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    println!("\n🦀 Interactive Message Client");
    println!("================================\n");

    interactive_client("192.168.1.176:8082").await
}

async fn interactive_client(addr: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("📡 Connecting to message bus at {}...", addr);

    let client = MessageClient::connect(addr).await?;

    println!("✅ Connected successfully!\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

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
                let quantity = get_input("Quantity: ").parse::<i32>().unwrap_or(1);

                let msg = BusMessage::order_intent(&shared::message::OrderIntentPayload {
                    action: "add_dish".to_string(),
                    table_id: table_id,
                    order_id: None,
                    data: serde_json::json!({
                        "dishes": [{
                            "name": dish_name,
                            "quantity": quantity
                        }]
                    }),
                    operator: Some("client_user".to_string()),
                });
                client.send(&msg).await?;
            }
            "2" => {
                // Payment request
                let table_id = get_input("Table ID: ");
                let amount = get_input("Amount (cents): ").parse::<i32>().unwrap_or(0);
                let method = get_input("Payment method (cash/card/wechat): ");

                let msg = BusMessage::order_intent(&shared::message::OrderIntentPayload {
                    action: "payment".to_string(),
                    table_id: table_id,
                    order_id: None,
                    data: serde_json::json!({
                        "amount": amount,
                        "method": method
                    }),
                    operator: Some("client_user".to_string()),
                });
                client.send(&msg).await?;
            }
            "3" => {
                // Checkout
                let table_id = get_input("Table ID: ");

                let msg = BusMessage::order_intent(&shared::message::OrderIntentPayload {
                    action: "checkout".to_string(),
                    table_id: table_id,
                    order_id: None,
                    data: serde_json::Value::Null,
                    operator: Some("client_user".to_string()),
                });
                client.send(&msg).await?;
            }
            "4" => {
                // Dish price update
                let dish_id = get_input("Dish ID: ");
                let new_price = get_input("New price (cents): ").parse::<i32>().unwrap_or(0);

                let msg = BusMessage::data_sync(
                    "dish_price",
                    serde_json::json!({
                        "dish_id": dish_id,
                        "new_price": new_price,
                        "updated_by": "client_user"
                    }),
                );
                client.send(&msg).await?;
            }
            "5" => {
                // Dish sold out
                let dish_id = get_input("Dish ID: ");

                let msg = BusMessage::data_sync(
                    "dish_sold_out",
                    serde_json::json!({
                        "dish_id": dish_id,
                        "available": false
                    }),
                );
                client.send(&msg).await?;
            }
            "6" => {
                // System notification
                let title = get_input("Notification title: ");
                let body = get_input("Notification body: ");

                let msg = BusMessage::notification(&title, &body);
                client.send(&msg).await?;
            }
            "7" => {
                // Server command
                let command = get_input("Command (config_update/sync_dishes/restart): ");
                let key = get_input("Key (optional): ");

                let msg = BusMessage::server_command(
                    &command,
                    if key.is_empty() {
                        serde_json::json!({
                            "command": command,
                            "reason": "client"
                        })
                    } else {
                        serde_json::json!({
                            "command": command,
                            "key": key,
                            "reason": "client"
                        })
                    },
                );
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
                                BusMessage::order_intent(&shared::message::OrderIntentPayload {
                                    action: value
                                        .get("action")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("custom")
                                        .to_string(),
                                    table_id: value
                                        .get("table_id")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    order_id: value
                                        .get("order_id")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                    data: value
                                        .get("data")
                                        .cloned()
                                        .unwrap_or(serde_json::Value::Null),
                                    operator: value
                                        .get("operator")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                })
                            }
                            "order_sync" => {
                                BusMessage::order_sync(&shared::message::OrderSyncPayload {
                                    action: value
                                        .get("action")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("custom")
                                        .to_string(),
                                    table_id: value
                                        .get("table_id")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                    order_id: value
                                        .get("order_id")
                                        .and_then(|v| v.as_str())
                                        .map(|s| s.to_string()),
                                    status: value
                                        .get("status")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown")
                                        .to_string(),
                                    source: value
                                        .get("source")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("unknown")
                                        .to_string(),
                                    data: value.get("data").cloned(),
                                })
                            }
                            "data_sync" => BusMessage::data_sync(
                                value
                                    .get("sync_type")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("custom"),
                                value.get("data").cloned().unwrap_or_default(),
                            ),
                            "server_command" => BusMessage::server_command(
                                value
                                    .get("command")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("custom"),
                                value.get("data").cloned().unwrap_or_default(),
                            ),
                            _ => BusMessage::notification("Custom", &json_str),
                        };
                        client.send(&msg).await?;
                    }
                    Err(e) => {
                        println!("❌ Invalid JSON: {}\n", e);
                        continue;
                    }
                }
            }
            _ => {
                println!("❌ Invalid choice. Please try again.\n");
                continue;
            }
        }

        // 用户要求不显示发出的消息，所以这里不打印
        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    }

    Ok(())
}

fn print_menu() {
    println!("📋 请选择操作:");
    println!("  1. 📝 添加菜品");
    println!("  2. 💰 支付请求");
    println!("  3. 🧾 结账");
    println!("  4. 💾 更新菜价");
    println!("  5. ❌ 菜品售罄");
    println!("  6. 📢 系统通知");
    println!("  7. 🎮 服务器指令");
    println!("  8. 🔧 自定义 JSON");
    println!("  0. ❌ 退出");
    println!();
}

fn get_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).unwrap();
    input.trim().to_string()
}

fn print_received_message(msg: &BusMessage) {
    let timestamp = chrono::Local::now().format("%H:%M:%S");

    match msg.event_type {
        EventType::OrderIntent => {
            if let Ok(payload) = msg.parse_payload::<serde_json::Value>() {
                let action = payload
                    .get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let table_id = payload
                    .get("table_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(no table)");

                let details = match action {
                    "add_dish" => {
                        let dishes = payload
                            .get("data")
                            .and_then(|v| v.get("dishes"))
                            .and_then(|v| v.as_array());
                        if let Some(dishes) = dishes {
                            let dish_list: Vec<String> = dishes
                                .iter()
                                .map(|d| {
                                    let name =
                                        d.get("name").and_then(|v| v.as_str()).unwrap_or("?");
                                    let qty =
                                        d.get("quantity").and_then(|v| v.as_i64()).unwrap_or(0);
                                    format!("{}x{}", qty, name)
                                })
                                .collect();
                            format!("桌台: {} | 菜品: {}", table_id, dish_list.join(", "))
                        } else {
                            format!("桌台: {}", table_id)
                        }
                    }
                    "payment" => {
                        let amount = payload
                            .get("data")
                            .and_then(|v| v.get("amount"))
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        let method = payload
                            .get("data")
                            .and_then(|v| v.get("method"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        format!("桌台: {} | 金额: {}分 | 方式: {}", table_id, amount, method)
                    }
                    "checkout" => {
                        format!("桌台: {} | 结算请求", table_id)
                    }
                    _ => format!("桌台: {} | 操作: {}", table_id, action),
                };

                println!("[{}] [收到] 📝 ORDER INTENT | {}", timestamp, details);
            } else {
                println!("[{}] [收到] 📝 ORDER INTENT", timestamp);
            }
        }
        EventType::OrderSync => {
            if let Ok(payload) = msg.parse_payload::<serde_json::Value>() {
                let action = payload
                    .get("action")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let table_id = payload
                    .get("table_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(no table)");
                let status = payload
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(no status)");

                println!(
                    "[{}] [收到] 🔄 ORDER SYNC | 桌台: {} | 状态: {} | 操作: {}",
                    timestamp, table_id, status, action
                );
            } else {
                println!("[{}] [收到] 🔄 ORDER SYNC", timestamp);
            }
        }
        EventType::DataSync => {
            if let Ok(payload) = msg.parse_payload::<serde_json::Value>() {
                let sync_type = payload
                    .get("sync_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");

                match sync_type {
                    "dish_price" => {
                        let dish_id = payload
                            .get("dish_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        let new_price = payload
                            .get("new_price")
                            .and_then(|v| v.as_i64())
                            .unwrap_or(0);
                        println!(
                            "[{}] [收到] 💾 DATA SYNC | 菜品价格 | ID: {} | 新价格: {}分",
                            timestamp, dish_id, new_price
                        );
                    }
                    "dish_sold_out" => {
                        let dish_id = payload
                            .get("dish_id")
                            .and_then(|v| v.as_str())
                            .unwrap_or("?");
                        let available = payload
                            .get("available")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        let status = if available { "有货" } else { "售罄" };
                        println!(
                            "[{}] [收到] 💾 DATA SYNC | 菜品状态 | ID: {} | 状态: {}",
                            timestamp, dish_id, status
                        );
                    }
                    _ => {
                        println!(
                            "[{}] [收到] 💾 DATA SYNC | 类型: {} | {}",
                            timestamp, sync_type, payload
                        );
                    }
                }
            } else {
                println!("[{}] [收到] 💾 DATA SYNC", timestamp);
            }
        }
        EventType::Notification => {
            if let Ok(payload) = msg.parse_payload::<serde_json::Value>() {
                let title = payload
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(no title)");
                let body = payload
                    .get("body")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(no body)");
                println!(
                    "[{}] [收到] 📢 NOTIFICATION | 标题: {} | 内容: {}",
                    timestamp, title, body
                );
            } else {
                println!("[{}] [收到] 📢 NOTIFICATION", timestamp);
            }
        }
        EventType::ServerCommand => {
            if let Ok(payload) = msg.parse_payload::<serde_json::Value>() {
                let command = payload
                    .get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                let reason = payload
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("(no reason)");

                println!(
                    "[{}] [收到] 🎮 SERVER COMMAND | 指令: {} | 原因: {}",
                    timestamp, command, reason
                );
            } else {
                println!("[{}] [收到] 🎮 SERVER COMMAND", timestamp);
            }
        }
    }
}
