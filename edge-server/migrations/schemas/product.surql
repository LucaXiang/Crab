-- Product Table Schema
-- 产品表

DEFINE TABLE OVERWRITE product TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE name ON product TYPE string
    ASSERT string::len($value) > 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE image ON product TYPE string
    DEFAULT ""
    PERMISSIONS FULL;

-- 分类关联 (SurrealDB Record Link)
DEFINE FIELD OVERWRITE category ON product TYPE record<category>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE sort_order ON product TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

-- 税率 (百分比，如 10 表示 10%)
DEFINE FIELD OVERWRITE tax_rate ON product TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

-- 小票名称 (可选，默认使用 name)
DEFINE FIELD OVERWRITE receipt_name ON product TYPE option<string>
    PERMISSIONS FULL;

-- 厨房打印名称
DEFINE FIELD OVERWRITE kitchen_print_name ON product TYPE option<string>
    PERMISSIONS FULL;

-- 厨房打印目的地
DEFINE FIELD OVERWRITE kitchen_print_destinations ON product TYPE array<record<print_destination>>
    DEFAULT []
    PERMISSIONS FULL;

-- 标签打印目的地
DEFINE FIELD OVERWRITE label_print_destinations ON product TYPE array<record<print_destination>>
    DEFAULT []
    PERMISSIONS FULL;

-- 厨房打印启用状态 (-1=继承, 0=禁用, 1=启用)
DEFINE FIELD OVERWRITE is_kitchen_print_enabled ON product TYPE int
    DEFAULT -1
    PERMISSIONS FULL;

-- 标签打印启用状态 (-1=继承, 0=禁用, 1=启用)
DEFINE FIELD OVERWRITE is_label_print_enabled ON product TYPE int
    DEFAULT -1
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE is_active ON product TYPE bool
    DEFAULT true
    PERMISSIONS FULL;

-- 标签关联
DEFINE FIELD OVERWRITE tags ON product TYPE array<record<tag>>
    DEFAULT []
    PERMISSIONS FULL;

-- 嵌入式规格数组 (文档数据库风格)
DEFINE FIELD OVERWRITE specs ON product TYPE array<object>
    DEFAULT []
    PERMISSIONS FULL;

-- 规格对象字段定义
DEFINE FIELD OVERWRITE specs.*.name ON product TYPE string PERMISSIONS FULL;
DEFINE FIELD OVERWRITE specs.*.price ON product TYPE decimal DEFAULT 0 PERMISSIONS FULL;
DEFINE FIELD OVERWRITE specs.*.display_order ON product TYPE int DEFAULT 0 PERMISSIONS FULL;
DEFINE FIELD OVERWRITE specs.*.is_default ON product TYPE bool DEFAULT false PERMISSIONS FULL;
DEFINE FIELD OVERWRITE specs.*.is_active ON product TYPE bool DEFAULT true PERMISSIONS FULL;
DEFINE FIELD OVERWRITE specs.*.external_id ON product TYPE option<int> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE specs.*.receipt_name ON product TYPE option<string> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE specs.*.is_root ON product TYPE bool DEFAULT false PERMISSIONS FULL;

DEFINE INDEX OVERWRITE product_category ON product FIELDS category;
DEFINE INDEX OVERWRITE product_sort_order ON product FIELDS sort_order;
DEFINE INDEX OVERWRITE product_external_id ON product FIELDS specs[*].external_id;
