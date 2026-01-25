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

-- =============================================================================
-- 订单项表 (规范化，用于数据分析)
-- =============================================================================
-- 关联: order->order_item (通过 order_id 字段)

DEFINE TABLE OVERWRITE order_item TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE order_id ON order_item TYPE record<order>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE spec ON order_item TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE name ON order_item TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE spec_name ON order_item TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE price ON order_item TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE quantity ON order_item TYPE int
    DEFAULT 1
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE discount_amount ON order_item TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE surcharge_amount ON order_item TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE unit_price ON order_item TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE line_total ON order_item TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE note ON order_item TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE created_at ON order_item TYPE datetime
    DEFAULT time::now()
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE order_item_order ON order_item FIELDS order_id;
DEFINE INDEX OVERWRITE order_item_spec ON order_item FIELDS spec;

-- =============================================================================
-- 订单项属性表 (规范化)
-- =============================================================================
-- 关联: order_item->order_item_attribute (通过 item_id 字段)

DEFINE TABLE OVERWRITE order_item_attribute TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE item_id ON order_item_attribute TYPE record<order_item>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE attr_id ON order_item_attribute TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE option_idx ON order_item_attribute TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE name ON order_item_attribute TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE price ON order_item_attribute TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE item_attr_item ON order_item_attribute FIELDS item_id;

-- =============================================================================
-- 支付记录表 (规范化)
-- =============================================================================
-- 关联: order->order_payment (通过 order_id 字段)

DEFINE TABLE OVERWRITE order_payment TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE order_id ON order_payment TYPE record<order>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE method ON order_payment TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE amount ON order_payment TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE time ON order_payment TYPE datetime
    DEFAULT time::now()
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE reference ON order_payment TYPE option<string>
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE payment_order ON order_payment FIELDS order_id;
DEFINE INDEX OVERWRITE payment_method ON order_payment FIELDS method;
DEFINE INDEX OVERWRITE payment_time ON order_payment FIELDS time;
