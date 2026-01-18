use serde::{Deserialize, Serialize};
use std::fmt;

// ==================== Notification Level ====================

/// 通知级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationLevel {
    /// 普通信息
    Info,
    /// 警告
    Warning,
    /// 错误
    Error,
    /// 严重错误
    Critical,
}

impl fmt::Display for NotificationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Info => write!(f, "info"),
            Self::Warning => write!(f, "warning"),
            Self::Error => write!(f, "error"),
            Self::Critical => write!(f, "critical"),
        }
    }
}

/// 通知分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NotificationCategory {
    /// 系统级通知
    System,
    /// 打印机相关
    Printer,
    /// 网络相关
    Network,
    /// 业务相关（如订单、支付）
    Business,
}

// ==================== Server Commands ====================

/// 服务器指令 - 上层服务器发给边缘端的指令
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command", content = "params")]
pub enum ServerCommand {
    /// 激活边缘服务器（接收证书和元数据）
    Activate {
        tenant_id: String,
        tenant_name: String,
        edge_id: String,
        edge_name: String,
        tenant_ca_pem: String,
        edge_cert_pem: String,
        edge_key_pem: String,
    },

    /// 更新服务器配置
    ConfigUpdate {
        key: String,
        value: serde_json::Value,
    },

    /// 远程重启
    Restart {
        delay_seconds: u32,
        reason: Option<String>,
    },

    /// 健康检查 Ping
    Ping,
}

// ==================== Payloads ====================

/// 握手载荷 (客户端 -> 边缘服务端)
///
/// 包含客户端的协议版本信息，用于服务端进行版本校验。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HandshakePayload {
    /// 协议版本
    pub version: u16,
    /// 客户端名称/标识
    pub client_name: Option<String>,
    /// 客户端版本
    pub client_version: Option<String>,
    /// 客户端唯一标识 (UUID)
    pub client_id: Option<String>,
}

/// 通知载荷 (服务端 -> 客户端)
///
/// 用于向用户展示系统状态、错误或业务提示。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationPayload {
    /// 标题
    pub title: String,
    /// 消息内容
    pub message: String,
    /// 通知级别
    pub level: NotificationLevel,
    /// 通知分类
    pub category: NotificationCategory,
    /// 附加数据 (JSON)
    pub data: Option<serde_json::Value>,
}

/// 服务器指令载荷 (上层服务器 -> 边缘服务端)
///
/// 包含具体的管理指令，如激活、配置更新等。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerCommandPayload {
    pub command: ServerCommand,
}

/// 请求指令载荷 (客户端 -> 边缘服务端)
///
/// 表示客户端发起的业务操作请求，通常需要服务端处理并返回结果（或产生副作用）。
///
/// # 示例
/// - `action`: "order.add_item"
/// - `params`: `{ "dish_id": "123", "quantity": 1 }`
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RequestCommandPayload {
    /// 操作标识 (例如: "order.add_item", "printer.test")
    pub action: String,
    /// 操作参数 (可选的 JSON 对象)
    pub params: Option<serde_json::Value>,
}

/// 同步信号载荷 (边缘服务端 -> 所有客户端)
///
/// 当某个资源发生变更时（由某个客户端请求触发，或服务端后台触发），
/// 服务端广播此信号，通知所有感兴趣的客户端刷新数据。
///
/// # 示例
/// - `resource`: "order"
/// - `version`: 42
/// - `action`: "updated"
/// - `id`: "order_123"
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncPayload {
    /// 资源类型 (例如: "order", "menu", "table")
    pub resource: String,
    /// 版本号 (用于前端判断是否需要全量刷新，差距 > 5 时触发)
    pub version: u64,
    /// 变更类型 (例如: "created", "updated", "deleted")
    pub action: String,
    /// 资源 ID (必填，每次 Sync 都应指定具体的实体 ID)
    pub id: String,
    /// 资源数据 (可选，deleted 时为 None)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// 通用响应载荷 (服务端 -> 客户端)
///
/// 用于响应 RequestCommand
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsePayload {
    /// 是否成功
    pub success: bool,
    /// 响应消息/错误描述
    pub message: String,
    /// 响应数据 (可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
    /// 错误代码 (可选, 仅在失败时有用)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_code: Option<String>,
}

// ==================== Convenience Constructors ====================

impl NotificationPayload {
    pub fn info(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            level: NotificationLevel::Info,
            category: NotificationCategory::System,
            data: None,
        }
    }

    pub fn warning(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            level: NotificationLevel::Warning,
            category: NotificationCategory::System,
            data: None,
        }
    }

    pub fn error(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            message: message.into(),
            level: NotificationLevel::Error,
            category: NotificationCategory::System,
            data: None,
        }
    }
}

impl ResponsePayload {
    pub fn success(message: impl Into<String>, data: Option<serde_json::Value>) -> Self {
        Self {
            success: true,
            message: message.into(),
            data,
            error_code: None,
        }
    }

    pub fn error(message: impl Into<String>, code: Option<String>) -> Self {
        Self {
            success: false,
            message: message.into(),
            data: None,
            error_code: code,
        }
    }
}
