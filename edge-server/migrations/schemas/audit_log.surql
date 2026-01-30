-- Audit Log Table Schema
-- 税务级审计日志表，SHA256 哈希链防篡改
-- Append-only: 不允许 update/delete

DEFINE TABLE OVERWRITE audit_log TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select WHERE $auth.role = role:admin OR $auth.id = employee:admin
        FOR create FULL
        FOR update, delete NONE;

-- 序列号（全局递增，哈希链标识）
DEFINE FIELD OVERWRITE sequence ON audit_log TYPE int
    PERMISSIONS FULL;

-- 时间戳（Unix 毫秒）
DEFINE FIELD OVERWRITE timestamp ON audit_log TYPE int
    PERMISSIONS FULL;

-- 操作类型（snake_case 字符串，与 Rust AuditAction 枚举对应）
DEFINE FIELD OVERWRITE action ON audit_log TYPE string
    PERMISSIONS FULL;

-- 资源类型（如 "system", "auth", "order", "employee"）
DEFINE FIELD OVERWRITE resource_type ON audit_log TYPE string
    PERMISSIONS FULL;

-- 资源 ID（如 "server:main", "employee:emp1"）
DEFINE FIELD OVERWRITE resource_id ON audit_log TYPE string
    PERMISSIONS FULL;

-- 操作人 ID（系统事件为 null）
DEFINE FIELD OVERWRITE operator_id ON audit_log TYPE option<string>
    PERMISSIONS FULL;

-- 操作人名称
DEFINE FIELD OVERWRITE operator_name ON audit_log TYPE option<string>
    PERMISSIONS FULL;

-- 结构化详情（任意 JSON 对象）
DEFINE FIELD OVERWRITE details ON audit_log FLEXIBLE TYPE object DEFAULT {}
    PERMISSIONS FULL;

-- SHA256 哈希链 — 前一条审计日志哈希
DEFINE FIELD OVERWRITE prev_hash ON audit_log TYPE string
    PERMISSIONS FULL;

-- SHA256 哈希链 — 当前记录哈希
DEFINE FIELD OVERWRITE curr_hash ON audit_log TYPE string
    PERMISSIONS FULL;

-- 索引
DEFINE INDEX OVERWRITE audit_sequence_idx ON audit_log FIELDS sequence UNIQUE;
DEFINE INDEX OVERWRITE audit_timestamp_idx ON audit_log FIELDS timestamp;
DEFINE INDEX OVERWRITE audit_action_idx ON audit_log FIELDS action;
DEFINE INDEX OVERWRITE audit_operator_idx ON audit_log FIELDS operator_id;
DEFINE INDEX OVERWRITE audit_resource_type_idx ON audit_log FIELDS resource_type;
