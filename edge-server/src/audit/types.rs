//! 审计日志类型定义
//!
//! 税务级审计日志的核心数据结构。
//! 所有条目不可变、不可删除，支持 SHA256 哈希链防篡改。

use serde::{Deserialize, Serialize};

/// 审计操作类型（枚举，非自由文本）
///
/// 按领域分组，确保每个敏感操作都有明确的类型标识。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuditAction {
    // ═══ 系统生命周期 ═══
    /// 系统正常启动
    SystemStartup,
    /// 系统正常关闭
    SystemShutdown,
    /// 系统异常关闭（上次未正常关闭）
    SystemAbnormalShutdown,
    /// 系统长时间停机（>24h 未运行）
    SystemLongDowntime,
    /// 用户确认启动异常（前端 dialog 回应）
    AcknowledgeStartupIssue,

    // ═══ 认证 ═══
    /// 登录成功
    LoginSuccess,
    /// 登录失败
    LoginFailed,
    /// 登出
    Logout,

    // ═══ 订单（财务关键）═══
    /// 订单完成结账
    OrderCompleted,
    /// 订单作废
    OrderVoided,
    /// 添加支付
    OrderPaymentAdded,
    /// 取消支付
    OrderPaymentCancelled,
    /// 订单合并
    OrderMerged,
    /// 订单转移（换桌）
    OrderMoved,
    /// 订单拆分
    OrderSplit,
    /// 订单恢复（从作废恢复）
    OrderRestored,

    // ═══ 管理操作 ═══
    /// 员工创建
    EmployeeCreated,
    /// 员工更新
    EmployeeUpdated,
    /// 员工删除
    EmployeeDeleted,
    /// 角色创建
    RoleCreated,
    /// 角色更新
    RoleUpdated,
    /// 角色删除
    RoleDeleted,
    /// 商品价格变更
    ProductPriceChanged,
    /// 价格规则变更
    PriceRuleChanged,

    // ═══ 班次 ═══
    /// 班次开启
    ShiftOpened,
    /// 班次关闭
    ShiftClosed,

    // ═══ 系统配置 ═══
    /// 打印配置变更
    PrintConfigChanged,
    /// 门店信息变更
    StoreInfoChanged,
}

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// 审计日志条目（不可变）
///
/// 每条记录包含 SHA256 哈希链，确保防篡改。
/// - `prev_hash`: 前一条记录的哈希
/// - `curr_hash`: 当前记录的哈希（包含 prev_hash + 所有字段）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEntry {
    /// 全局递增序列号（唯一标识）
    pub id: u64,
    /// 时间戳（Unix 毫秒）
    pub timestamp: i64,
    /// 操作类型
    pub action: AuditAction,
    /// 资源类型（如 "order", "employee", "system"）
    pub resource_type: String,
    /// 资源 ID（如 "order:xxx", "employee:yyy"）
    pub resource_id: String,
    /// 操作人 ID（系统事件为 None）
    pub operator_id: Option<String>,
    /// 操作人名称
    pub operator_name: Option<String>,
    /// 结构化详情（JSON）
    pub details: serde_json::Value,
    /// 前一条审计日志哈希
    pub prev_hash: String,
    /// 当前记录哈希（SHA256）
    pub curr_hash: String,
}

/// 审计日志查询参数
#[derive(Debug, Clone, Deserialize)]
pub struct AuditQuery {
    /// 起始时间（Unix 毫秒，含）
    pub from: Option<i64>,
    /// 截止时间（Unix 毫秒，含）
    pub to: Option<i64>,
    /// 操作类型过滤
    pub action: Option<AuditAction>,
    /// 操作人 ID 过滤
    pub operator_id: Option<String>,
    /// 资源类型过滤
    pub resource_type: Option<String>,
    /// 分页偏移
    #[serde(default)]
    pub offset: usize,
    /// 分页大小（默认 50）
    #[serde(default = "default_limit")]
    pub limit: usize,
}

fn default_limit() -> usize {
    50
}

/// 审计日志列表响应
#[derive(Debug, Serialize)]
pub struct AuditListResponse {
    pub items: Vec<AuditEntry>,
    pub total: u64,
}

/// 审计链验证结果
#[derive(Debug, Serialize)]
pub struct AuditChainVerification {
    /// 验证的记录总数
    pub total_entries: u64,
    /// 链是否完整
    pub chain_intact: bool,
    /// 断裂点列表
    pub breaks: Vec<AuditChainBreak>,
}

/// 审计链断裂点
#[derive(Debug, Serialize)]
pub struct AuditChainBreak {
    /// 断裂处的序列号
    pub entry_id: u64,
    /// 期望的 prev_hash
    pub expected_prev_hash: String,
    /// 实际的 prev_hash
    pub actual_prev_hash: String,
}

/// 启动异常事件（需要前端用户确认）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupIssue {
    /// 对应审计日志的序列号
    pub sequence: u64,
    /// 异常类型
    pub action: AuditAction,
    /// 详情
    pub details: serde_json::Value,
    /// 时间戳
    pub timestamp: i64,
}

/// 前端确认启动异常请求
#[derive(Debug, Deserialize)]
pub struct AcknowledgeStartupRequest {
    /// 用户填写的原因（自由文本）
    pub reason: String,
}
