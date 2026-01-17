//! Interactive Message Client Example with mTLS support
//!
//! Demonstrates an interactive MessageClient that can:
//! 1. Input client_name and check local credential cache
//! 2. Authenticate with Auth Server to get mTLS certificates (if no cache)
//! 3. Connect to Edge Server using mTLS
//! 4. Send/Receive messages via TUI
//!
//! Run: cargo run --example message_client

use crab_client::{CertManager, Credential, CredentialStorage, MessageClient, NetworkMessageClient};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{prelude::*, widgets::*};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme};
use std::io::{self, Stdout, Write};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

#[derive(Default)]
#[allow(dead_code)]
struct App {
    input: Input,
    input_mode: InputMode,
    client: Option<NetworkMessageClient>,
    username: String,
    connected: bool,
    notifications: Vec<String>,
    notify_rx: Option<mpsc::Receiver<String>>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum InputMode {
    #[default]
    Normal,
    Editing,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Install default crypto provider (ring)
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Initialize TUI Logger with Tracing
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(tui_logger::tracing_subscriber_layer())
        .with(env_filter)
        .init();

    // Also init log crate adapter just in case dependencies use log crate
    tui_logger::init_logger(log::LevelFilter::Info).ok();
    tui_logger::set_default_level(log::LevelFilter::Info);

    // --- CLI Phase (Startup Wizard) ---
    println!("\n🦀 Interactive Message Client (mTLS)");
    println!("=====================================");
    println!("\n⚠️  Prerequisites:");
    println!("   1. Start Auth Server:  cargo run -p crab-auth");
    println!("   2. Start Edge Server:  cargo run -p edge-server");
    println!();

    // 1. Input client_name
    let cert_path: PathBuf = std::env::var("CRAB_CERT_PATH")
        .unwrap_or_else(|_| "./certs".to_string())
        .into();
    let default_client_name = "message-client".to_string();
    let client_name = get_input_with_default("Client Name", &default_client_name);

    println!("\n📁 Checking credential cache for '{}'...", client_name);

    // 2. Check local credential cache
    let cert_manager = CertManager::new(&cert_path, &client_name);
    let mut username = String::new();
    let mut token = String::new();
    let mut tenant_id = String::new();

    if cert_manager.has_credential() {
        println!("✅ Found cached credential!");

        // Load and display cached info
        let client_cert_path = cert_path.join(&client_name);
        let storage = CredentialStorage::new(&client_cert_path, "credential.json");
        if let Some(cred) = storage.load() {
            println!("   Client: {}", cred.client_name);
            println!(
                "   Token: {}...",
                &cred.token[..std::cmp::min(20, cred.token.len())]
            );

            // Verify not expired (if has expiry)
            if cred.expires_at.is_some() {
                println!("   Has expiry date");
            }

            // Use cached token
            token = cred.token;
            tenant_id = cred.tenant_id;
            username = cred.client_name.clone();

            let use_cache = get_input_with_default("Use cached credential? (y/n)", "y");
            if use_cache.to_lowercase() != "y" {
                println!("Clearing cache and requiring re-authentication...");
                cert_manager.logout()?;
                token.clear();
            }
        }
    } else {
        println!("📭 No cached credential found");
    }

    let cert_url: String = "http://localhost:3001".into();
    // 3. If no cached token, require authentication
    if token.is_empty() {
        // Authenticate
        println!("\n🔑 Authentication required");
        username = get_input("Username: ");
        let password = get_input("Password: ");

        println!("Connecting to Auth Server...");
        let http_client = reqwest::Client::new();

        let login_res = http_client
            .post(format!("{}/api/auth/login", cert_url))
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
        token = login_data["token"]
            .as_str()
            .ok_or("No token in login response")?
            .to_string();
        tenant_id = login_data["tenant_id"]
            .as_str()
            .ok_or("No tenant_id in login response")?
            .to_string();

        println!("✅ Login successful! Tenant: {}", tenant_id);

        // Save credential to cache
        println!("💾 Saving credential to cache...");
        let client_cert_path = cert_path.join(&client_name);
        let storage = CredentialStorage::new(&client_cert_path, "credential.json");
        let cred = Credential {
            client_name: client_name.clone(),
            token: token.clone(),
            expires_at: None,
            tenant_id: tenant_id.clone(),
        };
        storage.save(&cred).ok();
        println!("✅ Credential cached at: {:?}", storage.path());
    }

    let edge_addr = get_input_with_default("Edge Server Address", "127.0.0.1:8081");
    // Note: Edge Server cert includes "localhost" and "edge-server-{uuid}" as valid DNS names

    // 4. Get certificates (from cache or request new)
    println!("\n📜 Checking for local certificates...");

    let (cert_pem, key_pem, tenant_ca_pem) = if cert_manager.has_local_certificates() {
        println!("✅ Found local certificates!");
        cert_manager.load_local_certificates()?
    } else {
        println!("📭 No local certificates found, requesting from Auth Server...");

        let device_id = crab_cert::generate_hardware_id();
        println!("Using Device ID: {}", device_id);

        let common_name = client_name.clone();
        println!("Requesting cert for: {}", common_name);

        let http_client = reqwest::Client::new();

        println!("Requesting certificate from Auth Server...");

        let issue_res = http_client
            .post(format!("{}/api/cert/issue", cert_url))
            .header("Authorization", format!("Bearer {}", token))
            .json(&serde_json::json!({
                "tenant_id": tenant_id,
                "common_name": &common_name,
                "is_server": false,
                "device_id": device_id
            }))
            .send()
            .await?;

        let status = issue_res.status();
        if !status.is_success() {
            let error_text = issue_res.text().await;
            return Err(format!(
                "Cert issuance failed (HTTP {}): {}\n\n\
                Make sure Auth Server is running on {}",
                status,
                error_text.unwrap_or_else(|_| "Unknown error".to_string()),
                cert_url
            ).into());
        }

        let cert_data: serde_json::Value = issue_res.json().await?;

        // Check for error response
        if let Some(error) = cert_data.get("error") {
            return Err(format!("Cert issuance failed: {}", error).into());
        }

        let cert_pem = cert_data["cert"].as_str()
            .ok_or("No cert received - Auth Server may need activation")?
            .to_string();
        let key_pem = cert_data["key"].as_str()
            .ok_or("No key received")?
            .to_string();
        let tenant_ca_pem = cert_data["tenant_ca_cert"]
            .as_str()
            .ok_or("No tenant CA received")?
            .to_string();

        println!("✅ Certificate received!");

        // Save certificates to local storage
        println!("💾 Saving certificates to local storage...");
        cert_manager.save_certificates(&cert_pem, &key_pem, &tenant_ca_pem)?;

        (cert_pem, key_pem, tenant_ca_pem)
    };

    // Clone for multiple uses (TLS config and connection)
    let cert_pem_clone = cert_pem.clone();
    let key_pem_clone = key_pem.clone();
    let tenant_ca_pem_clone = tenant_ca_pem.clone();

    // 5. Verify device_id in certificate (hardware binding)
    println!("🔍 Verifying certificate hardware binding...");
    let current_device_id = crab_cert::generate_hardware_id();
    let cert_metadata = crab_cert::CertMetadata::from_pem(&cert_pem_clone)
        .map_err(|e| format!("Failed to parse certificate metadata: {}", e))?;

    if let Some(cert_device_id) = &cert_metadata.device_id {
        if cert_device_id != &current_device_id {
            println!("⚠️  Certificate device_id mismatch!");
            println!("   Certificate bound to: {}", cert_device_id);
            println!("   Current device:       {}", current_device_id);
            println!("   Certificate may have been copied from another machine.");
            let force = get_input_with_default("Continue anyway?", "n");
            if force.to_lowercase() != "y" {
                return Err("Certificate device_id mismatch".into());
            }
        } else {
            println!("✅ Device binding verified: {}", current_device_id);
        }
    } else {
        println!("⚠️  Certificate has no device_id binding (may be legacy)");
    }

    // 5. Configure mTLS
    println!("\n🔐 Configuring mTLS...");

    let mut cert_reader = std::io::Cursor::new(cert_pem_clone.clone());
    let certs: Vec<CertificateDer> =
        rustls_pemfile::certs(&mut cert_reader).collect::<Result<_, _>>()?;
    let cert = certs.into_iter().next().ok_or("No cert found")?;

    let mut key_reader = std::io::Cursor::new(key_pem_clone.clone());
    let key = rustls_pemfile::private_key(&mut key_reader)?.ok_or("No private key found")?;

    let mut root_store = RootCertStore::empty();
    let mut ca_reader = std::io::Cursor::new(tenant_ca_pem_clone);
    for cert in rustls_pemfile::certs(&mut ca_reader) {
        root_store.add(cert?)?;
    }

    #[derive(Debug)]
    struct SkipHostnameVerifier;
    impl ServerCertVerifier for SkipHostnameVerifier {
        fn verify_server_cert(
            &self,
            _end_entity: &CertificateDer<'_>,
            _intermediates: &[CertificateDer<'_>],
            _server_name: &ServerName<'_>,
            _ocsp_response: &[u8],
            _now: UnixTime,
        ) -> Result<ServerCertVerified, rustls::Error> {
            Ok(ServerCertVerified::assertion())
        }

        fn verify_tls12_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, rustls::Error> {
            Ok(HandshakeSignatureValid::assertion())
        }

        fn verify_tls13_signature(
            &self,
            _message: &[u8],
            _cert: &CertificateDer<'_>,
            _dss: &DigitallySignedStruct,
        ) -> Result<HandshakeSignatureValid, rustls::Error> {
            Ok(HandshakeSignatureValid::assertion())
        }

        fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
            vec![
                SignatureScheme::RSA_PSS_SHA256,
                SignatureScheme::RSA_PKCS1_SHA256,
                SignatureScheme::ECDSA_NISTP256_SHA256,
            ]
        }
    }

    let _tls_config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipHostnameVerifier))
        .with_client_auth_cert(vec![cert], key)?;

    println!("✅ mTLS configured.");

    // 6. Connect to Edge Server using mTLS
    println!("\n🔌 Connecting to Edge Server...");

    // Pass PEM bytes directly to connect_mtls (it handles parsing internally)
    let ca_cert_pem = tenant_ca_pem.as_bytes().to_vec();
    let client_cert_pem = cert_pem.as_bytes().to_vec();
    let client_key_pem = key_pem.as_bytes().to_vec();

    // Connect using mTLS
    let is_connected = match NetworkMessageClient::connect_mtls(
        &edge_addr,
        &ca_cert_pem,
        &client_cert_pem,
        &client_key_pem,
        &client_name,
    ).await {
        Ok(client) => {
            println!("✅ Connected to Edge Server via mTLS!");
            Some(client)
        }
        Err(e) => {
            println!("❌ Connection failed: {}", e);
            println!("   Make sure the Edge Server is running with mTLS enabled.");
            None
        }
    };

    let connected = is_connected.is_some();

    if connected {
        // 确保用户看到连接成功的消息
        println!("\n✅ Connected to Edge Server via mTLS!");
        println!("\nStarting TUI... (press 'e' to edit, 'q' to quit)");
    } else {
        println!("\n❌ Connection failed. Exiting.");
        return Ok(());
    }

    // 只在连接成功时启动 TUI
    if let Some(client) = is_connected {
        if let Err(e) = run_tui(client, username.clone()).await {
            // TUI 失败时回退到 CLI 模式
            eprintln!("TUI 不可用 ({}), 使用 CLI 模式", e);
            run_cli(username).await;
        }
    }

    println!("Goodbye!");
    Ok(())
}

async fn run_tui(
    client: NetworkMessageClient,
    username: String,
) -> io::Result<()> {
    use ratatui::backend::CrosstermBackend as TuiBackend;
    use ratatui::Terminal;

    // 初始化 TUI
    let stdout = io::stdout();
    let backend = TuiBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // 启用原始模式
    enable_raw_mode()?;

    // 创建消息 channel 用于接收通知
    let (notify_tx, notify_rx) = mpsc::channel(32);

    // 启动后台任务接收服务端通知
    let client_clone = client.clone();
    let notify_tx_clone = notify_tx.clone();
    tokio::spawn(async move {
        loop {
            match client_clone.recv().await {
                Ok(msg) => {
                    if msg.event_type == shared::message::EventType::Notification {
                        if let Ok(payload) = msg.parse_payload::<shared::message::NotificationPayload>() {
                            let notification = format!(
                                "[{:?}] {}: {}",
                                payload.category,
                                payload.title,
                                payload.message
                            );
                            let _ = notify_tx_clone.send(notification).await;
                        }
                    }
                }
                Err(_) => break,
            }
        }
    });

    // 创建 App
    let mut app = App {
        input: Input::default(),
        input_mode: InputMode::Normal,
        client: Some(client),
        username,
        connected: true,
        notifications: Vec::new(),
        notify_rx: Some(notify_rx),
    };

    // 运行 TUI
    let result = run_app(&mut terminal, &mut app).await;

    // 恢复原始模式
    disable_raw_mode()?;

    // 恢复标准输出
    execute!(terminal.backend_mut(), LeaveAlternateScreen).ok();

    result
}

/// CLI fallback mode when TUI is not available
async fn run_cli(username: String) {
    println!("\n✅ Connected to Edge Server via mTLS!");
    println!("Username: {}", username);
    println!("\nCommands:");
    println!("  /help      - Show help");
    println!("  /status    - Show connection status");
    println!("  /req <cmd> - Send request command");
    println!("  /ping      - Ping server");
    println!("  /quit      - Quit");
    println!("");

    let mut input = String::new();
    loop {
        print!("> ");
        io::stdout().flush().ok();

        match io::stdin().read_line(&mut input) {
            Ok(0) | Err(_) => break,
            Ok(_) => {
                let cmd = input.trim();
                if cmd.is_empty() {
                    input.clear();
                    continue;
                }
                if cmd == "/quit" || cmd == "q" {
                    break;
                }
                println!("[INFO] Command: {}", cmd);
                // 这里只是打印，实际命令处理需要在 TUI 中
            }
        }
        input.clear();
    }
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        // 非阻塞接收通知
        if let Some(ref mut rx) = app.notify_rx {
            while let Ok(notification) = rx.try_recv() {
                app.notifications.push(notification);
                // 保留最多 100 条通知
                if app.notifications.len() > 100 {
                    app.notifications.remove(0);
                }
            }
        }

        // 快速检查是否有事件，非阻塞
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc if matches!(app.input_mode, InputMode::Normal) => break,
                        KeyCode::Char('e') if matches!(app.input_mode, InputMode::Normal) => {
                            app.input_mode = InputMode::Editing;
                        }
                        KeyCode::Esc if matches!(app.input_mode, InputMode::Editing) => {
                            app.input.reset();
                            app.input_mode = InputMode::Normal;
                        }
                        KeyCode::Enter if matches!(app.input_mode, InputMode::Editing) => {
                            let input_str: String = app.input.value().into();
                            if !input_str.is_empty() {
                                handle_command(app, &input_str).await;
                            }
                            app.input.reset();
                            app.input_mode = InputMode::Normal;
                        }
                        _ if matches!(app.input_mode, InputMode::Editing) => {
                            app.input.handle_event(&Event::Key(key));
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    Ok(())
}

async fn handle_command(app: &mut App, cmd: &str) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }

    match parts[0] {
        "/help" => {
            tracing::info!("Commands:");
            tracing::info!("  /quit                  - Exit");
            tracing::info!("  /status                - Show connection status");
            tracing::info!("  /req <command> [args]  - Send request command");
            tracing::info!("  /ping                  - Ping server");
        }
        "/quit" => {
            tracing::info!("Press Esc then 'q' to quit");
        }
        "/status" => {
            tracing::info!(
                "Connection status: {}",
                if app.connected {
                    "Connected"
                } else {
                    "Disconnected"
                }
            );
        }
        "/req" => {
            if parts.len() < 2 {
                tracing::warn!("Usage: /req <command> [args]");
                return;
            }
            let command = parts[1];
            let args: Vec<&str> = parts[2..].to_vec();
            send_request(app, command, &args).await;
        }
        "/ping" => {
            send_request(app, "ping", &[]).await;
        }
        _ => {
            tracing::warn!(
                "Unknown command: {}. Type /help for available commands.",
                parts[0]
            );
        }
    }
}

async fn send_request(app: &mut App, command: &str, args: &[&str]) {
    let Some(ref client) = app.client else {
        tracing::error!("Not connected to server");
        return;
    };

    let params = if args.is_empty() {
        None
    } else {
        Some(serde_json::json!({ "args": args }))
    };

    let request = shared::message::RequestCommandPayload {
        action: command.to_string(),
        params,
    };

    let msg = shared::message::BusMessage::request_command(&request);

    match client.send(&msg).await {
        Ok(()) => {
            tracing::info!("Request sent: {}", command);
            // 等待响应
            match client.recv().await {
                Ok(response) => {
                    if let Ok(payload) = response.parse_payload::<shared::message::ResponsePayload>() {
                        if payload.success {
                            tracing::info!("Response: {}", payload.message);
                        } else {
                            tracing::error!("Error: {}", payload.message);
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to receive response: {:?}", e);
                }
            }
        }
        Err(e) => {
            tracing::error!("Failed to send request: {:?}", e);
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(3),
        ])
        .split(f.area());

    let title = Paragraph::new(vec![Line::from(vec![
        Span::raw(" 🦀 Crab Message Client "),
        Span::styled(
            if app.connected {
                " Connected "
            } else {
                " Disconnected "
            },
            Style::default().fg(Color::Green),
        ),
        Span::raw(format!(" [{}]", app.username)),
    ])])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    f.render_widget(title, chunks[0]);

    // 显示通知列表
    let notifications: Vec<Line> = app
        .notifications
        .iter()
        .rev()
        .take(20)
        .map(|n| {
            Line::from(vec![Span::styled(
                n,
                Style::default().fg(Color::Yellow),
            )])
        })
        .collect();

    let notification_list = if notifications.is_empty() {
        vec![Line::from(vec![Span::styled(
            " No notifications yet... ",
            Style::default().fg(Color::DarkGray),
        )])]
    } else {
        notifications
    };

    let notify_area = Paragraph::new(notification_list)
        .block(
            Block::default()
                .title(" Notifications ")
                .border_style(
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::DIM),
                )
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(notify_area, chunks[1]);

    let input_block = Block::default()
        .borders(Borders::ALL)
        .title(" Command Input (Type /help) ");

    let style = match app.input_mode {
        InputMode::Normal => Style::default().fg(Color::Gray),
        InputMode::Editing => Style::default().fg(Color::Yellow),
    };

    let width = chunks[2].width.max(3) - 3;
    let scroll = app.input.visual_scroll(width as usize);
    let input = Paragraph::new(app.input.value())
        .style(style)
        .scroll((0, scroll as u16))
        .block(input_block);
    f.render_widget(input, chunks[2]);

    if app.input_mode == InputMode::Editing {
        f.set_cursor_position((
            chunks[2].x + ((app.input.visual_cursor().max(scroll) - scroll) as u16) + 1,
            chunks[2].y + 1,
        ));
    }

    if app.input_mode == InputMode::Normal {
        let help_text = Paragraph::new("Press 'e' to edit, 'q' to quit")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Right);
        f.render_widget(help_text, chunks[0]);
    }
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
