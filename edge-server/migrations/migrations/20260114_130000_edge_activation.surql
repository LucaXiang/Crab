-- Edge Activation Table
-- 边缘服务器激活状态表

DEFINE TABLE OVERWRITE edge_activation TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select FULL
        FOR create, update WHERE $auth.role = role:admin OR $auth.id = employee:admin
        FOR delete NONE;

DEFINE FIELD OVERWRITE is_activated ON edge_activation TYPE bool DEFAULT false
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE activated_at ON edge_activation TYPE option<datetime>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE tenant_id ON edge_activation TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE tenant_name ON edge_activation TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE edge_id ON edge_activation TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE edge_name ON edge_activation TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE cert_fingerprint ON edge_activation TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE cert_expires_at ON edge_activation TYPE option<datetime>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE last_heartbeat ON edge_activation TYPE option<datetime>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE created_at ON edge_activation TYPE datetime VALUE time::now() READONLY
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE updated_at ON edge_activation TYPE datetime VALUE time::now()
    PERMISSIONS FULL;

-- 初始化默认激活记录
UPDATE edge_activation:default CONTENT {
    is_activated: false,
};
