-- System Issue Schema
-- 系统问题表：本地异常检测和远程服务器推送的问题
-- 前端按协议渲染对话框（阻塞式/通知式）

DEFINE TABLE OVERWRITE system_issue TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update WHERE true
        FOR delete NONE;

-- 来源: "local" (本地检测) | "remote" (远程推送)
DEFINE FIELD OVERWRITE source ON system_issue TYPE string
    PERMISSIONS FULL;

-- 问题类型标识，同时作为前端 i18n key
-- 例: "abnormal_shutdown", "long_downtime", "subscription_expiring", "custom_notice"
DEFINE FIELD OVERWRITE kind ON system_issue TYPE string
    PERMISSIONS FULL;

-- 是否阻塞 POS 操作 (true=全屏遮罩不可关闭, false=可关闭通知)
DEFINE FIELD OVERWRITE blocking ON system_issue TYPE bool
    PERMISSIONS FULL;

-- 关联实体 (可选, "table:id" 格式, 如 "order:abc123")
DEFINE FIELD OVERWRITE target ON system_issue TYPE option<string>
    PERMISSIONS FULL;

-- i18n 动态插值参数 (如 {"days": "7", "hours": "48"})
DEFINE FIELD OVERWRITE params ON system_issue FLEXIBLE TYPE object DEFAULT {}
    PERMISSIONS FULL;

-- 远程广播用: 标题文本 (本地问题留空, 前端用 kind 查 i18n)
DEFINE FIELD OVERWRITE title ON system_issue TYPE option<string>
    PERMISSIONS FULL;

-- 远程广播用: 描述文本
DEFINE FIELD OVERWRITE description ON system_issue TYPE option<string>
    PERMISSIONS FULL;

-- 预定义选项 (远程广播直接填文本, 本地问题填 i18n key)
DEFINE FIELD OVERWRITE options ON system_issue TYPE array DEFAULT []
    PERMISSIONS FULL;

-- 状态: "pending" | "resolved"
DEFINE FIELD OVERWRITE status ON system_issue TYPE string
    DEFAULT "pending"
    PERMISSIONS FULL;

-- 用户选择的选项或输入的文本
DEFINE FIELD OVERWRITE response ON system_issue TYPE option<string>
    PERMISSIONS FULL;

-- 操作员 ID
DEFINE FIELD OVERWRITE resolved_by ON system_issue TYPE option<string>
    PERMISSIONS FULL;

-- 回应时间戳 (Unix 毫秒)
DEFINE FIELD OVERWRITE resolved_at ON system_issue TYPE option<int>
    PERMISSIONS FULL;

-- 创建时间戳 (Unix 毫秒)
DEFINE FIELD OVERWRITE created_at ON system_issue TYPE int
    PERMISSIONS FULL;

-- 索引
DEFINE INDEX OVERWRITE idx_system_issue_status ON system_issue FIELDS status;
DEFINE INDEX OVERWRITE idx_system_issue_kind ON system_issue FIELDS kind;
DEFINE INDEX OVERWRITE idx_system_issue_source ON system_issue FIELDS source;
