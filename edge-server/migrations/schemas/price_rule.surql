-- Price Rule Schema
-- 价格调整规则（折扣/附加费）

DEFINE TABLE OVERWRITE price_rule TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE name ON price_rule TYPE string
    ASSERT string::len($value) > 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE display_name ON price_rule TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE receipt_name ON price_rule TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE description ON price_rule TYPE option<string>
    PERMISSIONS FULL;

-- 规则类型: DISCOUNT, SURCHARGE
DEFINE FIELD OVERWRITE rule_type ON price_rule TYPE string
    ASSERT $value IN ["DISCOUNT", "SURCHARGE"]
    PERMISSIONS FULL;

-- 应用范围: GLOBAL, CATEGORY, TAG, PRODUCT
DEFINE FIELD OVERWRITE product_scope ON price_rule TYPE string
    ASSERT $value IN ["GLOBAL", "CATEGORY", "TAG", "PRODUCT"]
    PERMISSIONS FULL;

-- 目标 (根据 scope): record<category> | record<tag> | record<product> | null
DEFINE FIELD OVERWRITE target ON price_rule TYPE option<record>
    PERMISSIONS FULL;

-- 区域范围: -1=全部, 0=零售, >0=指定区域
DEFINE FIELD OVERWRITE zone_scope ON price_rule TYPE int
    DEFAULT -1
    PERMISSIONS FULL;

-- 调整类型: PERCENTAGE, FIXED_AMOUNT
DEFINE FIELD OVERWRITE adjustment_type ON price_rule TYPE string
    ASSERT $value IN ["PERCENTAGE", "FIXED_AMOUNT"]
    PERMISSIONS FULL;

-- 调整值 (百分比如30=30%, 固定金额单位:分)
DEFINE FIELD OVERWRITE adjustment_value ON price_rule TYPE int
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE priority ON price_rule TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE is_stackable ON price_rule TYPE bool
    DEFAULT true
    PERMISSIONS FULL;

-- 时间模式: ALWAYS, SCHEDULE, ONETIME
DEFINE FIELD OVERWRITE time_mode ON price_rule TYPE string
    DEFAULT "ALWAYS"
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE start_time ON price_rule TYPE option<datetime>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE end_time ON price_rule TYPE option<datetime>
    PERMISSIONS FULL;

-- 周期配置 (JSON)
DEFINE FIELD OVERWRITE schedule_config ON price_rule TYPE option<object>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE is_active ON price_rule TYPE bool
    DEFAULT true
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE created_by ON price_rule TYPE option<record<employee>>
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE rule_active ON price_rule FIELDS is_active, product_scope, time_mode;
