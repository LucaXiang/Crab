-- Order Schema (Graph/Document hybrid)
-- 订单表，订单项嵌入，事件用图关系

DEFINE TABLE OVERWRITE order TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

-- 订单号 (唯一)
DEFINE FIELD OVERWRITE receipt_number ON order TYPE string
    ASSERT string::len($value) > 0
    PERMISSIONS FULL;

-- 区域/桌台 (快照，不使用 record link 因为需要历史记录)
DEFINE FIELD OVERWRITE zone_name ON order TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE table_name ON order TYPE option<string>
    PERMISSIONS FULL;

-- 状态: COMPLETED, VOID, MOVED, MERGED (归档订单，无 OPEN/ACTIVE)
DEFINE FIELD OVERWRITE status ON order TYPE string
    ASSERT $value IN ["COMPLETED", "VOID", "MOVED", "MERGED"]
    PERMISSIONS FULL;

-- 新增：关联订单（用于 MOVED/MERGED 场景）
DEFINE FIELD OVERWRITE related_order_id ON order TYPE option<record<order>>
    PERMISSIONS FULL;

-- 新增：操作员 ID
DEFINE FIELD OVERWRITE operator_id ON order TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE start_time ON order TYPE datetime
    DEFAULT time::now()
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE end_time ON order TYPE option<datetime>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE guest_count ON order TYPE option<int>
    PERMISSIONS FULL;

-- 金额 (单位: 元，如 10.50 = ¥10.50)
DEFINE FIELD OVERWRITE total_amount ON order TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE paid_amount ON order TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE discount_amount ON order TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE surcharge_amount ON order TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

-- 完整快照 JSON (用于详情查询，包含完整的 OrderSnapshot)
DEFINE FIELD OVERWRITE snapshot_json ON order TYPE string
    PERMISSIONS FULL;

-- 订单项 JSON (已废弃，保留兼容)
DEFINE FIELD OVERWRITE items_json ON order TYPE option<string>
    PERMISSIONS FULL;

-- 支付记录 JSON (已废弃，保留兼容)
DEFINE FIELD OVERWRITE payments_json ON order TYPE option<string>
    PERMISSIONS FULL;

-- 哈希链 (防篡改)
DEFINE FIELD OVERWRITE prev_hash ON order TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE curr_hash ON order TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE created_at ON order TYPE datetime
    DEFAULT time::now()
    PERMISSIONS FULL;

-- 索引
DEFINE INDEX OVERWRITE order_receipt ON order FIELDS receipt_number UNIQUE;
DEFINE INDEX OVERWRITE order_status ON order FIELDS status;
DEFINE INDEX OVERWRITE order_end_time ON order FIELDS end_time;
DEFINE INDEX OVERWRITE order_hash ON order FIELDS curr_hash;

-- =============================================================================
-- 订单事件 (Graph Edge)
-- =============================================================================
-- 使用: RELATE order:1->has_event->order_event:xxx
-- 查询: SELECT ->has_event->order_event FROM order:1 ORDER BY timestamp

DEFINE TABLE OVERWRITE order_event TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

-- 事件类型: CREATED, ITEM_ADDED, ITEM_REMOVED, PAID, VOID, etc.
DEFINE FIELD OVERWRITE event_type ON order_event TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE timestamp ON order_event TYPE datetime
    DEFAULT time::now()
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE data ON order_event TYPE option<object>
    PERMISSIONS FULL;

-- 哈希链
DEFINE FIELD OVERWRITE prev_hash ON order_event TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE curr_hash ON order_event TYPE string
    PERMISSIONS FULL;

-- 订单事件边关系
DEFINE TABLE OVERWRITE has_event TYPE RELATION
    FROM order
    TO order_event
    SCHEMAFULL
    PERMISSIONS
        FOR select, create
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE INDEX OVERWRITE event_time ON order_event FIELDS timestamp;
