-- Category Table Schema
-- 产品分类表

DEFINE TABLE OVERWRITE category TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE name ON category TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE sort_order ON category TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

-- 打印目的地
DEFINE FIELD OVERWRITE print_destinations ON category TYPE array<record<print_destination>>
    DEFAULT []
    PERMISSIONS FULL;

-- 厨房打印开关
DEFINE FIELD OVERWRITE is_kitchen_print_enabled ON category TYPE bool
    DEFAULT true
    PERMISSIONS FULL;

-- 标签打印开关
DEFINE FIELD OVERWRITE is_label_print_enabled ON category TYPE bool
    DEFAULT true
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE is_active ON category TYPE bool
    DEFAULT true
    PERMISSIONS FULL;

-- 虚拟分类支持
DEFINE FIELD OVERWRITE is_virtual ON category TYPE bool
    DEFAULT false
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE tag_ids ON category TYPE array<record<tag>>
    DEFAULT []
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE match_mode ON category TYPE string
    DEFAULT 'any'
    PERMISSIONS FULL;

-- 是否在 POS 主页显示此分类
DEFINE FIELD OVERWRITE is_display ON category TYPE bool
    DEFAULT true
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE category_name_unique ON category FIELDS name UNIQUE;
DEFINE INDEX OVERWRITE category_sort_order ON category FIELDS sort_order;
