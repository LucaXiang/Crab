//! Employee login and logout

use super::*;

impl ClientBridge {
    /// 员工登录 (使用 CrabClient)
    pub async fn login_employee(
        &self,
        username: &str,
        password: &str,
    ) -> Result<super::super::session_cache::EmployeeSession, BridgeError> {
        let mut mode_guard = self.mode.write().await;

        let result = match &mut *mode_guard {
            ClientMode::Server {
                server_state: _,
                client,
                ..
            } => {
                let current_client = client.take().ok_or(BridgeError::NotInitialized)?;

                match current_client {
                    LocalClientState::Connected(connected) => {
                        match connected.login(username, password).await {
                            Ok(authenticated) => {
                                // Restore state FIRST to prevent state loss
                                // if me()/token() returns None
                                *client = Some(LocalClientState::Authenticated(authenticated));
                                let session = extract_local_session(client, username)?;
                                tracing::debug!(username = %username, "Employee logged in via CrabClient (local)");
                                Ok(session)
                            }
                            Err((e, connected)) => {
                                *client = Some(LocalClientState::Connected(connected));
                                Err(BridgeError::Client(e))
                            }
                        }
                    }
                    LocalClientState::Authenticated(auth) => {
                        let connected = auth.logout().await;
                        match connected.login(username, password).await {
                            Ok(authenticated) => {
                                *client = Some(LocalClientState::Authenticated(authenticated));
                                let session = extract_local_session(client, username)?;
                                tracing::debug!(username = %username, "Employee re-logged in via CrabClient (local)");
                                Ok(session)
                            }
                            Err((e, connected)) => {
                                *client = Some(LocalClientState::Connected(connected));
                                Err(BridgeError::Client(e))
                            }
                        }
                    }
                }
            }
            ClientMode::Client { client, .. } => {
                let current_client = client.take().ok_or(BridgeError::NotInitialized)?;

                match current_client {
                    RemoteClientState::Connected(connected) => {
                        match connected.login(username, password).await {
                            Ok(authenticated) => {
                                *client = Some(RemoteClientState::Authenticated(authenticated));
                                let session = extract_remote_session(client, username)?;
                                tracing::debug!(username = %username, "Employee logged in via CrabClient (remote)");
                                Ok(session)
                            }
                            Err((e, connected)) => {
                                *client = Some(RemoteClientState::Connected(connected));
                                Err(BridgeError::Client(e))
                            }
                        }
                    }
                    RemoteClientState::Authenticated(auth) => {
                        let connected = auth.logout().await;
                        match connected.login(username, password).await {
                            Ok(authenticated) => {
                                *client = Some(RemoteClientState::Authenticated(authenticated));
                                let session = extract_remote_session(client, username)?;
                                tracing::debug!(username = %username, "Employee re-logged in via CrabClient (remote)");
                                Ok(session)
                            }
                            Err((e, connected)) => {
                                *client = Some(RemoteClientState::Connected(connected));
                                Err(BridgeError::Client(e))
                            }
                        }
                    }
                }
            }
            ClientMode::Disconnected => Err(BridgeError::NotInitialized),
        };

        drop(mode_guard);

        if let Ok(ref session) = result {
            // 1. 保存到磁盘
            {
                let tenant_manager = self.tenant_manager.read().await;
                if let Err(e) = tenant_manager.save_current_session(session) {
                    tracing::warn!("Failed to persist session: {}", e);
                }
            }
            // 2. 更新内存中的 current_session
            {
                let mut tenant_manager = self.tenant_manager.write().await;
                tenant_manager.set_current_session(session.clone());
            }
        }

        result
    }

    /// 员工登出
    pub async fn logout_employee(&self) -> Result<(), BridgeError> {
        let mut mode_guard = self.mode.write().await;

        match &mut *mode_guard {
            ClientMode::Server {
                server_state: _,
                client,
                ..
            } => {
                if let Some(current_client) = client.take() {
                    match current_client {
                        LocalClientState::Authenticated(auth) => {
                            let connected = auth.logout().await;
                            *client = Some(LocalClientState::Connected(connected));
                            tracing::debug!("Employee logged out (local)");
                        }
                        LocalClientState::Connected(c) => {
                            *client = Some(LocalClientState::Connected(c));
                        }
                    }
                }
            }
            ClientMode::Client { client, .. } => {
                if let Some(current_client) = client.take() {
                    match current_client {
                        RemoteClientState::Authenticated(auth) => {
                            let connected = auth.logout().await;
                            *client = Some(RemoteClientState::Connected(connected));
                            tracing::debug!("Employee logged out (remote)");
                        }
                        RemoteClientState::Connected(c) => {
                            *client = Some(RemoteClientState::Connected(c));
                        }
                    }
                }
            }
            ClientMode::Disconnected => {}
        }

        drop(mode_guard);

        let tenant_manager = self.tenant_manager.read().await;
        if let Err(e) = tenant_manager.clear_current_session() {
            tracing::warn!("Failed to clear cached session: {}", e);
        }

        Ok(())
    }
}

/// Extract session from already-restored local client state.
/// Client is guaranteed to be in Authenticated state when called.
fn extract_local_session(
    client: &Option<LocalClientState>,
    username: &str,
) -> Result<super::super::session_cache::EmployeeSession, BridgeError> {
    let auth_ref = match client.as_ref() {
        Some(LocalClientState::Authenticated(a)) => a,
        _ => unreachable!("client must be Authenticated when extract_local_session is called"),
    };

    let user_info = auth_ref.me().cloned().ok_or_else(|| {
        BridgeError::Client(crab_client::ClientError::Auth(
            "No user info after login".into(),
        ))
    })?;
    let token = auth_ref
        .token()
        .ok_or_else(|| {
            BridgeError::Client(crab_client::ClientError::Auth(
                "No token received after login".into(),
            ))
        })?
        .to_string();
    let expires_at = super::super::session_cache::EmployeeSession::parse_jwt_exp(&token);

    Ok(super::super::session_cache::EmployeeSession {
        username: username.to_string(),
        token,
        user_info,
        login_mode: super::super::session_cache::LoginMode::Online,
        expires_at,
        logged_in_at: shared::util::now_millis(),
    })
}

/// Extract session from already-restored remote client state.
/// Client is guaranteed to be in Authenticated state when called.
fn extract_remote_session(
    client: &Option<RemoteClientState>,
    username: &str,
) -> Result<super::super::session_cache::EmployeeSession, BridgeError> {
    let auth_ref = match client.as_ref() {
        Some(RemoteClientState::Authenticated(a)) => a,
        _ => unreachable!("client must be Authenticated when extract_remote_session is called"),
    };

    let user_info = auth_ref.me().cloned().ok_or_else(|| {
        BridgeError::Client(crab_client::ClientError::Auth(
            "No user info after login".into(),
        ))
    })?;
    let token = auth_ref
        .token()
        .ok_or_else(|| {
            BridgeError::Client(crab_client::ClientError::Auth(
                "No token received after login".into(),
            ))
        })?
        .to_string();
    let expires_at = super::super::session_cache::EmployeeSession::parse_jwt_exp(&token);

    Ok(super::super::session_cache::EmployeeSession {
        username: username.to_string(),
        token,
        user_info,
        login_mode: super::super::session_cache::LoginMode::Online,
        expires_at,
        logged_in_at: shared::util::now_millis(),
    })
}
