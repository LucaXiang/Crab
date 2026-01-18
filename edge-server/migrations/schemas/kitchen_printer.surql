-- Kitchen Printer Table Schema
-- 厨房打印机表

DEFINE TABLE OVERWRITE kitchen_printer TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE name ON kitchen_printer TYPE string
    ASSERT string::len($value) > 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE printer_name ON kitchen_printer TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE description ON kitchen_printer TYPE option<string>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE is_active ON kitchen_printer TYPE bool
    DEFAULT true
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE printer_name_unique ON kitchen_printer FIELDS name UNIQUE;
