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

-- 是否多规格产品
DEFINE FIELD OVERWRITE has_multi_spec ON product TYPE bool
    DEFAULT false
    PERMISSIONS FULL;

-- 小票名称 (可选，默认使用 name)
DEFINE FIELD OVERWRITE receipt_name ON product TYPE option<string>
    PERMISSIONS FULL;

-- 厨房打印名称
DEFINE FIELD OVERWRITE kitchen_print_name ON product TYPE option<string>
    PERMISSIONS FULL;

-- 厨房打印机关联 (覆盖分类设置)
DEFINE FIELD OVERWRITE kitchen_printer ON product TYPE option<record<kitchen_printer>>
    PERMISSIONS FULL;

-- 打印设置: -1=继承分类, 0=禁用, 1=启用
DEFINE FIELD OVERWRITE is_kitchen_print_enabled ON product TYPE int
    DEFAULT -1
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE is_label_print_enabled ON product TYPE int
    DEFAULT -1
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE is_active ON product TYPE bool
    DEFAULT true
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE product_category ON product FIELDS category;
DEFINE INDEX OVERWRITE product_sort_order ON product FIELDS sort_order;
