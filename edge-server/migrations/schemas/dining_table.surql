-- Dining Table Schema
-- 桌台表

DEFINE TABLE OVERWRITE dining_table TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE name ON dining_table TYPE string
    ASSERT string::len($value) > 0
    PERMISSIONS FULL;

-- 区域关联
DEFINE FIELD OVERWRITE zone ON dining_table TYPE record<zone>
    PERMISSIONS FULL;

-- 座位数
DEFINE FIELD OVERWRITE capacity ON dining_table TYPE int
    DEFAULT 4
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE is_active ON dining_table TYPE bool
    DEFAULT true
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE table_zone ON dining_table FIELDS zone;
DEFINE INDEX OVERWRITE table_name_zone ON dining_table FIELDS zone, name UNIQUE;
