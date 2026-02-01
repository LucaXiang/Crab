-- Attribute Schema (Graph-oriented)
-- 属性表，选项直接嵌入，使用 RELATE 建立与产品/分类的关系

DEFINE TABLE OVERWRITE attribute TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE name ON attribute TYPE string
    ASSERT string::len($value) > 0
    PERMISSIONS FULL;

-- 选择模式
DEFINE FIELD OVERWRITE is_multi_select ON attribute TYPE bool
    DEFAULT false
    PERMISSIONS FULL;

-- 多选上限 (null = 无限)
DEFINE FIELD OVERWRITE max_selections ON attribute TYPE option<int>
    PERMISSIONS FULL;

-- 默认选项索引 (支持多选)
DEFINE FIELD OVERWRITE default_option_indices ON attribute TYPE option<array<int>>
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

-- 厨打设置
DEFINE FIELD OVERWRITE show_on_kitchen_print ON attribute TYPE bool
    DEFAULT false
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE kitchen_print_name ON attribute TYPE option<string>
    PERMISSIONS FULL;

-- 选项直接嵌入 (Graph DB 风格: 减少 JOIN)
-- options: [{ name: "微辣", price_modifier: 0, kitchen_print_name: null }, ...]
DEFINE FIELD OVERWRITE options ON attribute TYPE array<object>
    DEFAULT []
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE options.*.name ON attribute TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE options.*.price_modifier ON attribute TYPE float
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE options.*.display_order ON attribute TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE options.*.is_active ON attribute TYPE bool
    DEFAULT true
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE options.*.receipt_name ON attribute TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE options.*.kitchen_print_name ON attribute TYPE option<string>
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

-- 覆盖属性的默认选项索引 (支持多选)
DEFINE FIELD OVERWRITE default_option_indices ON has_attribute TYPE option<array<int>>
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE has_attr_unique ON has_attribute FIELDS in, out UNIQUE;
