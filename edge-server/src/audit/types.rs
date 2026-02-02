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
    /// 用户回应系统问题（启动异常/远程通知）
    ResolveSystemIssue,

    // ═══ 认证 ═══
    /// 登录成功
    LoginSuccess,
    /// 登录失败
    LoginFailed,
    /// 登出
    Logout,

    // ═══ 订单（财务关键 — 仅终结状态，中间操作由 OrderEvents 事件溯源覆盖）═══
    /// 订单完成结账
    OrderCompleted,
    /// 订单作废
    OrderVoided,
    /// 订单合并
    OrderMerged,
    /// 订单转移（换桌）
    OrderMoved,

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

    // ═══ 班次 ═══
    /// 班次开启
    ShiftOpened,
    /// 班次关闭
    ShiftClosed,

    // ═══ 商品目录 ═══
    /// 商品创建
    ProductCreated,
    /// 商品更新
    ProductUpdated,
    /// 商品删除
    ProductDeleted,
    /// 分类创建
    CategoryCreated,
    /// 分类更新
    CategoryUpdated,
    /// 分类删除
    CategoryDeleted,
    /// 标签创建
    TagCreated,
    /// 标签更新
    TagUpdated,
    /// 标签删除
    TagDeleted,
    /// 属性创建
    AttributeCreated,
    /// 属性更新
    AttributeUpdated,
    /// 属性删除
    AttributeDeleted,

    // ═══ 价格规则 ═══
    /// 价格规则创建
    PriceRuleCreated,
    /// 价格规则更新
    PriceRuleUpdated,
    /// 价格规则删除
    PriceRuleDeleted,

    // ═══ 区域与桌台 ═══
    /// 区域创建
    ZoneCreated,
    /// 区域更新
    ZoneUpdated,
    /// 区域删除
    ZoneDeleted,
    /// 桌台创建
    TableCreated,
    /// 桌台更新
    TableUpdated,
    /// 桌台删除
    TableDeleted,

    // ═══ 打印 ═══
    /// 标签模板创建
    LabelTemplateCreated,
    /// 标签模板更新
    LabelTemplateUpdated,
    /// 标签模板删除
    LabelTemplateDeleted,
    /// 打印目的地创建
    PrintDestinationCreated,
    /// 打印目的地更新
    PrintDestinationUpdated,
    /// 打印目的地删除
    PrintDestinationDeleted,

    // ═══ 日结报告 ═══
    /// 日结报告生成
    DailyReportGenerated,

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
    /// 关联目标（可选，指向相关审计条目或资源，如 "system_issue:xxx"）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
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


