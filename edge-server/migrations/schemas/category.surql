-- Category Table Schema
-- 产品分类表

DEFINE TABLE OVERWRITE category TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE name ON category TYPE string
    ASSERT string::len($value) > 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE sort_order ON category TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

-- 厨房打印机关联 (SurrealDB Record Link)
DEFINE FIELD OVERWRITE kitchen_printer ON category TYPE option<record<kitchen_printer>>
    PERMISSIONS FULL;

-- 打印设置
DEFINE FIELD OVERWRITE is_kitchen_print_enabled ON category TYPE bool
    DEFAULT true
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE is_label_print_enabled ON category TYPE bool
    DEFAULT true
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE is_active ON category TYPE bool
    DEFAULT true
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE category_name_unique ON category FIELDS name UNIQUE;
DEFINE INDEX OVERWRITE category_sort_order ON category FIELDS sort_order;
