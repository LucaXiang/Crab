-- Product Specification Table Schema
-- 产品规格表，支持标签关联

DEFINE TABLE OVERWRITE product_specification TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

-- 产品关联 (SurrealDB Record Link)
DEFINE FIELD OVERWRITE product ON product_specification TYPE record<product>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE name ON product_specification TYPE string
    ASSERT string::len($value) > 0
    PERMISSIONS FULL;

-- 价格 (单位: 分)
DEFINE FIELD OVERWRITE price ON product_specification TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE display_order ON product_specification TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

-- 是否默认规格
DEFINE FIELD OVERWRITE is_default ON product_specification TYPE bool
    DEFAULT false
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE is_active ON product_specification TYPE bool
    DEFAULT true
    PERMISSIONS FULL;

-- 是否根规格 (单规格产品的唯一规格)
DEFINE FIELD OVERWRITE is_root ON product_specification TYPE bool
    DEFAULT false
    PERMISSIONS FULL;

-- 外部系统ID (用于同步)
DEFINE FIELD OVERWRITE external_id ON product_specification TYPE option<int>
    PERMISSIONS FULL;

-- 标签关联 (SurrealDB 特性: 直接嵌入记录链接数组)
-- 多对多关系，无需中间表
DEFINE FIELD OVERWRITE tags ON product_specification TYPE array<record<tag>>
    DEFAULT []
    PERMISSIONS FULL;

-- 时间戳
DEFINE FIELD OVERWRITE created_at ON product_specification TYPE datetime
    DEFAULT time::now()
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE updated_at ON product_specification TYPE datetime
    DEFAULT time::now()
    PERMISSIONS FULL;

-- 索引
DEFINE INDEX OVERWRITE spec_product ON product_specification FIELDS product;
DEFINE INDEX OVERWRITE spec_product_name ON product_specification FIELDS product, name UNIQUE;
DEFINE INDEX OVERWRITE spec_display_order ON product_specification FIELDS display_order;
DEFINE INDEX OVERWRITE spec_active ON product_specification FIELDS product, is_active, display_order;
