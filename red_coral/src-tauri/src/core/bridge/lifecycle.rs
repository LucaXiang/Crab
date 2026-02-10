//! Mode lifecycle: start, stop, restore, rebuild

use super::*;

impl ClientBridge {
    /// 恢复上次的会话状态 (启动时调用)
    pub async fn restore_last_session(self: &Arc<Self>) -> Result<(), BridgeError> {
        let config = self.config.read().await;
        let mode = config.current_mode;
        let client_config = config.client_config.clone();
        drop(config);

        // 注: 租户选择已在构造函数中同步恢复（确保 get_app_state 立即可用）

        let result = match mode {
            Some(ModeType::Server) => {
                tracing::info!("Restoring Server mode...");
                if let Err(e) = self.start_server_mode().await {
                    tracing::error!("Failed to restore Server mode: {}", e);
                    Err(e)
                } else {
                    Ok(())
                }
            }
            Some(ModeType::Client) => {
                if let Some(cfg) = client_config {
                    tracing::info!("Restoring Client mode...");
                    if let Err(e) = self
                        .start_client_mode(&cfg.edge_url, &cfg.message_addr)
                        .await
                    {
                        tracing::error!("Failed to restore Client mode: {}", e);
                        Err(e)
                    } else {
                        Ok(())
                    }
                } else {
                    tracing::warn!("Client mode configured but missing client_config");
                    Ok(())
                }
            }
            None => {
                tracing::info!("Starting in Disconnected mode (no mode selected)");
                Ok(())
            }
        };

        result
    }

    /// 以 Server 模式启动
    ///
    /// 如果已经在 Server 模式，直接返回成功（幂等操作）
    pub async fn start_server_mode(&self) -> Result<(), BridgeError> {
        let mut mode_guard = self.mode.write().await;

        // 如果已经在 Server 模式，直接返回成功
        if matches!(&*mode_guard, ClientMode::Server { .. }) {
            tracing::debug!("Already in Server mode, skipping start");
            return Ok(());
        }

        // 如果在 Client 模式，先停止再切换
        if matches!(&*mode_guard, ClientMode::Client { .. }) {
            tracing::debug!("Stopping Client mode to switch to Server mode");
            *mode_guard = ClientMode::Disconnected;
        }

        let config = self.config.read().await;
        let server_config = &config.server_config;

        let tenant_manager = self.tenant_manager.read().await;
        let work_dir = if let Some(path) = tenant_manager.current_tenant_path() {
            // Server work_dir is {tenant}/server/
            let server_dir = path.join("server");
            tracing::debug!(path = %server_dir.display(), "Using server directory");
            server_dir.to_string_lossy().to_string()
        } else {
            tracing::warn!(
                "No active tenant, falling back to default data dir: {:?}",
                server_config.data_dir
            );
            server_config.data_dir.to_string_lossy().to_string()
        };
        drop(tenant_manager);

        let auth_url = config.auth_url.clone();

        let edge_config = edge_server::Config::builder()
            .work_dir(work_dir)
            .http_port(server_config.http_port)
            .message_tcp_port(server_config.message_port)
            .auth_server_url(auth_url)
            .build();

        let server_state = edge_server::ServerState::initialize(&edge_config)
            .await
            .map_err(|e| BridgeError::Server(format!("Edge server initialization failed: {e}")))?;

        let server_instance =
            edge_server::Server::with_state(edge_config.clone(), server_state.clone());
        let shutdown_token = server_instance.shutdown_token();

        let server_task = tokio::spawn(async move {
            if let Err(e) = server_instance.run().await {
                tracing::error!("Server run error: {}", e);
            }
        });

        let state_arc = Arc::new(server_state);

        let router = state_arc
            .https_service()
            .router()
            .ok_or_else(|| {
                tracing::error!("Router is None after ServerState initialization");
                BridgeError::Server("Router not initialized".to_string())
            })?;

        let message_bus = state_arc.message_bus();
        let client_tx = message_bus.sender_to_server().clone();
        let server_tx = message_bus.sender().clone();

        // 启动消息广播订阅 (转发给前端)
        let listener_task = if let Some(handle) = &self.app_handle {
            let mut server_rx = message_bus.subscribe();
            let handle_clone = handle.clone();
            let listener_token = shutdown_token.clone();

            let handle = tokio::spawn(async move {
                tracing::debug!("Server message listener started");
                loop {
                    tokio::select! {
                        _ = listener_token.cancelled() => {
                            tracing::debug!("Server message listener shutdown");
                            break;
                        }
                        result = server_rx.recv() => {
                            match result {
                                Ok(msg) => {
                                    // Route messages to appropriate channels
                                    use crate::events::MessageRoute;
                                    match MessageRoute::from_bus_message(msg) {
                                        MessageRoute::OrderSync(order_sync) => {
                                            if let Err(e) = handle_clone.emit("order-sync", &*order_sync) {
                                                tracing::warn!("Failed to emit order sync: {}", e);
                                            }
                                        }
                                        MessageRoute::ServerMessage(event) => {
                                            tracing::debug!(event_type = %event.event_type, "Emitting server-message");
                                            if let Err(e) = handle_clone.emit("server-message", &event) {
                                                tracing::warn!("Failed to emit server message: {}", e);
                                            }
                                        }
                                    }
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                    tracing::warn!("Server message listener lagged {} messages", n);
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                    tracing::debug!("Server message channel closed");
                                    break;
                                }
                            }
                        }
                    }
                }
            });
            Some(handle)
        } else {
            None
        };

        let client = CrabClient::local()
            .with_router(router)
            .with_message_channels(client_tx, server_tx)
            .build()?;

        let connected_client = client.connect().await?;

        // 尝试加载缓存的员工会话
        let tenant_manager_read = self.tenant_manager.read().await;
        let cached_session = tenant_manager_read.load_current_session().ok().flatten();
        drop(tenant_manager_read);

        let client_state = if let Some(session) = cached_session {
            tracing::debug!(username = %session.username, "Restoring cached session");
            match connected_client
                .restore_session(session.token.clone(), session.user_info.clone())
                .await
            {
                Ok(authenticated_client) => {
                    tracing::debug!(username = %session.username, "Session restored");
                    let mut tenant_manager = self.tenant_manager.write().await;
                    tenant_manager.set_current_session(session);
                    LocalClientState::Authenticated(authenticated_client)
                }
                Err(e) => {
                    tracing::warn!("Failed to restore session: {}", e);
                    let tenant_manager = self.tenant_manager.read().await;
                    let _ = tenant_manager.clear_current_session();
                    let client = CrabClient::local()
                        .with_router(
                            state_arc
                                .https_service()
                                .router()
                                .ok_or_else(|| BridgeError::Server("Router not initialized".to_string()))?,
                        )
                        .with_message_channels(
                            state_arc.message_bus().sender_to_server().clone(),
                            state_arc.message_bus().sender().clone(),
                        )
                        .build()?;
                    LocalClientState::Connected(client.connect().await?)
                }
            }
        } else {
            tracing::debug!("No cached employee session found");
            LocalClientState::Connected(connected_client)
        };

        *mode_guard = ClientMode::Server {
            server_state: state_arc,
            client: Some(client_state),
            server_task,
            listener_task,
            shutdown_token,
        };

        let http_port = server_config.http_port;
        drop(config);
        {
            let mut config = self.config.write().await;
            config.current_mode = Some(ModeType::Server);
        }
        self.save_config().await?;

        tracing::info!(port = http_port, "Server mode started");
        Ok(())
    }

    /// 以 Client 模式连接
    pub async fn start_client_mode(
        self: &Arc<Self>,
        edge_url: &str,
        message_addr: &str,
    ) -> Result<(), BridgeError> {
        let mut mode_guard = self.mode.write().await;

        // 如果在其他模式，先停止
        if let ClientMode::Server { shutdown_token, .. } = &*mode_guard {
            tracing::info!("Stopping Server mode to switch to Client mode...");
            shutdown_token.cancel();
            let old_mode = std::mem::replace(&mut *mode_guard, ClientMode::Disconnected);
            drop(mode_guard);

            if let ClientMode::Server {
                server_task,
                listener_task,
                ..
            } = old_mode
            {
                let server_abort = server_task.abort_handle();
                let listener_abort = listener_task.as_ref().map(|lt| lt.abort_handle());

                match tokio::time::timeout(std::time::Duration::from_secs(10), async {
                    let server_result = server_task.await;
                    if let Some(lt) = listener_task {
                        let _ = lt.await;
                    }
                    server_result
                })
                .await
                {
                    Ok(Ok(())) => tracing::debug!("Server tasks completed gracefully"),
                    Ok(Err(e)) if e.is_cancelled() => tracing::debug!("Server task cancelled"),
                    Ok(Err(e)) => tracing::error!("Server task panicked: {}", e),
                    Err(_) => {
                        tracing::warn!("Server shutdown timed out (10s), aborting remaining tasks");
                        server_abort.abort();
                        if let Some(la) = listener_abort {
                            la.abort();
                        }
                    }
                }
            }
        } else if let ClientMode::Client { shutdown_token, .. } = &*mode_guard {
            tracing::info!("Already in Client mode, stopping first...");
            shutdown_token.cancel();
            *mode_guard = ClientMode::Disconnected;
            drop(mode_guard);
        } else {
            drop(mode_guard);
        }

        let tenant_manager = self.tenant_manager.read().await;
        let paths = tenant_manager
            .current_paths()
            .ok_or(TenantError::NoTenantSelected)?;

        let config = self.config.read().await;
        let auth_url = config.auth_url.clone();
        drop(config);

        if !paths.has_client_certificates() {
            return Err(BridgeError::Config(
                "No cached certificates. Please activate tenant first.".into(),
            ));
        }

        // CrabClient 使用 cert_path + client_name 构建 CertManager
        // 我们传 certs_dir 作为 cert_path，空字符串作为 client_name
        // 这样 CertManager 会在 {tenant}/certs/ 查找证书
        // 握手时 CrabClient 会自动从证书中读取正确的 name
        let client = CrabClient::remote()
            .auth_server(&auth_url)
            .edge_server(edge_url)
            .cert_path(paths.certs_dir())
            .client_name("") // 空字符串使 CertManager 直接使用 certs_dir
            .build()?;

        let connected_client = client.connect_with_credentials(message_addr).await?;

        tracing::info!(edge_url = %edge_url, message_addr = %message_addr, "Client mode connected");

        let client_shutdown_token = tokio_util::sync::CancellationToken::new();

        // 启动消息广播订阅 (转发给前端)
        if let Some(handle) = &self.app_handle {
            if let Some(mc) = connected_client.message_client() {
                // 消息监听
                let mut rx = mc.subscribe();
                let handle_clone = handle.clone();
                let token = client_shutdown_token.clone();

                tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            _ = token.cancelled() => {
                                tracing::debug!("Client message listener shutdown");
                                break;
                            }
                            result = rx.recv() => {
                                match result {
                                    Ok(msg) => {
                                        use crate::events::MessageRoute;
                                        match MessageRoute::from_bus_message(msg) {
                                            MessageRoute::OrderSync(order_sync) => {
                                                if let Err(e) =
                                                    handle_clone.emit("order-sync", &*order_sync)
                                                {
                                                    tracing::warn!("Failed to emit order sync: {}", e);
                                                }
                                            }
                                            MessageRoute::ServerMessage(event) => {
                                                if let Err(e) = handle_clone.emit("server-message", &event)
                                                {
                                                    tracing::warn!("Failed to emit server message: {}", e);
                                                }
                                            }
                                        }
                                    }
                                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                        tracing::warn!("Client message listener lagged {} messages", n);
                                    }
                                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                        tracing::debug!("Client message channel closed");
                                        break;
                                    }
                                }
                            }
                        }
                    }
                });

                tracing::debug!("Client message listener started");

                // 重连事件监听 (心跳失败或网络断开时触发)
                let mut reconnect_rx = mc.subscribe_reconnect();
                let handle_reconnect = handle.clone();
                let token = client_shutdown_token.clone();

                // 获取 bridge 自身引用（用于 ReconnectFailed 时触发重建）
                let bridge_for_rebuild = Arc::clone(self);

                tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            _ = token.cancelled() => {
                                tracing::debug!("Client reconnect listener shutdown");
                                break;
                            }
                            result = reconnect_rx.recv() => {
                                match result {
                                    Ok(event) => {
                                        use crab_client::ReconnectEvent;
                                        match event {
                                            ReconnectEvent::Disconnected => {
                                                tracing::warn!("Client disconnected, waiting for reconnection...");
                                                if let Err(e) = handle_reconnect.emit("connection-state-changed", false) {
                                                    tracing::warn!("Failed to emit connection state: {}", e);
                                                }
                                            }
                                            ReconnectEvent::Reconnected => {
                                                tracing::info!("Client reconnected successfully");
                                                if let Err(e) = handle_reconnect.emit("connection-state-changed", true) {
                                                    tracing::warn!("Failed to emit connection state: {}", e);
                                                }
                                            }
                                            ReconnectEvent::ReconnectFailed { attempts } => {
                                                tracing::error!("Client reconnection failed after {} attempts, triggering bridge rebuild", attempts);
                                                if let Err(e) = handle_reconnect.emit("connection-state-changed", false) {
                                                    tracing::warn!("Failed to emit connection state: {}", e);
                                                }

                                                // 在独立 task 中执行重建
                                                let bridge_arc = Arc::clone(&bridge_for_rebuild);
                                                let rebuild_handle = handle_reconnect.clone();
                                                tauri::async_runtime::spawn(async move {
                                                    do_rebuild_connection(bridge_arc, rebuild_handle).await;
                                                });

                                                // 退出此监听器（start_client_mode 会创建新的监听器）
                                                break;
                                            }
                                        }
                                    }
                                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                        tracing::warn!("Reconnect event listener lagged {} events", n);
                                    }
                                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                        tracing::debug!("Reconnect event channel closed");
                                        break;
                                    }
                                }
                            }
                        }
                    }
                });

                tracing::debug!("Client reconnection listener started");

                // 心跳状态监听 (每次心跳成功/失败都会触发)
                let mut heartbeat_rx = mc.subscribe_heartbeat();
                let handle_heartbeat = handle.clone();
                let token = client_shutdown_token.clone();

                tokio::spawn(async move {
                    loop {
                        tokio::select! {
                            _ = token.cancelled() => {
                                tracing::debug!("Client heartbeat listener shutdown");
                                break;
                            }
                            result = heartbeat_rx.recv() => {
                                match result {
                                    Ok(status) => {
                                        if let Err(e) = handle_heartbeat.emit("heartbeat-status", &status) {
                                            tracing::warn!("Failed to emit heartbeat status: {}", e);
                                        }
                                    }
                                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                                        tracing::warn!("Heartbeat listener lagged {} events", n);
                                    }
                                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                                        tracing::debug!("Heartbeat channel closed");
                                        break;
                                    }
                                }
                            }
                        }
                    }
                });

                tracing::debug!("Client heartbeat listener started");
            }
        }

        {
            let mut mode_guard = self.mode.write().await;
            if !matches!(&*mode_guard, ClientMode::Disconnected) {
                tracing::warn!("Mode changed during Client setup, aborting");
                client_shutdown_token.cancel();
                return Err(BridgeError::Server(
                    "Mode changed during client setup".to_string(),
                ));
            }
            *mode_guard = ClientMode::Client {
                client: Some(RemoteClientState::Connected(connected_client)),
                edge_url: edge_url.to_string(),
                message_addr: message_addr.to_string(),
                shutdown_token: client_shutdown_token,
            };
        }

        {
            let mut config = self.config.write().await;
            config.current_mode = Some(ModeType::Client);
            config.client_config = Some(ClientModeConfig {
                edge_url: edge_url.to_string(),
                message_addr: message_addr.to_string(),
            });
        }
        self.save_config().await?;

        Ok(())
    }

    /// 停止当前模式（优雅关闭）
    ///
    /// Server 模式: cancel shutdown_token → 等待 server_task + listener_task（10s 超时）
    /// Client 模式: cancel shutdown_token → 监听器自行退出
    pub async fn stop(&self) -> Result<(), BridgeError> {
        let mut mode_guard = self.mode.write().await;

        // 1. 发送 graceful shutdown 信号
        match &*mode_guard {
            ClientMode::Server { shutdown_token, .. } => {
                shutdown_token.cancel();
                tracing::info!("Server shutdown signal sent, waiting for tasks to stop...");
            }
            ClientMode::Client { shutdown_token, .. } => {
                shutdown_token.cancel();
                tracing::info!("Client shutdown signal sent");
            }
            ClientMode::Disconnected => {}
        }

        // 2. 取出 mode（move ownership of server_task 才能 await）
        let old_mode = std::mem::replace(&mut *mode_guard, ClientMode::Disconnected);
        drop(mode_guard);

        // 3. 等待 server_task + listener_task 完成（10s 超时保底）
        if let ClientMode::Server {
            server_task,
            listener_task,
            ..
        } = old_mode
        {
            let server_abort = server_task.abort_handle();
            let listener_abort = listener_task.as_ref().map(|lt| lt.abort_handle());

            match tokio::time::timeout(std::time::Duration::from_secs(10), async {
                // 并行等待 server_task 和 listener_task
                let server_result = server_task.await;
                if let Some(lt) = listener_task {
                    let _ = lt.await;
                }
                server_result
            })
            .await
            {
                Ok(Ok(())) => tracing::debug!("Server tasks completed gracefully"),
                Ok(Err(e)) if e.is_cancelled() => tracing::debug!("Server task cancelled"),
                Ok(Err(e)) => tracing::error!("Server task panicked: {}", e),
                Err(_) => {
                    tracing::warn!("Server shutdown timed out (10s), aborting remaining tasks");
                    server_abort.abort();
                    if let Some(la) = listener_abort {
                        la.abort();
                    }
                }
            }
        }

        // 4. 更新配置
        {
            let mut config = self.config.write().await;
            config.current_mode = None;
        }
        self.save_config().await?;

        tracing::info!("Mode stopped, now disconnected");

        Ok(())
    }

    /// 从当前 `ClientMode::Client` 读取连接参数，销毁旧 client 并重新连接。
    ///
    /// 仅在 Client 模式下有效，复用 `start_client_mode` 的逻辑。
    /// 返回 boxed future 显式标注 `Send`，
    /// 打破 start_client_mode → spawn(do_rebuild_connection) → rebuild_client_connection → start_client_mode
    /// 的递归 opaque type 循环。
    pub fn rebuild_client_connection(
        self: &Arc<Self>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), BridgeError>> + Send + '_>>
    {
        Box::pin(async move {
            let (edge_url, message_addr) = {
                let guard = self.mode.read().await;
                match &*guard {
                    ClientMode::Client {
                        edge_url,
                        message_addr,
                        ..
                    } => (edge_url.clone(), message_addr.clone()),
                    _ => return Err(BridgeError::NotInitialized),
                }
            };

            tracing::info!(
                edge_url = %edge_url,
                message_addr = %message_addr,
                "Rebuilding client connection..."
            );

            self.start_client_mode(&edge_url, &message_addr).await
        })
    }

    /// 退出当前租户：停止服务器 → 清除当前租户选择（保留文件）
    pub async fn exit_tenant(&self) -> Result<(), BridgeError> {
        let tenant_id = {
            let tm = self.tenant_manager.read().await;
            tm.current_tenant_id().map(|s| s.to_string())
        };

        let Some(tenant_id) = tenant_id else {
            return Err(BridgeError::Config("No current tenant".to_string()));
        };

        // 1. 停止服务器模式（切换到 Disconnected）
        self.stop().await?;

        // 2. 清除当前租户选择（不删除文件）
        {
            let mut tm = self.tenant_manager.write().await;
            tm.clear_current_tenant();
        }

        // 3. 清除配置中的当前租户
        {
            let mut config = self.config.write().await;
            config.current_tenant = None;
            config.save(&self.config_path)?;
        }

        tracing::info!(tenant_id = %tenant_id, "Exited tenant (files preserved)");
        Ok(())
    }
}

// ============================================================================
// 独立重建函数（在 tokio::spawn 中使用，避免 Send 问题）
// ============================================================================

/// Client 模式连接重建（限次 + 指数退避）。
///
/// 当 `NetworkMessageClient` 的 `reconnect_loop` 耗尽所有重连尝试后，
/// bridge 层会调用此函数进行更高层级的重建：销毁旧 client，重新执行
/// `start_client_mode` 建立全新连接。
///
/// - 最多 5 次重建，每次间隔指数退避 (5s → 10s → 20s → 40s → 80s)
/// - 每次重建内部 CrabClient 会持续网络重连
/// - 全部失败后切换到 `ClientMode::Disconnected`，通知前端
pub(super) async fn do_rebuild_connection(bridge: Arc<ClientBridge>, app_handle: tauri::AppHandle) {
    const MAX_REBUILDS: u32 = 5;
    let base_delay = std::time::Duration::from_secs(5);
    let mut delay = base_delay;

    for attempt in 1..=MAX_REBUILDS {
        tracing::info!(
            attempt,
            MAX_REBUILDS,
            delay_secs = delay.as_secs(),
            "Bridge rebuild attempt"
        );

        tokio::time::sleep(delay).await;

        let result = bridge.rebuild_client_connection().await;

        match result {
            Ok(()) => {
                tracing::info!(attempt, "Bridge rebuild succeeded");
                return;
            }
            Err(e) => {
                tracing::warn!(attempt, MAX_REBUILDS, error = %e, "Bridge rebuild failed");
            }
        }

        delay *= 2;
    }

    // 全部失败：切换到 Disconnected
    tracing::error!(
        MAX_REBUILDS,
        "All bridge rebuild attempts exhausted, switching to Disconnected"
    );
    {
        let mut guard = bridge.mode.write().await;
        if let ClientMode::Client { shutdown_token, .. } = &*guard {
            shutdown_token.cancel();
        }
        *guard = ClientMode::Disconnected;
    }

    let _ = app_handle.emit("connection-state-changed", false);
    let _ = app_handle.emit("connection-permanently-lost", true);
}
