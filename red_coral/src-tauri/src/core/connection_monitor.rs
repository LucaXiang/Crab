//! Connection Monitor - 连接状态监控和自动重连
//!
//! 定期检查连接状态，在 Client 模式下自动发送连接状态事件。
//! 支持自动重连，最多重试 3 次。

use std::sync::Arc;
use std::time::Duration;
use tauri::Emitter;
use tokio::sync::RwLock;
use tokio::time::interval;

use super::client_bridge::{ClientBridge, ModeType};

/// 重连配置
const MAX_RETRY_ATTEMPTS: u32 = 3;
const RETRY_DELAY_MS: u64 = 5000;

/// 连接监控器
///
/// 定期检查连接状态，在 Client 模式下自动发送连接状态事件。
/// 支持自动重连，最多重试 MAX_RETRY_ATTEMPTS 次。
pub struct ConnectionMonitor {
    bridge: Arc<RwLock<ClientBridge>>,
    check_interval: Duration,
}

impl ConnectionMonitor {
    /// 创建新的连接监控器
    ///
    /// # Arguments
    /// * `bridge` - ClientBridge 的共享引用
    /// * `check_interval` - 检查间隔时间
    pub fn new(bridge: Arc<RwLock<ClientBridge>>, check_interval: Duration) -> Self {
        Self {
            bridge,
            check_interval,
        }
    }

    /// 启动监控循环
    ///
    /// 该方法会在后台持续运行，定期检查连接状态。
    /// 在 Client 模式下，如果检测到断开连接，会尝试自动重连，
    /// 并发送 Tauri 事件通知前端。
    ///
    /// # Arguments
    /// * `app_handle` - Tauri AppHandle，用于发送事件
    pub async fn start(self, app_handle: tauri::AppHandle) {
        let mut ticker = interval(self.check_interval);
        let mut consecutive_failures = 0u32;

        loop {
            ticker.tick().await;

            let mode_info = {
                let bridge = self.bridge.read().await;
                bridge.get_mode_info().await
            };

            // 只在 Client 模式下进行检查
            if mode_info.mode != ModeType::Client {
                consecutive_failures = 0;
                continue;
            }

            if !mode_info.is_connected {
                consecutive_failures += 1;
                tracing::warn!(
                    "Connection lost (attempt {}/{})",
                    consecutive_failures,
                    MAX_RETRY_ATTEMPTS
                );

                // 发送断开事件
                let _ = app_handle.emit(
                    "connection-status",
                    serde_json::json!({
                        "connected": false,
                        "reconnecting": true,
                        "attempt": consecutive_failures,
                    }),
                );

                if consecutive_failures <= MAX_RETRY_ATTEMPTS {
                    // 尝试重连
                    tokio::time::sleep(Duration::from_millis(RETRY_DELAY_MS)).await;

                    let bridge = self.bridge.read().await;
                    match bridge.reconnect_client().await {
                        Ok(()) => {
                            tracing::info!("Reconnected successfully");
                            consecutive_failures = 0;
                            let _ = app_handle.emit(
                                "connection-status",
                                serde_json::json!({
                                    "connected": true,
                                    "reconnecting": false,
                                }),
                            );
                        }
                        Err(e) => {
                            tracing::error!("Reconnect failed: {}", e);
                        }
                    }
                } else {
                    // 超过重试次数，通知前端
                    let _ = app_handle.emit(
                        "connection-status",
                        serde_json::json!({
                            "connected": false,
                            "reconnecting": false,
                            "error": "Max retry attempts exceeded",
                        }),
                    );
                }
            } else {
                // 连接正常，重置计数器
                if consecutive_failures > 0 {
                    consecutive_failures = 0;
                    let _ = app_handle.emit(
                        "connection-status",
                        serde_json::json!({
                            "connected": true,
                            "reconnecting": false,
                        }),
                    );
                }
            }
        }
    }
}
