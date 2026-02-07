-- Print Destination Schema
-- 打印目的地（抽象层）

DEFINE TABLE OVERWRITE print_destination TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

DEFINE FIELD OVERWRITE name ON print_destination TYPE string
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE description ON print_destination TYPE option<string>
    PERMISSIONS FULL;

-- 嵌入式打印机配置
DEFINE FIELD OVERWRITE printers ON print_destination TYPE array<object>
    DEFAULT []
    PERMISSIONS FULL;

-- 打印机字段定义
DEFINE FIELD OVERWRITE printers.*.printer_type ON print_destination TYPE string PERMISSIONS FULL;
DEFINE FIELD OVERWRITE printers.*.ip ON print_destination TYPE option<string> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE printers.*.port ON print_destination TYPE option<int> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE printers.*.driver_name ON print_destination TYPE option<string> PERMISSIONS FULL;
DEFINE FIELD OVERWRITE printers.*.priority ON print_destination TYPE int DEFAULT 0 PERMISSIONS FULL;
DEFINE FIELD OVERWRITE printers.*.is_active ON print_destination TYPE bool DEFAULT true PERMISSIONS FULL;

DEFINE FIELD OVERWRITE is_active ON print_destination TYPE bool
    DEFAULT true
    PERMISSIONS FULL;

DEFINE INDEX OVERWRITE print_dest_name ON print_destination FIELDS name UNIQUE;
