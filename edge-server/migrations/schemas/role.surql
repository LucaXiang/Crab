-- Role Table Schema
-- 角色表，存储权限角色信息

DEFINE TABLE OVERWRITE role TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE name ON role TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE display_name ON role TYPE string
    DEFAULT ""
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE description ON role TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE permissions ON role TYPE array<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE is_system ON role TYPE bool DEFAULT false
    PERMISSIONS NONE;

DEFINE FIELD OVERWRITE is_active ON role TYPE bool DEFAULT true
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE role_name_unique ON role FIELDS name UNIQUE;
