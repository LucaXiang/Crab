-- System State Schema
-- 系统状态表（哈希链状态缓存，单例）

DEFINE TABLE OVERWRITE system_state TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

-- 创世哈希
DEFINE FIELD OVERWRITE genesis_hash ON system_state TYPE option<string>
    PERMISSIONS FULL;

-- 最后订单
DEFINE FIELD OVERWRITE last_order ON system_state TYPE option<record<order>>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE last_order_hash ON system_state TYPE option<string>
    PERMISSIONS FULL;

-- 同步状态
DEFINE FIELD OVERWRITE synced_up_to ON system_state TYPE option<record<order>>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE synced_up_to_hash ON system_state TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE last_sync_time ON system_state TYPE option<int>
    PERMISSIONS FULL;

-- 统计
DEFINE FIELD OVERWRITE order_count ON system_state TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE created_at ON system_state TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE updated_at ON system_state TYPE int
    DEFAULT 0
    PERMISSIONS FULL;
