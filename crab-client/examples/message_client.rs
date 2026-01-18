//! Interactive Message Client - TUI 可视化客户端
//!
//! 功能:
//! - 显示连接状态、凭证信息、时钟检测状态
//! - Message Bus RPC 通信 (ping, status, echo)
//! - HTTP API 请求 (health, me)
//! - 实时显示服务器广播消息
//!
//! 运行前请确保：
//! 1. 启动 Auth Server: cargo run -p crab-auth
//! 2. 启动 Edge Server: cargo run --example interactive_demo -p edge-server
//!
//! 运行: cargo run -p crab-client --example message_client

use crab_client::{
    Authenticated, BusMessage, CertManager, Connected, CrabClient, NetworkMessageClient, Remote,
};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{prelude::*, widgets::*};
use std::io::{self, Stdout};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerWidget, TuiWidgetEvent, TuiWidgetState};

/// 客户端状态
#[derive(Default, Clone)]
struct ClientStatus {
    // 连接状态
    is_connected: bool,
    server_addr: String,

    // 凭证信息
    client_name: String,
    tenant_id: String,
    device_id: String,
    has_credential: bool,
    has_certificates: bool,

    // 时钟检测
    last_verified_at: String,
    clock_status: String,

    // 证书信息
    cert_expires_at: String,

    // Token 状态
    has_token: bool,
    token_preview: String,

    // RPC 统计
    rpc_count: u32,
    last_rpc_result: String,
}

struct App {
    input: Input,
    input_mode: InputMode,
    status: ClientStatus,
    logger_state: TuiWidgetState,
    /// 当前阶段
    phase: ClientPhase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ClientPhase {
    #[default]
    Disconnected,
    Connected,
    Authenticated,
}

impl Default for App {
    fn default() -> Self {
        Self {
            input: Input::default(),
            input_mode: InputMode::default(),
            status: ClientStatus::default(),
            logger_state: TuiWidgetState::new(),
            phase: ClientPhase::default(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    #[default]
    Normal,
    Editing,
}

// 配置常量
const AUTH_SERVER: &str = "http://127.0.0.1:3001";
const EDGE_HTTPS: &str = "https://127.0.0.1:3000";
const CERT_PATH: &str = "./certs";
const CLIENT_NAME: &str = "tui-client";
const MESSAGE_ADDR: &str = "127.0.0.1:8081";

/// 客户端状态机 - 持有不同阶段的客户端
enum ClientState {
    Disconnected,
    Connected(CrabClient<Remote, Connected>),
    Authenticated(CrabClient<Remote, Authenticated>),
}

impl Default for ClientState {
    fn default() -> Self {
        Self::Disconnected
    }
}

/// 启动通知监听器
///
/// 使用 NetworkMessageClient::subscribe() 订阅非响应消息
fn spawn_notification_listener(client: &NetworkMessageClient) {
    let mut rx = client.subscribe();
    tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    // 根据消息类型显示不同的日志
                    match msg.event_type {
                        shared::EventType::Notification => {
                            if let Ok(payload) =
                                msg.parse_payload::<shared::message::NotificationPayload>()
                            {
                                tracing::info!("📢 [{:?}] {}", payload.level, payload.message);
                                if let Some(data) = payload.data {
                                    tracing::debug!("   Data: {}", data);
                                }
                            } else {
                                tracing::info!("📢 Notification: {:?}", msg.payload);
                            }
                        }
                        shared::EventType::Sync => {
                            tracing::info!("🔄 Sync signal received");
                        }
                        shared::EventType::ServerCommand => {
                            tracing::info!("⚡ Server command: {:?}", msg.payload);
                        }
                        _ => {
                            tracing::debug!("📨 [{}] {:?}", msg.event_type, msg.payload);
                        }
                    }
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("Notification listener lagged, skipped {} messages", n);
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    tracing::debug!("Notification channel closed");
                    break;
                }
            }
        }
    });
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize TUI Logger
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(tui_logger::tracing_subscriber_layer())
        .with(env_filter)
        .init();

    tui_logger::init_logger(log::LevelFilter::Info).ok();
    tui_logger::set_default_level(log::LevelFilter::Info);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create App
    let mut app = App::default();

    // 初始化状态
    app.status.server_addr = MESSAGE_ADDR.to_string();
    app.status.client_name = CLIENT_NAME.to_string();

    // 检查本地凭证
    let cert_manager = CertManager::new(CERT_PATH, CLIENT_NAME);
    update_credential_status(&mut app.status, &cert_manager);

    tracing::info!("Message Client TUI started");
    tracing::info!("Press 'e' to enter command mode, 'q' to quit");
    tracing::info!("Type /help for available commands");

    // Run TUI loop
    let res = run_app(&mut terminal, &mut app).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

fn update_credential_status(status: &mut ClientStatus, cert_manager: &CertManager) {
    status.has_certificates = cert_manager.has_local_certificates();
    status.has_credential = cert_manager.has_credential();

    if let Ok(cred) = cert_manager.load_credential() {
        status.tenant_id = cred.tenant_id.clone();
        status.device_id = cred.device_id.clone().unwrap_or_else(|| "-".to_string());

        // 时钟检测状态
        if let Some(ts) = cred.last_verified_at {
            status.last_verified_at = format_timestamp(ts);
            status.clock_status = match cred.check_clock_tampering() {
                Ok(()) => "OK".to_string(),
                Err(e) => format!("WARN: {}", e),
            };
        } else {
            status.last_verified_at = "-".to_string();
            status.clock_status = "Not verified".to_string();
        }
    } else {
        status.tenant_id = "-".to_string();
        status.device_id = "-".to_string();
        status.last_verified_at = "-".to_string();
        status.clock_status = "-".to_string();
    }

    // 证书信息
    if status.has_certificates {
        if let Ok((cert_pem, _, _)) = cert_manager.load_local_certificates() {
            if let Ok(meta) = crab_cert::CertMetadata::from_pem(&cert_pem) {
                status.cert_expires_at = format!("{}", meta.not_after.date());
            }
        }
    } else {
        status.cert_expires_at = "-".to_string();
    }
}

fn format_timestamp(ts: u64) -> String {
    use chrono::{TimeZone, Utc};
    Utc.timestamp_opt(ts as i64, 0)
        .single()
        .map(|dt| dt.format("%H:%M:%S").to_string())
        .unwrap_or_else(|| ts.to_string())
}

/// 确保客户端处于 Connected 状态，如果未连接则自动重连
async fn ensure_connected(state: &Arc<RwLock<ClientState>>) -> bool {
    let mut write_state = state.write().await;

    // 如果已经是 Connected 或 Authenticated 状态，直接返回
    match &*write_state {
        ClientState::Connected(_) | ClientState::Authenticated(_) => return true,
        ClientState::Disconnected => {}
    }

    tracing::info!("Auto reconnecting...");

    // 重建客户端并重连
    let new_client = match CrabClient::remote()
        .auth_server(AUTH_SERVER)
        .edge_server(EDGE_HTTPS)
        .cert_path(CERT_PATH)
        .client_name(CLIENT_NAME)
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Failed to rebuild client: {}", e);
            *write_state = ClientState::Disconnected;
            return false;
        }
    };

    match new_client.reconnect(MESSAGE_ADDR).await {
        Ok(connected) => {
            tracing::info!("Auto reconnected successfully!");
            *write_state = ClientState::Connected(connected);
            true
        }
        Err(e) => {
            tracing::error!("Auto reconnect failed: {}", e);
            *write_state = ClientState::Disconnected;
            false
        }
    }
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
) -> io::Result<()> {
    let cert_manager = CertManager::new(CERT_PATH, CLIENT_NAME);

    // 客户端状态 - 使用 Arc<RwLock> 以便在异步任务中共享
    let client_state = Arc::new(RwLock::new(ClientState::Disconnected));

    loop {
        terminal.draw(|f| ui(f, app))?;

        let timeout = Duration::from_millis(100);
        if event::poll(timeout)? {
            match event::read()? {
                Event::Key(key) => {
                    if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) {
                        match app.input_mode {
                            InputMode::Normal => match key.code {
                                KeyCode::Char('e') => {
                                    app.input_mode = InputMode::Editing;
                                }
                                KeyCode::Char('q') | KeyCode::Esc => {
                                    return Ok(());
                                }
                                KeyCode::PageUp => {
                                    app.logger_state.transition(TuiWidgetEvent::PrevPageKey)
                                }
                                KeyCode::PageDown => {
                                    app.logger_state.transition(TuiWidgetEvent::NextPageKey)
                                }
                                KeyCode::Up => app.logger_state.transition(TuiWidgetEvent::UpKey),
                                KeyCode::Down => {
                                    app.logger_state.transition(TuiWidgetEvent::DownKey)
                                }
                                _ => {}
                            },
                            InputMode::Editing => match key.code {
                                KeyCode::Enter => {
                                    let input_str: String = app.input.value().into();
                                    if !input_str.is_empty() {
                                        handle_command(
                                            app,
                                            &input_str,
                                            &cert_manager,
                                            &client_state,
                                        )
                                        .await;
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
                _ => {}
            }
        }
    }
}

async fn handle_command(
    app: &mut App,
    cmd: &str,
    cert_manager: &CertManager,
    client_state: &Arc<RwLock<ClientState>>,
) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return;
    }

    match parts[0] {
        "/help" => {
            tracing::info!("=== Connection Commands ===");
            tracing::info!("/setup <user> <pass>  - First-time setup (download certs)");
            tracing::info!("/reconnect            - Reconnect using cached certs");
            tracing::info!("/disconnect           - Disconnect from server");
            tracing::info!("");
            tracing::info!("=== Authentication Commands ===");
            tracing::info!("/login <user> <pass>  - Employee login (get HTTP token)");
            tracing::info!("/logout               - Employee logout");
            tracing::info!("");
            tracing::info!("=== Message Bus RPC Commands ===");
            tracing::info!("/ping                 - Send ping via message bus");
            tracing::info!("/status               - Get server status via message bus");
            tracing::info!("/echo <msg>           - Echo message via message bus");
            tracing::info!("");
            tracing::info!("=== HTTP API Commands ===");
            tracing::info!("/health               - Check server health (no auth)");
            tracing::info!("/me                   - Get current user info (requires login)");
            tracing::info!("");
            tracing::info!("=== Credential Commands ===");
            tracing::info!("/refresh              - Refresh credential timestamp");
            tracing::info!("/check                - Run self-check");
            tracing::info!("/quit                 - Exit application");
        }

        // === Connection Commands ===
        "/setup" => {
            if parts.len() < 3 {
                tracing::error!("Usage: /setup <username> <password>");
                return;
            }
            let username = parts[1];
            let password = parts[2];

            tracing::info!("Setting up with user: {}", username);

            let client = match CrabClient::remote()
                .auth_server(AUTH_SERVER)
                .edge_server(EDGE_HTTPS)
                .cert_path(CERT_PATH)
                .client_name(CLIENT_NAME)
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Failed to build client: {}", e);
                    return;
                }
            };

            match client.setup(username, password, MESSAGE_ADDR).await {
                Ok(connected) => {
                    tracing::info!("Setup successful! Connected to {}", MESSAGE_ADDR);
                    app.phase = ClientPhase::Connected;
                    app.status.is_connected = true;
                    update_credential_status(&mut app.status, cert_manager);

                    // 启动通知监听器
                    if let Some(mc) = connected.message_client() {
                        spawn_notification_listener(mc);
                        tracing::info!("📡 Notification listener started");
                    }

                    // 存储连接状态 (edge_http_client 已在 CrabClient 内部创建)
                    *client_state.write().await = ClientState::Connected(connected);
                }
                Err(e) => {
                    tracing::error!("Setup failed: {}", e);
                }
            }
        }

        "/reconnect" => {
            tracing::info!("Reconnecting (with self-check & timestamp refresh)...");

            let client = match CrabClient::remote()
                .auth_server(AUTH_SERVER)
                .edge_server(EDGE_HTTPS)
                .cert_path(CERT_PATH)
                .client_name(CLIENT_NAME)
                .build()
            {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Failed to build client: {}", e);
                    return;
                }
            };

            if !client.has_cached_credentials() {
                tracing::error!("No cached credentials. Use /setup first.");
                return;
            }

            match client.reconnect(MESSAGE_ADDR).await {
                Ok(connected) => {
                    tracing::info!("Reconnected successfully!");
                    app.phase = ClientPhase::Connected;
                    app.status.is_connected = true;
                    update_credential_status(&mut app.status, cert_manager);

                    // 启动通知监听器
                    if let Some(mc) = connected.message_client() {
                        spawn_notification_listener(mc);
                        tracing::info!("📡 Notification listener started");
                    }

                    // 存储连接状态 (edge_http_client 已在 CrabClient 内部创建)
                    *client_state.write().await = ClientState::Connected(connected);
                }
                Err(e) => {
                    tracing::error!("Reconnect failed: {}", e);
                }
            }
        }

        "/disconnect" => {
            tracing::info!("Disconnecting...");

            let mut state = client_state.write().await;
            match std::mem::take(&mut *state) {
                ClientState::Connected(client) => {
                    let _ = client.disconnect().await;
                }
                ClientState::Authenticated(client) => {
                    let client = client.logout().await;
                    let _ = client.disconnect().await;
                }
                ClientState::Disconnected => {}
            }
            *state = ClientState::Disconnected;

            app.phase = ClientPhase::Disconnected;
            app.status.is_connected = false;
            app.status.has_token = false;
            app.status.token_preview = String::new();
        }

        // === Authentication Commands ===
        "/login" => {
            if parts.len() < 3 {
                tracing::error!("Usage: /login <username> <password>");
                return;
            }
            let username = parts[1];
            let password = parts[2];

            // 自动确保已连接
            if !ensure_connected(&client_state).await {
                tracing::error!("Failed to connect. Please check credentials and certificates.");
                return;
            }

            let mut state = client_state.write().await;
            match std::mem::take(&mut *state) {
                ClientState::Connected(client) => {
                    tracing::info!("Logging in as {}...", username);
                    match client.login(username, password).await {
                        Ok(authenticated) => {
                            let token = authenticated.token().unwrap_or("");
                            tracing::info!("Login successful!");
                            app.phase = ClientPhase::Authenticated;
                            app.status.has_token = true;
                            app.status.token_preview =
                                token.chars().take(16).collect::<String>() + "...";
                            *state = ClientState::Authenticated(authenticated);
                        }
                        Err(e) => {
                            tracing::error!("Login failed: {}", e);
                            // 自动重连并保持 Connected 状态
                            if ensure_connected(&client_state).await {
                                tracing::info!("Reconnected. Please try login again.");
                            }
                        }
                    }
                }
                ClientState::Authenticated(client) => {
                    tracing::warn!("Already logged in. Use /logout first.");
                    *state = ClientState::Authenticated(client);
                }
                ClientState::Disconnected => {
                    tracing::error!("Not connected.");
                    *state = ClientState::Disconnected;
                }
            }
        }

        "/logout" => {
            let mut state = client_state.write().await;
            match std::mem::take(&mut *state) {
                ClientState::Authenticated(client) => {
                    tracing::info!("Logging out...");
                    let connected = client.logout().await;
                    app.phase = ClientPhase::Connected;
                    app.status.has_token = false;
                    app.status.token_preview = String::new();
                    *state = ClientState::Connected(connected);
                }
                other => {
                    tracing::warn!("Not logged in.");
                    *state = other;
                }
            }
        }

        // === Message Bus RPC Commands (only requires mTLS connection) ===
        "/ping" => {
            let state = client_state.read().await;
            let msg_client = match &*state {
                ClientState::Connected(c) => c.message_client(),
                ClientState::Authenticated(c) => c.message_client(),
                ClientState::Disconnected => None,
            };
            if let Some(mc) = msg_client {
                send_rpc(app, "ping", None, mc).await;
            } else {
                tracing::error!("Not connected. Use /reconnect first.");
            }
        }

        "/status" => {
            let state = client_state.read().await;
            let msg_client = match &*state {
                ClientState::Connected(c) => c.message_client(),
                ClientState::Authenticated(c) => c.message_client(),
                ClientState::Disconnected => None,
            };
            if let Some(mc) = msg_client {
                send_rpc(app, "status", None, mc).await;
            } else {
                tracing::error!("Not connected. Use /reconnect first.");
            }
        }

        "/echo" => {
            if parts.len() < 2 {
                tracing::error!("Usage: /echo <message>");
                return;
            }
            let msg = parts[1..].join(" ");

            let state = client_state.read().await;
            let msg_client = match &*state {
                ClientState::Connected(c) => c.message_client(),
                ClientState::Authenticated(c) => c.message_client(),
                ClientState::Disconnected => None,
            };
            if let Some(mc) = msg_client {
                send_rpc(app, "echo", Some(serde_json::json!({ "message": msg })), mc).await;
            } else {
                tracing::error!("Not connected. Use /reconnect first.");
            }
        }

        // === HTTP API Commands (HTTPS + mTLS) ===
        "/health" => {
            tracing::info!("Checking server health...");

            // 从 CrabClient 获取缓存的 mTLS HTTP 客户端
            let state = client_state.read().await;
            let http_client = match &*state {
                ClientState::Connected(c) => c.edge_http_client(),
                ClientState::Authenticated(c) => c.edge_http_client(),
                ClientState::Disconnected => None,
            };

            if let Some(http) = http_client {
                match http.get(format!("{}/health", EDGE_HTTPS)).send().await {
                    Ok(resp) => {
                        let status = resp.status();
                        let body = resp.text().await.unwrap_or_default();
                        tracing::info!("Health: {} - {}", status, body);
                        app.status.last_rpc_result = format!("HTTP {}", status);
                    }
                    Err(e) => {
                        tracing::error!("Health check failed: {}", e);
                        app.status.last_rpc_result = "Error".to_string();
                    }
                }
            } else {
                tracing::error!("mTLS HTTP client not available.");
                tracing::error!("Connect first using /setup or /reconnect");
                app.status.last_rpc_result = "Error".to_string();
            }
        }

        "/me" => {
            let state = client_state.read().await;
            match &*state {
                ClientState::Authenticated(client) => {
                    if let Some(token) = client.token() {
                        tracing::info!("Fetching user info...");

                        // 使用 CrabClient 的 mTLS HTTP 客户端
                        if let Some(http) = client.edge_http_client() {
                            match http
                                .get(format!("{}/api/auth/me", EDGE_HTTPS))
                                .header("Authorization", format!("Bearer {}", token))
                                .send()
                                .await
                            {
                                Ok(resp) => {
                                    let status = resp.status();
                                    let body = resp.text().await.unwrap_or_default();
                                    if status.is_success() {
                                        tracing::info!("User info: {}", body);
                                    } else {
                                        tracing::warn!("Response: {} - {}", status, body);
                                    }
                                    app.status.last_rpc_result = format!("HTTP {}", status);
                                }
                                Err(e) => {
                                    tracing::error!("Request failed: {}", e);
                                    app.status.last_rpc_result = "Error".to_string();
                                }
                            }
                        } else {
                            tracing::error!("mTLS HTTP client not available.");
                            app.status.last_rpc_result = "Error".to_string();
                        }
                    }
                }
                _ => {
                    tracing::error!("Not authenticated. Use /login first.");
                }
            }
        }

        // === Credential Commands ===
        "/refresh" => {
            tracing::info!("Refreshing credential timestamp from Auth Server...");
            match cert_manager.refresh_credential_timestamp(AUTH_SERVER).await {
                Ok(()) => {
                    tracing::info!("Timestamp refreshed successfully!");
                    update_credential_status(&mut app.status, cert_manager);
                }
                Err(e) => {
                    tracing::error!("Refresh failed: {}", e);
                }
            }
        }

        "/check" => {
            tracing::info!("Running self-check...");
            match cert_manager.self_check() {
                Ok(()) => {
                    tracing::info!("Self-check passed!");
                    update_credential_status(&mut app.status, cert_manager);
                }
                Err(e) => {
                    tracing::error!("Self-check failed: {}", e);
                }
            }
        }

        "/quit" => {
            tracing::info!("Use 'q' key in normal mode to quit.");
        }

        _ => {
            tracing::warn!(
                "Unknown command: {}. Type /help for available commands.",
                parts[0]
            );
        }
    }
}

/// 发送 RPC 请求 (只需 mTLS 连接)
async fn send_rpc(
    app: &mut App,
    action: &str,
    params: Option<serde_json::Value>,
    message_client: &NetworkMessageClient,
) {
    tracing::info!("Sending RPC: {}", action);

    let request = BusMessage::request_command(&shared::message::RequestCommandPayload {
        action: action.to_string(),
        params,
    });

    match message_client
        .request(&request, Duration::from_secs(5))
        .await
    {
        Ok(response) => {
            match response.parse_payload::<shared::message::ResponsePayload>() {
                Ok(payload) => {
                    let status_str = if payload.success { "OK" } else { "FAIL" };
                    tracing::info!("Response: {} - {}", status_str, payload.message);
                    if let Some(data) = &payload.data {
                        tracing::info!("Data: {}", data);
                    }
                    app.status.last_rpc_result = status_str.to_string();
                }
                Err(e) => {
                    tracing::warn!("Failed to parse response: {}", e);
                    app.status.last_rpc_result = "Parse Error".to_string();
                }
            }
            app.status.rpc_count += 1;
        }
        Err(e) => {
            tracing::error!("RPC failed: {}", e);
            app.status.last_rpc_result = "Error".to_string();
        }
    }
}

fn ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header
            Constraint::Min(1),    // Main (Logs + Status)
            Constraint::Length(3), // Input
        ])
        .split(f.area());

    // Split Main into Logs and Status
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(60), // Logs
            Constraint::Percentage(40), // Status
        ])
        .split(chunks[1]);

    // Header
    let phase_text = match app.phase {
        ClientPhase::Disconnected => ("Disconnected", Color::Red),
        ClientPhase::Connected => ("Connected", Color::Yellow),
        ClientPhase::Authenticated => ("Authenticated", Color::Green),
    };

    let title = Paragraph::new(vec![Line::from(vec![
        Span::raw(" "),
        Span::styled(
            "Crab Message Client",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled(
            phase_text.0,
            Style::default()
                .fg(phase_text.1)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" | "),
        Span::styled(
            format!("RPC: {}", app.status.rpc_count),
            Style::default().fg(Color::Magenta),
        ),
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
                .title(" Logs ")
                .borders(Borders::ALL)
                .border_style(
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::DIM),
                ),
        )
        .output_separator('|')
        .output_timestamp(Some("%H:%M:%S".to_string()))
        .output_level(Some(TuiLoggerLevelOutput::Abbreviated))
        .output_target(false)
        .output_file(false)
        .output_line(false)
        .style(Style::default().fg(Color::White))
        .state(&app.logger_state);
    f.render_widget(tui_sm, main_chunks[0]);

    // Status Panel
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6), // Connection
            Constraint::Length(9), // Credential
            Constraint::Min(1),    // Certificate & RPC
        ])
        .split(main_chunks[1]);

    // Connection Status
    let conn_block = Block::default()
        .title(" Connection ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let conn_style = if app.status.is_connected {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Red)
    };

    let conn_text = vec![
        Line::from(vec![
            Span::raw("Status: "),
            Span::styled(
                if app.status.is_connected {
                    "Connected"
                } else {
                    "Disconnected"
                },
                conn_style,
            ),
        ]),
        Line::from(vec![
            Span::raw("Server: "),
            Span::styled(&app.status.server_addr, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::raw("Token:  "),
            if app.status.has_token {
                Span::styled(&app.status.token_preview, Style::default().fg(Color::Green))
            } else {
                Span::styled("None", Style::default().fg(Color::Gray))
            },
        ]),
    ];
    f.render_widget(Paragraph::new(conn_text).block(conn_block), right_chunks[0]);

    // Credential Status
    let cred_block = Block::default()
        .title(" Credential ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let cred_style = if app.status.has_credential {
        Style::default().fg(Color::Green)
    } else {
        Style::default().fg(Color::Red)
    };

    let clock_style = if app.status.clock_status == "OK" {
        Style::default().fg(Color::Green)
    } else if app.status.clock_status.starts_with("WARN") {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::Gray)
    };

    let cred_text = vec![
        Line::from(vec![
            Span::raw("Cached: "),
            Span::styled(
                if app.status.has_credential {
                    "Yes"
                } else {
                    "No"
                },
                cred_style,
            ),
            Span::raw("  Certs: "),
            Span::styled(
                if app.status.has_certificates {
                    "Yes"
                } else {
                    "No"
                },
                if app.status.has_certificates {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                },
            ),
        ]),
        Line::from(vec![
            Span::raw("Tenant: "),
            Span::styled(&app.status.tenant_id, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::raw("Device: "),
            Span::styled(&app.status.device_id, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![Span::raw("")]),
        Line::from(vec![
            Span::raw("Clock:  "),
            Span::styled(&app.status.clock_status, clock_style),
        ]),
        Line::from(vec![
            Span::raw("Last:   "),
            Span::styled(
                &app.status.last_verified_at,
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(cred_text).block(cred_block), right_chunks[1]);

    // Certificate & RPC Status
    let bottom_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(4), // Certificate
            Constraint::Min(1),    // RPC
        ])
        .split(right_chunks[2]);

    let cert_block = Block::default()
        .title(" Certificate ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let cert_text = vec![Line::from(vec![
        Span::raw("Expires: "),
        Span::styled(
            &app.status.cert_expires_at,
            Style::default().fg(Color::Yellow),
        ),
    ])];
    f.render_widget(
        Paragraph::new(cert_text).block(cert_block),
        bottom_chunks[0],
    );

    // RPC Status
    let rpc_block = Block::default()
        .title(" Last RPC ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let rpc_style = if app.status.last_rpc_result == "OK"
        || app.status.last_rpc_result.starts_with("HTTP 2")
    {
        Style::default().fg(Color::Green)
    } else if app.status.last_rpc_result.contains("Error") || app.status.last_rpc_result == "FAIL" {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::Yellow)
    };

    let rpc_text = vec![Line::from(vec![
        Span::raw("Result: "),
        Span::styled(
            if app.status.last_rpc_result.is_empty() {
                "-"
            } else {
                &app.status.last_rpc_result
            },
            rpc_style,
        ),
    ])];
    f.render_widget(Paragraph::new(rpc_text).block(rpc_block), bottom_chunks[1]);

    // Input
    let input_block = Block::default()
        .borders(Borders::ALL)
        .title(" Command (Type /help) ");

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
}
