//! Interactive Message Client Example with mTLS support (TUI Enhanced)
//!
//! Demonstrates an interactive MessageClient that can:
//! 1. Authenticate with Auth Server to get mTLS certificates
//! 2. Connect to Edge Server using mTLS
//! 3. Send/Receive messages via TUI
//!
//! Run: cargo run --example message_client

use crab_client::MessageClient;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{prelude::*, widgets::*};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme};
use shared::message::BusMessage;
use std::io::{self, Stdout, Write};
use std::sync::Arc;
use std::time::Duration;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerWidget};

#[derive(Default)]
struct App {
    input: Input,
    input_mode: InputMode,
    client: Option<MessageClient>,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
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
    // We use println! here which bypasses the tui-logger (which captures tracing/log macros)
    // So the user sees these prompts in the standard terminal before TUI starts.

    println!("\n🦀 Interactive Message Client (mTLS)");
    println!("=====================================\n");

    let auth_url = get_input_with_default("Auth Server URL", "http://localhost:3001");
    let edge_addr = get_input_with_default("Edge Server Address", "127.0.0.1:8081");

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
    let tenant_id = login_data["tenant_id"]
        .as_str()
        .ok_or("No tenant_id in login response")?
        .to_string();

    println!(
        "✅ Login successful! Token received for Tenant: {}",
        tenant_id
    );

    // 2. Request Certificate
    println!("\n📜 Requesting Client Certificate...");

    // Auto-detect Device ID
    let device_id = crab_cert::generate_hardware_id();
    println!("Using Device ID: {}", device_id);

    // Custom Client Name (Common Name)
    let default_common_name = format!("client-{}", username);
    let common_name = get_input_with_default("Client Name (Common Name)", &default_common_name);
    println!("Requesting cert for: {}", common_name);

    let issue_res = http_client
        .post(format!("{}/api/cert/issue", auth_url))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({
            "tenant_id": tenant_id,
            "common_name": &common_name,
            "is_server": false,
            "hardware_id": device_id
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
    let cert = certs.into_iter().next().ok_or("No cert found")?;

    let mut key_reader = std::io::Cursor::new(key_pem);
    let key = rustls_pemfile::private_key(&mut key_reader)?.ok_or("No private key found")?;

    // Load Tenant CA
    let mut root_store = RootCertStore::empty();
    let mut ca_reader = std::io::Cursor::new(tenant_ca_pem);
    for cert in rustls_pemfile::certs(&mut ca_reader) {
        root_store.add(cert?)?;
    }

    // Custom verifier to skip hostname verification for localhost demo
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

    // Build TLS config
    // We need to use dangerous() builder to set custom verifier
    let tls_config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(SkipHostnameVerifier))
        .with_client_auth_cert(vec![cert], key)?;

    println!("✅ mTLS configured.");
    println!("🚀 Connecting to Edge Server at {}...", edge_addr);

    // 4. Connect
    let domain = "localhost"; // Matches cert CN usually, but we skip verify
    let client = MessageClient::connect_tls(&edge_addr, domain, tls_config, &common_name).await?;
    println!("✅ Connected successfully!");

    // Wait a moment for user to see success
    std::thread::sleep(std::time::Duration::from_secs(1));

    // --- TUI Phase ---
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::default();
    app.client = Some(client);

    tracing::info!("✅ Client Ready. Type /help for commands.");

    // Start background receiver for TUI logging
    let client_clone = app.client.as_ref().unwrap().clone();
    tokio::spawn(async move {
        loop {
            match client_clone.recv().await {
                Ok(msg) => {
                    tracing::info!("📨 [RECV] {:?}", msg.event_type);
                    if let Ok(payload) = msg.parse_payload::<serde_json::Value>() {
                        tracing::info!("   Data: {}", payload);
                    }
                }
                Err(e) => {
                    tracing::error!("❌ Disconnected/Error: {}", e);
                    // Simple retry delay or break
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
            }
        }
    });

    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.input_mode {
                        InputMode::Normal => match key.code {
                            KeyCode::Char('e') => {
                                app.input_mode = InputMode::Editing;
                            }
                            KeyCode::Char('q') | KeyCode::Esc => {
                                if let Some(client) = &app.client {
                                    let _ = client.close().await;
                                }
                                return Ok(());
                            }
                            _ => {}
                        },
                        InputMode::Editing => match key.code {
                            KeyCode::Enter => {
                                let input_str: String = app.input.value().into();
                                if !input_str.is_empty() {
                                    handle_command(app, &input_str).await;
                                    app.input.reset();
                                }
                            }
                            KeyCode::Esc => {
                                app.input_mode = InputMode::Normal;
                            }
                            _ => {
                                app.input.handle_event(&Event::Key(key));
                            }
                        },
                    }
                }
            }
        }
    }
}

async fn handle_command(app: &mut App, cmd: &str) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }

    match parts[0] {
        "/help" => {
            tracing::info!("Commands:");
            tracing::info!("  /notify <title> <msg>  - Send notification");
            tracing::info!("  /req <cmd> [args]      - Send request command");
            tracing::info!("  /ping                  - Send ping (as ServerCommand)");
            tracing::info!("  /quit                  - Exit");
        }
        "/quit" => {
            tracing::warn!("Press Esc then 'q' to quit");
            if let Some(client) = &app.client {
                let _ = client.close().await;
            }
        }
        "/notify" => {
            if parts.len() < 3 {
                tracing::error!("Usage: /notify <title> <message>");
                return;
            }
            let title = parts[1];
            let msg_content = parts[2..].join(" ");

            if let Some(client) = &app.client {
                let payload = shared::message::NotificationPayload::info(title, &msg_content);
                let msg = BusMessage::notification(&payload);
                if let Err(e) = client.send(&msg).await {
                    tracing::error!("Failed to send: {}", e);
                } else {
                    tracing::info!("✅ Sent Notification: {} - {}", title, msg_content);
                }
            }
        }
        "/req" => {
            if parts.len() < 2 {
                tracing::error!("Usage: /req <command> [args_json]");
                return;
            }
            let command = parts[1];
            let args_str = if parts.len() > 2 {
                parts[2..].join(" ")
            } else {
                "{}".to_string()
            };
            let args: serde_json::Value =
                serde_json::from_str(&args_str).unwrap_or(serde_json::json!({}));

            if let Some(client) = &app.client {
                let payload = shared::message::RequestCommandPayload {
                    action: command.to_string(),
                    params: Some(args),
                };
                let msg = BusMessage::request_command(&payload);
                if let Err(e) = client.send(&msg).await {
                    tracing::error!("Failed to send Request: {}", e);
                } else {
                    tracing::info!("✅ Sent Request: {}", command);
                }
            }
        }
        "/ping" => {
            if let Some(client) = &app.client {
                // Ping should be a RequestCommand so server can reply
                let payload = shared::message::RequestCommandPayload {
                    action: "ping".to_string(),
                    params: None,
                };
                let msg = BusMessage::request_command(&payload);
                if let Err(e) = client.send(&msg).await {
                    tracing::error!("Failed to send Ping: {}", e);
                } else {
                    tracing::info!("✅ Sent Ping Request");
                }
            }
        }
        _ => {
            tracing::warn!("Unknown command: {}", parts[0]);
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(1),    // Logs
            Constraint::Length(3), // Input
        ])
        .split(f.area());

    // Header
    let title = Paragraph::new(vec![Line::from(vec![
        Span::raw(" 🦀 Crab Message Client "),
        Span::styled(" mTLS Secured ", Style::default().fg(Color::Green)),
    ])])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    f.render_widget(title, chunks[0]);

    // Logs
    let tui_sm = TuiLoggerWidget::default()
        .block(
            Block::default()
                .title(" Messages ")
                .border_style(
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::DIM),
                )
                .borders(Borders::ALL),
        )
        .output_separator('|')
        .output_timestamp(Some("%H:%M:%S".to_string()))
        .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
        .output_target(false)
        .output_file(false)
        .output_line(false)
        .style(Style::default().fg(Color::White));
    f.render_widget(tui_sm, chunks[1]);

    // Input
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

    // Cursor
    if app.input_mode == InputMode::Editing {
        f.set_cursor_position((
            chunks[2].x + ((app.input.visual_cursor().max(scroll) - scroll) as u16) + 1,
            chunks[2].y + 1,
        ));
    }

    // Help
    if app.input_mode == InputMode::Normal {
        let help_text = Paragraph::new("Press 'e' to edit, 'q' to quit")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Right);
        f.render_widget(help_text, chunks[0]);
    }
}

// Helper functions for CLI input
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
