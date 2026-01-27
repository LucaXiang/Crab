-- Employee Table Schema
-- 员工表，存储系统用户信息

DEFINE TABLE OVERWRITE employee TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE username ON employee TYPE string
    ASSERT string::len($value) > 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE hash_pass ON employee TYPE string
    ASSERT string::len($value) > 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE display_name ON employee TYPE string
    DEFAULT ""
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE role ON employee TYPE record<role>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE is_system ON employee TYPE bool DEFAULT false
    PERMISSIONS NONE;

DEFINE FIELD OVERWRITE is_active ON employee TYPE bool DEFAULT true
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE username_unique ON employee FIELDS username UNIQUE;
