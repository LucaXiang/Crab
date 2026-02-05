-- Zone Schema
-- 区域表（如：大厅、露台、包厢）

DEFINE TABLE OVERWRITE zone TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE name ON zone TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE description ON zone TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE is_active ON zone TYPE bool
    DEFAULT true
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE zone_name ON zone FIELDS name;
