//! Interactive Demo - TUI Enhanced Edge Server Experience
//!
//! Run: cargo run --example interactive_demo

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use edge_server::Config;
use edge_server::message::ConnectedClient;
use edge_server::server::ServerState;
use edge_server::{BusMessage, MessageClient};
use ratatui::{prelude::*, widgets::*};
use std::io::{self, Stdout};
use std::time::Duration;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;
use tui_logger::{TuiLoggerLevelOutput, TuiLoggerWidget, TuiWidgetEvent, TuiWidgetState};

use tokio::sync::mpsc;

#[derive(Default, Clone)]
struct DemoStatus {
    is_active: bool,
    tenant_id: String,
    edge_id: String,
    device_id: String,
    plan: String,
    sub_status: String,
    expires_at: String,
    clients: Vec<ConnectedClient>,
}

struct App {
    /// Input field state
    input: Input,
    /// Current input mode
    input_mode: InputMode,
    /// Message client for sending commands
    msg_client: Option<MessageClient>,
    /// Server state reference
    server_state: Option<ServerState>,
    /// Loading state
    is_loading: bool,
    /// Current Status
    status: DemoStatus,
    /// Logger Widget State
    logger_state: TuiWidgetState,
}

impl Default for App {
    fn default() -> Self {
        Self {
            input: Input::default(),
            input_mode: InputMode::default(),
            msg_client: None,
            server_state: None,
            is_loading: false,
            status: DemoStatus::default(),
            logger_state: TuiWidgetState::new(),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum InputMode {
    #[default]
    Normal,
    Editing,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Install default crypto provider
    let _ = rustls::crypto::ring::default_provider().install_default();

    // Initialize TUI Logger with Tracing
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,surrealdb=warn"));

    tracing_subscriber::registry()
        .with(tui_logger::tracing_subscriber_layer())
        .with(env_filter)
        .init();

    // Also init log crate adapter just in case dependencies use log crate
    tui_logger::init_logger(log::LevelFilter::Info).ok();
    tui_logger::set_default_level(log::LevelFilter::Info);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create App state
    let mut app = App {
        is_loading: true,
        ..Default::default()
    };

    // Channel to receive initialized state
    let (tx, mut rx) = mpsc::channel(1);
    // Channel to receive status updates
    let (status_tx, mut status_rx) = mpsc::channel(1);

    // Start Server Logic in Background
    tokio::spawn(async move {
        tracing::debug!("Starting Edge Server...");

        // Create a temporary directory for this demo
        let temp_dir = "./temp_interactive_demo";
        if !std::path::Path::new(temp_dir).exists() {
            std::fs::create_dir_all(temp_dir).ok();
        }
        tracing::debug!("Data directory: {}", temp_dir);

        // 1. Start edge-server
        let mut config = Config::with_overrides(temp_dir, 3000, 8081);
        config.environment = "production".to_string();
        config.jwt.secret = "demo-secret".to_string();
        config.auth_server_url = "http://127.0.0.1:3001".to_string();

        let state = ServerState::initialize(&config).await;
        // Start background tasks immediately as we are not using Server::run to initialize state
        // But since we are going to use Server::with_state().run(), we need to be careful not to double start
        // NOTE: Server::run() now unconditionally calls start_background_tasks().
        // However, the interactive demo needs MessageBus running BEFORE Server::run() is called
        // because we create a MessageClient below and start subscribing.
        // If we wait for Server::run(), the MessageClient might try to connect to a non-running bus (though memory transport might be fine).
        // Actually, MemoryTransport doesn't need "start_background_tasks" to be *called* to exist,
        // but MessageHandler needs to run to *process* messages.
        //
        // If we let Server::run() start it, we are fine as long as we don't block.
        // Server::run() is spawned below.
        //
        // But to be safe and allow UI to work even if Server::run takes a moment,
        // we might want to start it here. BUT Server::run will start it again.
        //
        // Ideally, start_background_tasks should be idempotent.
        // For now, let's trust Server::run() to start it, and we spawn Server::run immediately.

        // state.start_background_tasks().await; // <-- REMOVED to avoid double start

        // Send state back to UI
        if tx.send(state.clone()).await.is_err() {
            tracing::error!("Failed to send server state to UI");
        }

        // Lifecycle Manager: Use Server::run() instead of manual orchestration
        // This ensures we test the exact same code path as the real binary
        let server = edge_server::Server::with_state(config, state.clone());

        // We spawn Server::run in a separate task because it's blocking (until shutdown)
        tokio::spawn(async move {
            tracing::debug!("🚀 Starting Edge Server via Server::run()...");
            if let Err(e) = server.run().await {
                tracing::error!("❌ Server run error: {}", e);
            }
        });

        // 2. Start event receiver
        let bus = state.message_bus();
        let mut client_rx = bus.subscribe_to_clients();
        let mut server_rx = bus.subscribe();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    msg_result = client_rx.recv() => {
                        if let Ok(msg) = msg_result {
                            tracing::info!("📨 [CLIENT] {:?} | ID: {}", msg.event_type, msg.request_id);
                            if let Ok(payload) = msg.parse_payload::<serde_json::Value>() {
                                tracing::info!("   Data: {}", payload);
                            }
                        }
                    }
                    msg_result = server_rx.recv() => {
                        if let Ok(msg) = msg_result {
                            let prefix = match msg.event_type {
                                edge_server::message::EventType::Notification => "📢 [NOTIFY]",
                                edge_server::message::EventType::ServerCommand => "🎮 [CMD]",
                                edge_server::message::EventType::Handshake => "🤝 [HANDSHAKE]",
                                edge_server::message::EventType::RequestCommand => "⚡ [REQ]",
                                edge_server::message::EventType::Sync => "🔄 [SYNC]",
                                edge_server::message::EventType::Response => "🔙 [RESP]",
                            };
                            tracing::info!("{} {:?}", prefix, msg.event_type);
                            if let Ok(payload) = msg.parse_payload::<serde_json::Value>() {
                                tracing::info!("   Data: {}", payload);
                            }
                        }
                    }
                }
            }
        });

        // Send state back to UI
        if tx.send(state.clone()).await.is_err() {
            tracing::error!("Failed to send server state to UI");
        }

        // Start Status Poller
        tokio::spawn(async move {
            loop {
                // Gather status
                let activation_res = state.activation_service().get_status().await;
                let cred_cache = state.activation_service().credential_cache.read().await;

                let mut status = DemoStatus::default();
                status.clients = state.message_bus().get_connected_clients();

                if let Ok(act) = activation_res {
                    status.is_active = act.is_activated;
                }

                if let Some(cred) = &*cred_cache {
                    status.tenant_id = cred.tenant_id.clone();
                    status.edge_id = cred.server_id.clone();
                    status.device_id = cred.device_id.clone().unwrap_or_else(|| "-".to_string());

                    if let Some(sub) = &cred.subscription {
                        status.plan = format!("{:?}", sub.plan);
                        status.sub_status = format!("{:?}", sub.status);
                        status.expires_at = sub
                            .expires_at
                            .map(|d| d.to_rfc3339())
                            .unwrap_or_else(|| "Never".to_string());
                    } else {
                        status.plan = "None".to_string();
                        status.sub_status = "No Subscription".to_string();
                        status.expires_at = "-".to_string();
                    }
                } else {
                    status.tenant_id = "-".to_string();
                    status.edge_id = "-".to_string();
                    status.device_id = "-".to_string();
                    status.plan = "None".to_string();
                    status.sub_status = "Not Activated".to_string();
                    status.expires_at = "-".to_string();
                }

                if status_tx.send(status).await.is_err() {
                    break;
                }

                tokio::time::sleep(Duration::from_secs(2)).await;
            }
        });
    });

    // Run TUI loop
    let res = run_app(&mut terminal, &mut app, &mut rx, &mut status_rx).await;

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err);
    }

    Ok(())
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    app: &mut App,
    rx: &mut mpsc::Receiver<ServerState>,
    status_rx: &mut mpsc::Receiver<DemoStatus>,
) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, app))?;

        // If still loading, wait for initialization or tick
        if app.is_loading {
            tokio::select! {
                state_res = rx.recv() => {
                    if let Some(state) = state_res {
                         app.msg_client = Some(MessageClient::memory(state.message_bus()));
                         app.server_state = Some(state);
                         app.is_loading = false;
                         // Force a manual log to verify visibility
                         tracing::info!("Press 'e' to edit commands, 'q' to quit");
                         tracing::info!("Use Up/Down/PgUp/PgDown to scroll logs");
                    }
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Just a tick to keep UI refreshing if needed
                }
            }
            continue;
        }

        // Handle status updates and input
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
                                        handle_command(app, &input_str).await;
                                        app.input.reset();
                                    }
                                    // Stay in editing mode for convenience, or switch back?
                                    // Let's stay in editing mode like a terminal
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

        // Poll for status updates (non-blocking)
        if let Ok(status) = status_rx.try_recv() {
            app.status = status;
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
            tracing::info!("Available commands:");
            tracing::info!(
                "  /notify [@client] <title> <msg> - Send notification (Broadcast or Unicast)"
            );
            tracing::info!("  /sync <res> <id> <act>          - Send sync signal (Broadcast)");
            tracing::info!("  /activate <user> <pass>         - Activate server with Auth Server");
            tracing::info!("  /ping                           - Send ping command (Broadcast)");
            tracing::info!("  /clear                          - Clear logs");
            tracing::info!("  /quit                           - Exit");
        }
        "/quit" => {
            // We can't easily exit main loop from here without return value,
            // but we can log instruction
            tracing::warn!("Press Esc then 'q' to quit application");
        }
        "/clear" => {
            // tui-logger doesn't easily support clearing, but we can emit a separator
            tracing::info!("--- LOGS CLEARED (Virtual) ---");
        }
        "/activate" => {
            if parts.len() < 3 {
                tracing::error!("Usage: /activate <username> <password>");
                return;
            }
            let username = parts[1].to_string();
            let password = parts[2].to_string();

            if let Some(state) = &app.server_state {
                tracing::info!("Starting activation for user: {}", username);
                // Hardcoded for demo purposes, matches the config in main()
                let auth_url = "http://127.0.0.1:3001".to_string();

                let state_clone = state.clone();
                tokio::spawn(async move {
                    let service = state_clone.provisioning_service(auth_url);
                    match service.activate(&username, &password).await {
                        Ok(_) => tracing::info!("✅ Activation successful! Server is now ready."),
                        Err(e) => tracing::error!("❌ Activation failed: {}", e),
                    }
                });
            } else {
                tracing::error!("Server not ready");
            }
        }
        "/notify" => {
            if parts.len() < 3 {
                tracing::error!("Usage: /notify [@client] <title> <message>");
                return;
            }

            if let Some(state) = &app.server_state {
                let target_arg = parts[1];

                if target_arg.starts_with('@') {
                    // Unicast to specific client
                    if parts.len() < 4 {
                        tracing::error!("Usage: /notify @<client> <title> <message>");
                        return;
                    }

                    let client_name = &target_arg[1..]; // Remove '@'
                    let title = parts[2];
                    let msg_content = parts[3..].join(" ");

                    // Find client by name (peer_identity)
                    let clients = state.message_bus().get_connected_clients();
                    let target_client = clients
                        .iter()
                        .find(|c| c.peer_identity.as_deref() == Some(client_name));

                    if let Some(client) = target_client {
                        let payload =
                            shared::message::NotificationPayload::info(title, &msg_content);
                        let mut msg = BusMessage::notification(&payload);
                        msg.target = Some(client.id.clone());

                        if let Err(e) = state.message_bus().send_to_client(&client.id, msg).await {
                            tracing::error!("Failed to send to {}: {}", client_name, e);
                        } else {
                            tracing::info!(
                                "✅ Sent Notification to {}: {} - {}",
                                client_name,
                                title,
                                msg_content
                            );
                        }
                    } else {
                        tracing::error!("❌ Client '{}' not found", client_name);
                    }
                } else {
                    // Broadcast (Original behavior)
                    let title = parts[1];
                    let msg_content = parts[2..].join(" ");

                    let payload = shared::message::NotificationPayload::info(title, &msg_content);
                    let msg = BusMessage::notification(&payload);

                    // Broadcast to all clients
                    if let Err(e) = state.message_bus().publish(msg).await {
                        tracing::error!("Failed to broadcast: {}", e);
                    } else {
                        tracing::info!("✅ Broadcasted Notification: {} - {}", title, msg_content);
                    }
                }
            } else {
                tracing::error!("Server not ready");
            }
        }
        "/ping" => {
            if let Some(state) = &app.server_state {
                let cmd = shared::message::ServerCommand::Ping;
                let payload = shared::message::ServerCommandPayload { command: cmd };
                let msg = BusMessage::server_command(&payload);

                // Broadcast Ping (Simulate Upstream -> Edge -> Clients)
                if let Err(e) = state.message_bus().publish(msg).await {
                    tracing::error!("Failed to broadcast Ping: {}", e);
                } else {
                    tracing::info!("✅ Broadcasted Ping Command");
                }
            } else {
                tracing::error!("Server not ready");
            }
        }
        "/sync" => {
            if parts.len() < 4 {
                tracing::error!("Usage: /sync <resource> <id> <action>");
                return;
            }
            let resource = parts[1].to_string();
            let id = parts[2].to_string();
            let action = parts[3].to_string();

            if let Some(state) = &app.server_state {
                let payload = shared::message::SyncPayload {
                    resource: resource.clone(),
                    id: Some(id.clone()),
                    action: action.clone(),
                };
                let msg = BusMessage::sync(&payload);

                // Broadcast Sync (Server -> All Clients)
                if let Err(e) = state.message_bus().publish(msg).await {
                    tracing::error!("Failed to broadcast Sync: {}", e);
                } else {
                    tracing::info!("✅ Broadcasted Sync: {} {} {}", resource, id, action);
                }
            } else {
                tracing::error!("Server not ready");
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
            Constraint::Min(1),    // Main Content (Logs + Status)
            Constraint::Length(3), // Input
        ])
        .split(f.area());

    // Split Main Content into Logs (Left) and Status (Right)
    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(70), // Logs
            Constraint::Percentage(30), // Status
        ])
        .split(chunks[1]);

    // Header
    let title = Paragraph::new(vec![Line::from(vec![
        Span::raw(" 🦀 Crab Edge Server "),
        Span::styled(" Interactive Demo ", Style::default().fg(Color::Yellow)),
        Span::raw(" | "),
        if app.is_loading {
            Span::styled(
                " INITIALIZING... ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD | Modifier::RAPID_BLINK),
            )
        } else {
            Span::styled(
                " Running ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )
        },
    ])])
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan)),
    );
    f.render_widget(title, chunks[0]);

    // Logs (TuiLoggerWidget)
    let tui_sm = TuiLoggerWidget::default()
        .block(
            Block::default()
                .title(" Logs ")
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
        .style(Style::default().fg(Color::White))
        .state(&app.logger_state);
    f.render_widget(tui_sm, main_chunks[0]);

    // Right Side (Status + Clients)
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(10), // Status
            Constraint::Min(1),     // Clients
        ])
        .split(main_chunks[1]);

    // 1. Status Panel
    let status_block = Block::default()
        .title(" Status ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let active_style = if app.status.is_active {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Red)
    };

    let status_text = vec![
        Line::from(vec![
            Span::raw("Activation: "),
            Span::styled(
                if app.status.is_active {
                    "Active"
                } else {
                    "Inactive"
                },
                active_style,
            ),
        ]),
        Line::from(vec![Span::raw("")]),
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
            Span::raw("Plan: "),
            Span::styled(&app.status.plan, Style::default().fg(Color::Blue)),
        ]),
    ];

    let status_paragraph = Paragraph::new(status_text)
        .block(status_block)
        .wrap(Wrap { trim: true });

    f.render_widget(status_paragraph, right_chunks[0]);

    // 2. Connected Clients Panel
    let clients_block = Block::default()
        .title(format!(" Clients ({}) ", app.status.clients.len()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let client_items: Vec<ListItem> = app
        .status
        .clients
        .iter()
        .map(|c| {
            let name = c.peer_identity.as_deref().unwrap_or("Unknown");
            let addr = c.addr.as_deref().unwrap_or("Unknown");
            // Shorten ID for display
            let short_id = if c.id.len() > 8 { &c.id[..8] } else { &c.id };

            let content = vec![
                Line::from(vec![Span::styled(
                    format!("ID: {}..", short_id),
                    Style::default().fg(Color::Cyan),
                )]),
                Line::from(vec![
                    Span::raw(" Name: "),
                    Span::styled(name, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(vec![
                    Span::raw(" IP:   "),
                    Span::styled(addr, Style::default().fg(Color::Green)),
                ]),
                Line::from(Span::raw(" ")),
            ];

            ListItem::new(content)
        })
        .collect();

    let clients_list = List::new(client_items).block(clients_block);
    f.render_widget(clients_list, right_chunks[1]);

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

    // Help hint
    if app.input_mode == InputMode::Normal {
        let help_text = Paragraph::new("Press 'e' to edit, 'q' to quit")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Right);
        f.render_widget(help_text, chunks[0]); // Overlay on header or create footer?
        // Let's put it in the header right side
    }
}
