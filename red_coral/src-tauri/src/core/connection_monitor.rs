//! Connection Monitor - 连接状态监控和自动重连
//!
//! 定期检查连接状态，在 Client 模式下自动发送连接状态事件。
//! 为未来的自动重连功能提供基础设施。

use std::sync::Arc;
use std::time::Duration;
use tauri::Emitter;
use tokio::sync::RwLock;
use tokio::time::interval;

use super::client_bridge::{ClientBridge, ModeType};

/// 连接监控器
///
/// 定期检查连接状态，在 Client 模式下自动发送连接状态事件。
/// 为未来的自动重连功能提供基础设施。
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
    /// 在 Client 模式下，如果检测到断开连接，会发送 Tauri 事件通知前端。
    ///
    /// # Arguments
    /// * `app_handle` - Tauri AppHandle，用于发送事件
    pub async fn start(self, app_handle: tauri::AppHandle) {
        let mut ticker = interval(self.check_interval);

        loop {
            ticker.tick().await;

            let bridge = self.bridge.read().await;
            let mode_info = bridge.get_mode_info().await;

            // 只在 Client 模式下进行检查
            if mode_info.mode == ModeType::Client && !mode_info.is_connected {
                tracing::warn!("Connection lost in client mode, attempting reconnect...");

                // Emit disconnected event
                let _ = app_handle.emit(
                    "connection-status",
                    serde_json::json!({
                        "connected": false,
                        "reconnecting": true,
                    }),
                );

                // 尝试重连
                drop(bridge);
                // TODO: 实现重连逻辑 (需要重构 ClientBridge API)
                // 目前只发送状态事件

                let bridge = self.bridge.read().await;
                let mode_info = bridge.get_mode_info().await;

                let _ = app_handle.emit(
                    "connection-status",
                    serde_json::json!({
                        "connected": mode_info.is_connected,
                        "reconnecting": false,
                    }),
                );
            }
        }
    }
}
