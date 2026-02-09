//! State queries and health checks

use super::*;

impl ClientBridge {
    /// 获取当前模式信息
    pub async fn get_mode_info(&self) -> ModeInfo {
        let mode_guard = self.mode.read().await;
        let tenant_manager = self.tenant_manager.read().await;

        let (mode, is_connected, is_authenticated, client_check_info) = match &*mode_guard {
            ClientMode::Disconnected => (ModeType::Disconnected, false, false, None),
            ClientMode::Server { client, .. } => {
                let is_auth = matches!(client, Some(LocalClientState::Authenticated(_)));
                (ModeType::Server, true, is_auth, None)
            }
            ClientMode::Client {
                client, edge_url, ..
            } => {
                let is_auth = matches!(client, Some(RemoteClientState::Authenticated(_)));
                let check_info = if let Some(state) = client {
                    let http = match state {
                        RemoteClientState::Connected(c) => c.edge_http_client().cloned(),
                        RemoteClientState::Authenticated(c) => c.edge_http_client().cloned(),
                    };
                    Some((edge_url.clone(), http))
                } else {
                    None
                };
                (ModeType::Client, false, is_auth, check_info)
            }
        };

        drop(mode_guard);

        // Perform real network health check for Client mode
        let final_is_connected = if mode == ModeType::Client {
            if let Some((url, Some(http))) = client_check_info {
                match http
                    .get(format!("{}/health", url))
                    .timeout(std::time::Duration::from_secs(2))
                    .send()
                    .await
                {
                    Ok(resp) => resp.status().is_success(),
                    Err(e) => {
                        tracing::warn!("Health check failed: {}", e);
                        false
                    }
                }
            } else {
                false
            }
        } else {
            is_connected
        };

        ModeInfo {
            mode,
            is_connected: final_is_connected,
            is_authenticated,
            tenant_id: tenant_manager.current_tenant_id().map(|s| s.to_string()),
            username: tenant_manager.current_session().map(|s| s.username.clone()),
        }
    }

    /// 获取应用状态 (用于前端路由守卫)
    pub async fn get_app_state(&self) -> AppState {
        let mode_guard = self.mode.read().await;
        let tenant_manager = self.tenant_manager.read().await;

        match &*mode_guard {
            ClientMode::Disconnected => {
                if tenant_manager.current_tenant_id().is_none() {
                    AppState::ServerNoTenant
                } else {
                    let has_certs = tenant_manager
                        .current_paths()
                        .map(|p| p.is_server_activated())
                        .unwrap_or(false);

                    if has_certs {
                        AppState::Uninitialized
                    } else {
                        let reason = ActivationRequiredReason::FirstTimeSetup;
                        AppState::ServerNeedActivation {
                            can_auto_recover: reason.can_auto_recover(),
                            recovery_hint: reason.recovery_hint_code().to_string(),
                            reason,
                        }
                    }
                }
            }

            ClientMode::Server {
                server_state,
                client,
                ..
            } => {
                // 1. 首先检查 edge-server 激活状态 (权威)
                let is_activated = server_state.is_activated().await;

                if !is_activated {
                    // 调用 edge-server 自检获取具体错误
                    let reason = self.detect_activation_reason_from_server(server_state, &tenant_manager).await;
                    return AppState::ServerNeedActivation {
                        can_auto_recover: reason.can_auto_recover(),
                        recovery_hint: reason.recovery_hint_code().to_string(),
                        reason,
                    };
                }

                let credential = server_state
                    .activation_service()
                    .get_credential()
                    .await
                    .ok()
                    .flatten();

                if let Some(_cred) = credential {
                    // 订阅阻止检查 (统一使用 edge-server 判断，包含签名陈旧检查)
                    let blocked_info = server_state.get_subscription_blocked_info().await;
                    if let Some(info) = blocked_info {
                        AppState::ServerSubscriptionBlocked { info }
                    } else {
                        // 2. 检查员工登录状态
                        // 优先检查 CrabClient 状态（权威）
                        if matches!(client, Some(LocalClientState::Authenticated(_))) {
                            return AppState::ServerAuthenticated;
                        }
                        // 其次检查内存中的 session
                        if tenant_manager.current_session().is_some() {
                            return AppState::ServerAuthenticated;
                        }
                        // 未登录
                        AppState::ServerReady
                    }
                } else {
                    // 无 credential，需要激活
                    let reason = self.detect_activation_reason(&tenant_manager, true); // Server mode
                    AppState::ServerNeedActivation {
                        can_auto_recover: reason.can_auto_recover(),
                        recovery_hint: reason.recovery_hint_code().to_string(),
                        reason,
                    }
                }
            }

            ClientMode::Client { client, .. } => match client {
                Some(RemoteClientState::Authenticated(_)) => AppState::ClientAuthenticated,
                Some(RemoteClientState::Connected(_)) => AppState::ClientConnected,
                None => {
                    let has_certs = tenant_manager
                        .current_paths()
                        .map(|p| p.has_client_certificates())
                        .unwrap_or(false);

                    if has_certs {
                        AppState::ClientDisconnected
                    } else {
                        AppState::ClientNeedSetup
                    }
                }
            },
        }
    }

    /// 获取当前活动会话 (用于启动时恢复登录状态)
    pub async fn get_current_session(&self) -> Option<super::super::session_cache::EmployeeSession> {
        let tenant_manager = self.tenant_manager.read().await;
        tenant_manager.current_session().cloned()
    }

    /// 重新检查订阅状态
    ///
    /// 在 Server 模式下，调用 edge-server 的 sync_subscription 从 auth-server 拉取最新订阅，
    /// 然后返回最新的 AppState。
    pub async fn check_subscription(&self) -> Result<AppState, BridgeError> {
        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { server_state, .. } => {
                // 从 auth-server 同步最新订阅状态
                server_state.sync_subscription().await;
                tracing::info!("Subscription re-checked from auth-server");
            }
            _ => {
                tracing::warn!("check_subscription called in non-Server mode, skipping sync");
            }
        }

        // 释放 mode_guard 以避免死锁（get_app_state 也需要读锁）
        drop(mode_guard);

        // 返回最新的 AppState
        Ok(self.get_app_state().await)
    }

    /// 获取健康检查组件 (订阅、网络、数据库)
    pub async fn get_health_components(
        &self,
    ) -> (
        shared::app_state::SubscriptionHealth,
        shared::app_state::NetworkHealth,
        shared::app_state::DatabaseHealth,
    ) {
        use shared::app_state::{DatabaseHealth, HealthLevel, NetworkHealth, SubscriptionHealth};

        let mode_guard = self.mode.read().await;

        match &*mode_guard {
            ClientMode::Server { server_state, .. } => {
                // === 订阅健康状态 ===
                let subscription = match server_state.activation_service().get_credential().await {
                    Ok(Some(cred)) => {
                        if let Some(sub) = &cred.subscription {
                            let status = match sub.status {
                                SubscriptionStatus::Active => HealthLevel::Healthy,
                                SubscriptionStatus::PastDue => HealthLevel::Warning,
                                SubscriptionStatus::Expired | SubscriptionStatus::Canceled => {
                                    HealthLevel::Critical
                                }
                                SubscriptionStatus::Inactive | SubscriptionStatus::Unpaid => {
                                    HealthLevel::Critical
                                }
                            };
                            let needs_refresh = sub.is_signature_expired();
                            SubscriptionHealth {
                                status: if needs_refresh {
                                    HealthLevel::Warning
                                } else {
                                    status
                                },
                                plan: Some(format!("{:?}", sub.plan)),
                                subscription_status: Some(format!("{:?}", sub.status)),
                                signature_valid_until: sub.signature_valid_until,
                                needs_refresh,
                            }
                        } else {
                            SubscriptionHealth {
                                status: HealthLevel::Unknown,
                                plan: None,
                                subscription_status: None,
                                signature_valid_until: 0,
                                needs_refresh: false,
                            }
                        }
                    }
                    _ => SubscriptionHealth {
                        status: HealthLevel::Unknown,
                        plan: None,
                        subscription_status: None,
                        signature_valid_until: 0,
                        needs_refresh: false,
                    },
                };

                // === 网络健康状态 ===
                // 尝试连接 auth server 检查可达性
                let network = {
                    let auth_url = std::env::var("AUTH_SERVER_URL")
                        .unwrap_or_else(|_| "https://localhost:3001".to_string());
                    let client = reqwest::Client::builder()
                        .danger_accept_invalid_certs(true) // 开发环境
                        .timeout(std::time::Duration::from_secs(3))
                        .build();

                    let (reachable, last_connected) = match client {
                        Ok(c) => {
                            match c.get(format!("{}/health", auth_url)).send().await {
                                Ok(resp) if resp.status().is_success() => {
                                    (true, Some(shared::util::now_millis()))
                                }
                                _ => (false, None),
                            }
                        }
                        Err(_) => (false, None),
                    };

                    NetworkHealth {
                        status: if reachable {
                            HealthLevel::Healthy
                        } else {
                            HealthLevel::Warning
                        },
                        auth_server_reachable: reachable,
                        last_connected_at: last_connected,
                    }
                };

                // === 数据库健康状态 ===
                let database = {
                    // 尝试执行简单查询检查数据库是否正常
                    let db_ok: bool = server_state.pool.acquire().await.is_ok();

                    DatabaseHealth {
                        status: if db_ok {
                            HealthLevel::Healthy
                        } else {
                            HealthLevel::Critical
                        },
                        size_bytes: None,
                        last_write_at: None,
                    }
                };

                (subscription, network, database)
            }

            ClientMode::Client { client, edge_url, .. } => {
                // Client 模式: 检查与 edge server 的连接
                let (network_status, reachable) = if let Some(state) = client {
                    let http = match state {
                        RemoteClientState::Connected(c) => c.edge_http_client().cloned(),
                        RemoteClientState::Authenticated(c) => c.edge_http_client().cloned(),
                    };
                    if let Some(http) = http {
                        match http
                            .get(format!("{}/health", edge_url))
                            .timeout(std::time::Duration::from_secs(2))
                            .send()
                            .await
                        {
                            Ok(resp) if resp.status().is_success() => (HealthLevel::Healthy, true),
                            _ => (HealthLevel::Warning, false),
                        }
                    } else {
                        (HealthLevel::Unknown, false)
                    }
                } else {
                    (HealthLevel::Critical, false)
                };

                let subscription = SubscriptionHealth {
                    status: HealthLevel::Unknown, // Client 模式不直接访问订阅
                    plan: None,
                    subscription_status: None,
                    signature_valid_until: 0,
                    needs_refresh: false,
                };

                let network = NetworkHealth {
                    status: network_status,
                    auth_server_reachable: reachable,
                    last_connected_at: if reachable {
                        Some(shared::util::now_millis())
                    } else {
                        None
                    },
                };

                let database = DatabaseHealth {
                    status: HealthLevel::Unknown, // Client 模式不直接访问数据库
                    size_bytes: None,
                    last_write_at: None,
                };

                (subscription, network, database)
            }

            ClientMode::Disconnected => {
                let subscription = SubscriptionHealth {
                    status: HealthLevel::Unknown,
                    plan: None,
                    subscription_status: None,
                    signature_valid_until: 0,
                    needs_refresh: false,
                };

                let network = NetworkHealth {
                    status: HealthLevel::Critical,
                    auth_server_reachable: false,
                    last_connected_at: None,
                };

                let database = DatabaseHealth {
                    status: HealthLevel::Unknown,
                    size_bytes: None,
                    last_write_at: None,
                };

                (subscription, network, database)
            }
        }
    }
}
