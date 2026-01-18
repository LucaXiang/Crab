-- Tag Table Schema
-- 标签表，用于给菜品/规格打标签

DEFINE TABLE OVERWRITE tag TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE name ON tag TYPE string
    ASSERT string::len($value) > 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE color ON tag TYPE string
    DEFAULT "#3B82F6"
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE display_order ON tag TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE is_active ON tag TYPE bool
    DEFAULT true
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE tag_name_unique ON tag FIELDS name UNIQUE;
DEFINE INDEX OVERWRITE tag_display_order ON tag FIELDS display_order;
