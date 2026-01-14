-- Audit Log Table Schema
-- 审计日志表，记录系统操作历史

DEFINE TABLE OVERWRITE audit_log TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select WHERE $auth.role = role:admin OR $auth.id = employee:admin
        FOR create FULL
        FOR update, delete NONE;

-- 操作类型
DEFINE FIELD OVERWRITE action ON audit_log TYPE string
    ASSERT $value IN [
        'login', 'logout', 'login_failed', 'token_refresh',
        'read', 'create', 'update', 'delete',
        'config_change', 'user_create', 'user_update', 'user_delete',
        'role_change', 'permission_change',
        'server_start', 'server_stop', 'backup_create', 'backup_restore',
        'unauthorized_access', 'invalid_token'
    ]
    PERMISSIONS FULL;

-- 操作资源
DEFINE FIELD OVERWRITE resource ON audit_log TYPE string
    PERMISSIONS FULL;

-- 是否成功
DEFINE FIELD OVERWRITE success ON audit_log TYPE bool DEFAULT true
    PERMISSIONS FULL;

-- 用户信息（可选）
DEFINE FIELD OVERWRITE user_id ON audit_log TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE username ON audit_log TYPE option<string>
    PERMISSIONS FULL;

-- 请求上下文
DEFINE FIELD OVERWRITE ip_address ON audit_log TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE user_agent ON audit_log TYPE option<string>
    PERMISSIONS FULL;

-- 详情
DEFINE FIELD OVERWRITE details ON audit_log TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE error ON audit_log TYPE option<string>
    PERMISSIONS FULL;

-- 时间戳（自动生成，只读）
DEFINE FIELD OVERWRITE created_at ON audit_log TYPE datetime VALUE time::now() READONLY
    PERMISSIONS FULL;

-- 索引
DEFINE INDEX OVERWRITE audit_action_idx ON audit_log FIELDS action;
DEFINE INDEX OVERWRITE audit_user_idx ON audit_log FIELDS user_id;
DEFINE INDEX OVERWRITE audit_resource_idx ON audit_log FIELDS resource;
DEFINE INDEX OVERWRITE audit_created_idx ON audit_log FIELDS created_at;
DEFINE INDEX OVERWRITE audit_success_idx ON audit_log FIELDS success;
