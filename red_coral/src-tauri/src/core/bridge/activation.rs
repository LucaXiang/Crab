//! Device activation and certificate detection

use super::*;

impl ClientBridge {
    /// 切换当前租户并保存配置
    ///
    /// Server 和 Client 模式都会先 stop，切换后重启对应模式
    pub async fn switch_tenant(self: &Arc<Self>, tenant_id: &str) -> Result<(), BridgeError> {
        let _lifecycle = self.lifecycle_lock.lock().await;

        // 检查当前模式，决定切换后是否重启
        let current_mode_type = {
            let mode = self.mode.read().await;
            match &*mode {
                ClientMode::Server { .. } => Some(ModeType::Server),
                ClientMode::Client { .. } => Some(ModeType::Client),
                ClientMode::Disconnected => None,
            }
        };

        let client_config_for_restart = if current_mode_type == Some(ModeType::Client) {
            self.config.read().await.client_config.clone()
        } else {
            None
        };

        // 如果在 Server 或 Client 模式，先停止（内联 stop 逻辑，lifecycle_lock 已持有）
        if current_mode_type.is_some() {
            tracing::info!("Stopping current mode to switch tenant...");
            let old_mode = {
                let mut mode_guard = self.mode.write().await;
                match &*mode_guard {
                    ClientMode::Server { shutdown_token, .. }
                    | ClientMode::Client { shutdown_token, .. } => {
                        shutdown_token.cancel();
                    }
                    ClientMode::Disconnected => {}
                }
                std::mem::replace(&mut *mode_guard, ClientMode::Disconnected)
            };
            super::lifecycle::await_mode_shutdown(old_mode).await;

            let mut config = self.config.write().await;
            config.current_mode = None;
            config.save(&self.config_path)?;
        }

        // 1. 切换内存状态
        {
            let mut tm = self.tenant_manager.write().await;
            tm.switch_tenant(tenant_id)?;
        }

        // 2. 更新并保存配置
        {
            let mut config = self.config.write().await;
            config.current_tenant = Some(tenant_id.to_string());
            config.save(&self.config_path)?;
        }

        // 3. 重启对应模式（释放 lifecycle_lock 前，因为 start 方法也会获取）
        drop(_lifecycle);

        match current_mode_type {
            Some(ModeType::Server) => {
                tracing::info!("Restarting server with new tenant...");
                self.start_server_mode().await?;
            }
            Some(ModeType::Client) => {
                if let Some(cfg) = client_config_for_restart {
                    tracing::info!("Restarting client with new tenant...");
                    self.start_client_mode(&cfg.edge_url, &cfg.message_addr)
                        .await?;
                } else {
                    tracing::warn!(
                        "Client mode was active but no client_config found, staying disconnected"
                    );
                }
            }
            None => {}
        }

        tracing::info!(tenant_id = %tenant_id, "Switched tenant and saved config");
        Ok(())
    }

    /// 激活设备并保存证书
    ///
    /// 仅保存证书到磁盘，不启动任何模式。
    /// 自动通过 refresh_token 获取 JWT，无需前端传入 token。
    /// 返回 (tenant_id, subscription_status)，前端据此决定下一步。
    pub async fn handle_activation(&self) -> Result<(String, Option<String>), BridgeError> {
        self.handle_activation_with_replace(None).await
    }

    /// 激活设备（支持替换已有设备）
    ///
    /// 自动通过 refresh_token 获取 JWT，无需前端传入 token。
    pub async fn handle_activation_with_replace(
        &self,
        replace_entity_id: Option<&str>,
    ) -> Result<(String, Option<String>), BridgeError> {
        let token = self.get_fresh_token().await?;
        let auth_url = self.get_auth_url().await;
        // 1. 调用 TenantManager 激活（保存证书和 credential 到磁盘）
        let (tenant_id, entity_id) = {
            let mut tm = self.tenant_manager.write().await;
            tm.activate_device(&auth_url, &token, replace_entity_id)
                .await?
        };

        // 2. 更新已知租户列表、当前租户、entity_id
        {
            let mut config = self.config.write().await;
            if !config.known_tenants.contains(&tenant_id) {
                config.known_tenants.push(tenant_id.clone());
            }
            config.current_tenant = Some(tenant_id.clone());
            config.active_entity_id = Some(entity_id.clone());
            config.save(&self.config_path)?;
        }

        // 3. 读取订阅状态（从刚保存的 credential）
        let subscription_status = {
            let tm = self.tenant_manager.read().await;
            tm.get_subscription_status(&tenant_id)
        };

        tracing::info!(tenant_id = %tenant_id, entity_id = %entity_id, ?subscription_status, "Device activated and config saved (mode not started)");
        Ok((tenant_id, subscription_status))
    }

    /// 激活客户端并保存证书
    ///
    /// 仅保存客户端证书到磁盘，不启动任何模式。
    /// 自动通过 refresh_token 获取 JWT，无需前端传入 token。
    /// 返回 (tenant_id, subscription_status)，前端据此决定下一步。
    pub async fn handle_client_activation(&self) -> Result<(String, Option<String>), BridgeError> {
        self.handle_client_activation_with_replace(None).await
    }

    /// 激活客户端（支持替换已有客户端）
    ///
    /// 自动通过 refresh_token 获取 JWT，无需前端传入 token。
    pub async fn handle_client_activation_with_replace(
        &self,
        replace_entity_id: Option<&str>,
    ) -> Result<(String, Option<String>), BridgeError> {
        let token = self.get_fresh_token().await?;
        let auth_url = self.get_auth_url().await;
        // 1. 调用 TenantManager 客户端激活
        let (tenant_id, entity_id) = {
            let mut tm = self.tenant_manager.write().await;
            tm.activate_client(&auth_url, &token, replace_entity_id)
                .await?
        };

        // 2. 更新已知租户列表、当前租户、entity_id
        {
            let mut config = self.config.write().await;
            if !config.known_tenants.contains(&tenant_id) {
                config.known_tenants.push(tenant_id.clone());
            }
            config.current_tenant = Some(tenant_id.clone());
            config.active_entity_id = Some(entity_id.clone());
            config.save(&self.config_path)?;
        }

        // 3. 读取订阅状态
        let subscription_status = {
            let tm = self.tenant_manager.read().await;
            tm.get_subscription_status(&tenant_id)
        };

        tracing::info!(tenant_id = %tenant_id, entity_id = %entity_id, ?subscription_status, "Client activated and config saved");
        Ok((tenant_id, subscription_status))
    }

    /// 验证租户凭据 (不签发证书)
    pub async fn verify_tenant(
        &self,
        username: &str,
        password: &str,
    ) -> Result<shared::activation::TenantVerifyData, BridgeError> {
        let auth_url = self.get_auth_url().await;
        let tm = self.tenant_manager.read().await;
        let data = tm.verify_tenant(&auth_url, username, password).await?;

        // 验证成功后，确保租户目录存在并切换
        drop(tm);
        let tenant_id = data.tenant_id.clone();
        {
            let mut tm = self.tenant_manager.write().await;
            let tenant_path = tm.base_path().join(&tenant_id);
            if !tenant_path.exists() {
                let paths = super::super::paths::TenantPaths::new(&tenant_path);
                paths.ensure_common_dirs()?;
                tm.load_existing_tenants()?;
            }
            if tm.current_tenant_id() != Some(&tenant_id) {
                tm.switch_tenant(&tenant_id)?;
            }
        }

        // 更新配置（包括保存 refresh_token 以便后续无密码操作）
        {
            let mut config = self.config.write().await;
            if !config.known_tenants.contains(&tenant_id) {
                config.known_tenants.push(tenant_id.clone());
            }
            config.current_tenant = Some(tenant_id);
            config.refresh_token = Some(data.refresh_token.clone());
            config.save(&self.config_path)?;
        }

        Ok(data)
    }

    /// 注销当前模式 (释放配额 + 删除本地证书)
    ///
    /// 内部自动通过 refresh_token 获取 JWT，无需前端传入 token。
    /// 远端注销成功后，stop() 和 delete_certs() 失败只 warn 不 return Err（best-effort）。
    pub async fn deactivate_current_mode(&self) -> Result<(), BridgeError> {
        let _lifecycle = self.lifecycle_lock.lock().await;

        let token = self.get_fresh_token().await?;
        let auth_url = self.get_auth_url().await;

        // 读取当前 entity_id 和模式
        let (entity_id, current_mode) = {
            let config = self.config.read().await;
            (config.active_entity_id.clone(), config.current_mode)
        };

        let entity_id = entity_id
            .ok_or_else(|| BridgeError::Config("No active entity_id to deactivate".to_string()))?;

        // 1. 先调用 auth-server 注销 (确保远端成功再清理本地)
        {
            let tm = self.tenant_manager.read().await;
            match current_mode {
                Some(ModeType::Server) => {
                    tm.deactivate_server(&auth_url, &token, &entity_id).await?;
                }
                Some(ModeType::Client) => {
                    tm.deactivate_client(&auth_url, &token, &entity_id).await?;
                }
                None => {
                    return Err(BridgeError::Config("No mode to deactivate".to_string()));
                }
            }
        }

        // 2. 停止当前模式 (best-effort: 远端已成功，本地失败只 warn)
        {
            let old_mode = {
                let mut mode_guard = self.mode.write().await;
                match &*mode_guard {
                    ClientMode::Server { shutdown_token, .. }
                    | ClientMode::Client { shutdown_token, .. } => {
                        shutdown_token.cancel();
                    }
                    ClientMode::Disconnected => {}
                }
                std::mem::replace(&mut *mode_guard, ClientMode::Disconnected)
            };
            super::lifecycle::await_mode_shutdown(old_mode).await;
        }

        // 3. 删除本地证书 (best-effort)
        {
            let tm = self.tenant_manager.read().await;
            let cert_result = match current_mode {
                Some(ModeType::Server) => tm.delete_server_certs(),
                Some(ModeType::Client) => tm.delete_client_certs(),
                None => Ok(()),
            };
            if let Err(e) = cert_result {
                tracing::warn!("Failed to delete certificates (best-effort): {}", e);
            }
        }

        // 4. 清除配置中的 entity_id 和模式（保留 ? — 配置保存失败是真正的错误）
        {
            let mut config = self.config.write().await;
            config.active_entity_id = None;
            config.current_mode = None;
            config.save(&self.config_path)?;
        }

        tracing::info!("Deactivated current mode and cleaned up certificates");
        Ok(())
    }

    /// 从 edge-server 检测需要激活的具体原因
    pub(super) async fn detect_activation_reason_from_server(
        &self,
        server_state: &edge_server::ServerState,
        tenant_manager: &TenantManager,
    ) -> ActivationRequiredReason {
        // 尝试调用 edge-server 的自检获取具体错误
        let cert_service = server_state.cert_service();
        let credential = server_state
            .activation_service()
            .get_credential()
            .await
            .ok()
            .flatten();

        match cert_service
            .self_check_with_binding(credential.as_ref())
            .await
        {
            Ok(()) => {
                // 自检通过但未激活，说明 Credential.json 不存在
                ActivationRequiredReason::FirstTimeSetup
            }
            Err(e) => self.parse_activation_error(&e.to_string(), tenant_manager),
        }
    }

    /// 解析激活错误消息
    pub(super) fn parse_activation_error(
        &self,
        error_str: &str,
        tenant_manager: &TenantManager,
    ) -> ActivationRequiredReason {
        let error_lower = error_str.to_lowercase();

        if error_lower.contains("expired") {
            // 证书过期
            if let Some(paths) = tenant_manager.current_paths() {
                if let Ok(cert_pem) = std::fs::read_to_string(paths.edge_cert()) {
                    if let Ok(metadata) = crab_cert::CertMetadata::from_pem(&cert_pem) {
                        let now = time::OffsetDateTime::now_utc();
                        let duration = metadata.not_after - now;
                        let days_overdue = -duration.whole_days();
                        let expired_at_millis = metadata.not_after.unix_timestamp() * 1000
                            + metadata.not_after.millisecond() as i64;
                        return ActivationRequiredReason::CertificateExpired {
                            expired_at: expired_at_millis,
                            days_overdue,
                        };
                    }
                }
            }
            ActivationRequiredReason::CertificateExpired {
                expired_at: 0,
                days_overdue: 0,
            }
        } else if error_lower.contains("hardware id mismatch")
            || error_lower.contains("device id mismatch")
            || error_lower.contains("device_id")
        {
            // 设备 ID 不匹配
            let (expected, actual) = self.extract_device_ids(error_str);
            ActivationRequiredReason::DeviceMismatch { expected, actual }
        } else if error_lower.contains("clock")
            || (error_lower.contains("time") && error_lower.contains("tamper"))
        {
            // 时钟篡改
            let direction = if error_lower.contains("backward") {
                ClockDirection::Backward
            } else {
                ClockDirection::Forward
            };
            let drift_seconds = error_str
                .split_whitespace()
                .find_map(|s| s.parse::<i64>().ok())
                .unwrap_or(0);
            ActivationRequiredReason::ClockTampering {
                direction,
                drift_seconds,
                last_verified_at: 0,
            }
        } else if error_lower.contains("signature") {
            // 签名无效
            ActivationRequiredReason::SignatureInvalid {
                component: "credential".to_string(),
                error: error_str.to_string(),
            }
        } else if error_lower.contains("chain")
            || (error_lower.contains("certificate") && error_lower.contains("invalid"))
        {
            // 证书链无效
            ActivationRequiredReason::CertificateInvalid {
                error: error_str.to_string(),
            }
        } else if error_lower.contains("not found") || error_lower.contains("missing") {
            // 文件缺失
            ActivationRequiredReason::FirstTimeSetup
        } else {
            // 未知错误，返回通用的绑定无效
            ActivationRequiredReason::BindingInvalid {
                error: error_str.to_string(),
            }
        }
    }

    /// 从错误消息中提取设备 ID
    fn extract_device_ids(&self, error_str: &str) -> (String, String) {
        // 尝试解析格式如 "expected xxx, got yyy" 或类似格式
        if let Some(idx) = error_str.find("expected ") {
            let rest = &error_str[idx + 9..];
            if let Some(comma_idx) = rest.find(", ") {
                let exp = rest[..comma_idx].trim().to_string();
                let act_start = rest.find("got ").map(|i| i + 4).unwrap_or(comma_idx + 2);
                let act_end = rest[act_start..]
                    .find(|c: char| !c.is_alphanumeric() && c != '-')
                    .unwrap_or(rest.len() - act_start);
                let act = rest[act_start..act_start + act_end].trim().to_string();
                return (exp, act);
            }
        }
        // 无法解析，返回掩码值
        (
            "***".to_string(),
            crab_cert::generate_hardware_id()[..8].to_string(),
        )
    }

    /// 检测需要激活的具体原因 (基于 TenantPaths)
    ///
    /// Server 模式: 检查 server/certs/ 下的证书
    /// Client 模式: 检查 certs/ 下的证书
    pub(super) fn detect_activation_reason(
        &self,
        tenant_manager: &TenantManager,
        for_server: bool,
    ) -> ActivationRequiredReason {
        // 1. 检查是否有路径管理器
        let paths = match tenant_manager.current_paths() {
            Some(p) => p,
            None => return ActivationRequiredReason::FirstTimeSetup,
        };

        // 2. 检查证书是否存在
        let has_certs = if for_server {
            paths.has_server_certificates()
        } else {
            paths.has_client_certificates()
        };

        if !has_certs {
            return ActivationRequiredReason::FirstTimeSetup;
        }

        // 3. 读取证书检查有效性
        let cert_path = if for_server {
            paths.edge_cert()
        } else {
            paths.client_cert()
        };

        let cert_pem = match std::fs::read_to_string(&cert_path) {
            Ok(pem) => pem,
            Err(_) => {
                return ActivationRequiredReason::CertificateInvalid {
                    error: "Cannot read certificate file".to_string(),
                }
            }
        };

        let metadata = match crab_cert::CertMetadata::from_pem(&cert_pem) {
            Ok(m) => m,
            Err(e) => {
                return ActivationRequiredReason::CertificateInvalid {
                    error: format!("Invalid certificate: {}", e),
                }
            }
        };

        // 4. 检查证书过期
        let now = time::OffsetDateTime::now_utc();
        let duration = metadata.not_after - now;
        let days_remaining = duration.whole_days();
        let not_after_millis =
            metadata.not_after.unix_timestamp() * 1000 + metadata.not_after.millisecond() as i64;

        if days_remaining < 0 {
            return ActivationRequiredReason::CertificateExpired {
                expired_at: not_after_millis,
                days_overdue: -days_remaining,
            };
        }

        if days_remaining <= 30 {
            return ActivationRequiredReason::CertificateExpiringSoon {
                expires_at: not_after_millis,
                days_remaining,
            };
        }

        // 5. 检查设备 ID 绑定
        let current_device_id = crab_cert::generate_hardware_id();
        if let Some(cert_device_id) = &metadata.device_id {
            if cert_device_id != &current_device_id {
                return ActivationRequiredReason::DeviceMismatch {
                    expected: cert_device_id[..8].to_string(),
                    actual: current_device_id[..8].to_string(),
                };
            }
        }

        // 证书有效，可能是其他原因或需要检查 credential
        ActivationRequiredReason::FirstTimeSetup
    }
}
