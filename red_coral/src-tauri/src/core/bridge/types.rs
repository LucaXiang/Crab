//! Bridge type definitions

use serde::{Deserialize, Serialize};

use crab_client::{Authenticated, Connected, CrabClient, Local, Remote};
use shared::app_state::{ActivationRequiredReason, SubscriptionBlockedInfo};

/// 运行模式类型 (公开枚举，仅 Server/Client)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModeType {
    Server,
    Client,
}

impl std::fmt::Display for ModeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModeType::Server => write!(f, "server"),
            ModeType::Client => write!(f, "client"),
        }
    }
}

/// 应用状态 (统一 Server/Client 模式)
///
/// 用于前端路由守卫和状态展示。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum AppState {
    // === 前置状态 (未选模式) ===
    /// 无租户 → 需要输入凭据
    NeedTenantLogin,

    /// 租户已验证 → 选模式
    TenantReady,

    // === Server 模式专属 ===
    /// Server: 需要激活 - 携带详细原因
    ServerNeedActivation {
        reason: ActivationRequiredReason,
        can_auto_recover: bool,
        recovery_hint: String,
    },

    /// Server: 订阅无效 - 携带详细信息
    ServerSubscriptionBlocked { info: SubscriptionBlockedInfo },

    /// Server: 服务器就绪，等待员工登录
    ServerReady,

    /// Server: 员工已登录
    ServerAuthenticated,

    // === Client 模式专属 ===
    /// Client: 需要激活 - 携带详细原因
    ClientNeedActivation {
        reason: ActivationRequiredReason,
        can_auto_recover: bool,
        recovery_hint: String,
    },

    /// Client: 有证书但连不上
    ClientDisconnected,

    /// Client: 已连接，等待员工登录
    ClientConnected,

    /// Client: 员工已登录
    ClientAuthenticated,
}

impl AppState {
    /// 是否可以访问 POS 主界面
    pub fn can_access_pos(&self) -> bool {
        matches!(
            self,
            AppState::ServerAuthenticated | AppState::ClientAuthenticated
        )
    }

    /// 是否需要员工登录
    pub fn needs_employee_login(&self) -> bool {
        matches!(self, AppState::ServerReady | AppState::ClientConnected)
    }

    /// 是否需要设置/激活
    pub fn needs_setup(&self) -> bool {
        matches!(
            self,
            AppState::NeedTenantLogin
                | AppState::TenantReady
                | AppState::ServerNeedActivation { .. }
                | AppState::ClientNeedActivation { .. }
                | AppState::ClientDisconnected
        )
    }

    /// 是否被订阅阻止
    pub fn is_subscription_blocked(&self) -> bool {
        matches!(self, AppState::ServerSubscriptionBlocked { .. })
    }
}

/// 模式信息 (用于前端显示)
#[derive(Debug, Clone, Serialize)]
pub struct ModeInfo {
    pub mode: Option<ModeType>,
    pub is_connected: bool,
    pub is_authenticated: bool,
    pub tenant_id: Option<String>,
    pub username: Option<String>,
}

// ============================================================================
// Internal state enums (pub(crate) for use within bridge module)
// ============================================================================

use edge_server::ServerState;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

/// Server 模式的客户端状态
pub(crate) enum LocalClientState {
    Connected(CrabClient<Local, Connected>),
    Authenticated(CrabClient<Local, Authenticated>),
}

/// Client 模式的客户端状态 (参考 message_client 示例)
pub(crate) enum RemoteClientState {
    Connected(CrabClient<Remote, Connected>),
    Authenticated(CrabClient<Remote, Authenticated>),
}

/// 客户端模式枚举
pub(crate) enum ClientMode {
    /// Server 模式: 本地运行 edge-server
    Server {
        server_state: Arc<ServerState>,
        client: Option<LocalClientState>,
        server_task: tokio::task::JoinHandle<()>,
        listener_task: Option<tokio::task::JoinHandle<()>>,
        shutdown_token: CancellationToken,
    },
    /// Client 模式: 连接远程 edge-server
    Client {
        client: Option<RemoteClientState>,
        edge_url: String,
        message_addr: String,
        shutdown_token: CancellationToken,
    },
    /// 未连接状态 (内部运行时状态)
    Disconnected,
}
