-- Order Schema (Graph Model)
-- 归档订单使用图边关系，只存储核心数据

-- =============================================================================
-- order 表
-- =============================================================================

DEFINE TABLE OVERWRITE order TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE receipt_number ON order TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE zone_name ON order TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE table_name ON order TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE status ON order TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE is_retail ON order TYPE bool
    DEFAULT false
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE guest_count ON order TYPE option<int>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE original_total ON order TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE subtotal ON order TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE total_amount ON order TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE paid_amount ON order TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE discount_amount ON order TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE surcharge_amount ON order TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE comp_total_amount ON order TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE order_manual_discount_amount ON order TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE order_manual_surcharge_amount ON order TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE order_rule_discount_amount ON order TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE order_rule_surcharge_amount ON order TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE tax ON order TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE start_time ON order TYPE int
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE end_time ON order TYPE option<int>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE operator_id ON order TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE operator_name ON order TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE void_type ON order TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE loss_reason ON order TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE loss_amount ON order TYPE option<decimal>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE void_note ON order TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE related_order_id ON order TYPE option<record<order>>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE prev_hash ON order TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE curr_hash ON order TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE created_at ON order TYPE int
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE order_receipt ON order FIELDS receipt_number UNIQUE;
DEFINE INDEX OVERWRITE order_status ON order FIELDS status;
DEFINE INDEX OVERWRITE order_end_time ON order FIELDS end_time;
DEFINE INDEX OVERWRITE order_hash ON order FIELDS curr_hash;
DEFINE INDEX OVERWRITE order_status_end_time ON order FIELDS status, end_time;
DEFINE INDEX OVERWRITE order_created_at ON order FIELDS created_at;

-- =============================================================================
-- order_item 表
-- =============================================================================

DEFINE TABLE OVERWRITE order_item TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE spec ON order_item TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE instance_id ON order_item TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE name ON order_item TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE spec_name ON order_item TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE price ON order_item TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE quantity ON order_item TYPE int
    DEFAULT 1
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE unpaid_quantity ON order_item TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE unit_price ON order_item TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE line_total ON order_item TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE discount_amount ON order_item TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE surcharge_amount ON order_item TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE rule_discount_amount ON order_item TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE rule_surcharge_amount ON order_item TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE tax ON order_item TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE tax_rate ON order_item TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE category_name ON order_item TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE applied_rules ON order_item TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE note ON order_item TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE is_comped ON order_item TYPE bool
    DEFAULT false
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE order_item_spec ON order_item FIELDS spec;
DEFINE INDEX OVERWRITE order_item_instance ON order_item FIELDS instance_id;

-- =============================================================================
-- order_item_option 表
-- =============================================================================

DEFINE TABLE OVERWRITE order_item_option TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE attribute_name ON order_item_option TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE option_name ON order_item_option TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE price ON order_item_option TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

-- =============================================================================
-- order_payment 表
-- =============================================================================

DEFINE TABLE OVERWRITE order_payment TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE seq ON order_payment TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE payment_id ON order_payment TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE method ON order_payment TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE amount ON order_payment TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE time ON order_payment TYPE int
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE cancelled ON order_payment TYPE bool
    DEFAULT false
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE cancel_reason ON order_payment TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE split_type ON order_payment TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE split_items ON order_payment TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE aa_shares ON order_payment TYPE option<int>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE aa_total_shares ON order_payment TYPE option<int>
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE payment_method ON order_payment FIELDS method;
DEFINE INDEX OVERWRITE payment_time ON order_payment FIELDS time;

-- =============================================================================
-- order_event 表
-- =============================================================================

DEFINE TABLE OVERWRITE order_event TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE seq ON order_event TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE event_type ON order_event TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE timestamp ON order_event TYPE int
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE data ON order_event TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE prev_hash ON order_event TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE curr_hash ON order_event TYPE string
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE event_time ON order_event FIELDS timestamp;

-- =============================================================================
-- 图边关系
-- =============================================================================

DEFINE TABLE OVERWRITE has_item TYPE RELATION
    FROM order TO order_item SCHEMAFULL
    PERMISSIONS FOR select, create
        WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE INDEX OVERWRITE has_item_in ON has_item FIELDS in;

DEFINE TABLE OVERWRITE has_option TYPE RELATION
    FROM order_item TO order_item_option SCHEMAFULL
    PERMISSIONS FOR select, create
        WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE INDEX OVERWRITE has_option_in ON has_option FIELDS in;

DEFINE TABLE OVERWRITE has_payment TYPE RELATION
    FROM order TO order_payment SCHEMAFULL
    PERMISSIONS FOR select, create
        WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE INDEX OVERWRITE has_payment_in ON has_payment FIELDS in;

DEFINE TABLE OVERWRITE has_event TYPE RELATION
    FROM order TO order_event SCHEMAFULL
    PERMISSIONS FOR select, create
        WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE INDEX OVERWRITE has_event_in ON has_event FIELDS in;
