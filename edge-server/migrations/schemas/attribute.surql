-- Attribute Schema (Graph-oriented)
-- 属性表，选项直接嵌入，使用 RELATE 建立与产品/分类的关系

DEFINE TABLE OVERWRITE attribute TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE name ON attribute TYPE string
    ASSERT string::len($value) > 0
    PERMISSIONS FULL;

-- 属性类型: single_select, multi_select
DEFINE FIELD OVERWRITE attr_type ON attribute TYPE string
    DEFAULT "single_select"
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE display_order ON attribute TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE is_active ON attribute TYPE bool
    DEFAULT true
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE show_on_receipt ON attribute TYPE bool
    DEFAULT false
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE receipt_name ON attribute TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE kitchen_printer ON attribute TYPE option<record<kitchen_printer>>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE is_global ON attribute TYPE bool
    DEFAULT false
    PERMISSIONS FULL;

-- 选项直接嵌入 (Graph DB 风格: 减少 JOIN)
-- options: [{ name: "微辣", price_modifier: 0, is_default: true }, ...]
DEFINE FIELD OVERWRITE options ON attribute TYPE array<object>
    DEFAULT []
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE attr_order ON attribute FIELDS display_order;

-- =============================================================================
-- 边关系: has_attribute (产品/分类 -> 属性)
-- =============================================================================
-- 使用: RELATE product:1->has_attribute->attribute:spicy SET is_required = true
-- 查询: SELECT ->has_attribute->attribute FROM product:1

DEFINE TABLE OVERWRITE has_attribute TYPE RELATION
    FROM product | category
    TO attribute
    SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE is_required ON has_attribute TYPE bool
    DEFAULT false
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE display_order ON has_attribute TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

-- 默认选项索引 (引用 options 数组的索引)
DEFINE FIELD OVERWRITE default_option_idx ON has_attribute TYPE option<int>
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE has_attr_unique ON has_attribute FIELDS in, out UNIQUE;
