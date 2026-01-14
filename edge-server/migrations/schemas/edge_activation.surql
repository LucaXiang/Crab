-- Edge Activation Table Schema
-- 边缘服务器激活状态表

DEFINE TABLE OVERWRITE edge_activation TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select FULL
        FOR create, update WHERE $auth.role = role:admin OR $auth.id = employee:admin
        FOR delete NONE;

-- 激活状态
DEFINE FIELD OVERWRITE is_activated ON edge_activation TYPE bool DEFAULT false
    PERMISSIONS FULL;

-- 激活时间
DEFINE FIELD OVERWRITE activated_at ON edge_activation TYPE option<datetime>
    PERMISSIONS FULL;

-- 租户信息
DEFINE FIELD OVERWRITE tenant_id ON edge_activation TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE tenant_name ON edge_activation TYPE option<string>
    PERMISSIONS FULL;

-- 边缘节点标识
DEFINE FIELD OVERWRITE edge_id ON edge_activation TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE edge_name ON edge_activation TYPE option<string>
    PERMISSIONS FULL;

-- 证书信息（不存储证书内容，只存储元数据）
DEFINE FIELD OVERWRITE cert_fingerprint ON edge_activation TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE cert_expires_at ON edge_activation TYPE option<datetime>
    PERMISSIONS FULL;

-- 最后心跳时间
DEFINE FIELD OVERWRITE last_heartbeat ON edge_activation TYPE option<datetime>
    PERMISSIONS FULL;

-- 创建/更新时间
DEFINE FIELD OVERWRITE created_at ON edge_activation TYPE datetime VALUE time::now() READONLY
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE updated_at ON edge_activation TYPE datetime VALUE time::now()
    PERMISSIONS FULL;
