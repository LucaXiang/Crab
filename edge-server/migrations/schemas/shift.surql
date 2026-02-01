-- Shift Schema
-- 班次管理表

DEFINE TABLE OVERWRITE shift TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

-- 操作员 ID（关联 employee 表）
DEFINE FIELD OVERWRITE operator_id ON shift TYPE record<employee>
    PERMISSIONS FULL;

-- 操作员姓名快照
DEFINE FIELD OVERWRITE operator_name ON shift TYPE string
    PERMISSIONS FULL;

-- 班次状态
DEFINE FIELD OVERWRITE status ON shift TYPE string
    DEFAULT "OPEN"
    ASSERT $value IN ["OPEN", "CLOSED"]
    PERMISSIONS FULL;

-- 开班时间 (Unix timestamp millis)
DEFINE FIELD OVERWRITE start_time ON shift TYPE int
    PERMISSIONS FULL;

-- 收班时间 (Unix timestamp millis)
DEFINE FIELD OVERWRITE end_time ON shift TYPE option<int>
    PERMISSIONS FULL;

-- 备用金 (开班时的现金准备金)
DEFINE FIELD OVERWRITE starting_cash ON shift TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

-- 预期现金 (starting_cash + 班次内现金收款)
DEFINE FIELD OVERWRITE expected_cash ON shift TYPE decimal
    DEFAULT 0
    PERMISSIONS FULL;

-- 实际现金 (收班时盘点)
DEFINE FIELD OVERWRITE actual_cash ON shift TYPE option<decimal>
    PERMISSIONS FULL;

-- 现金差异 (actual_cash - expected_cash)
DEFINE FIELD OVERWRITE cash_variance ON shift TYPE option<decimal>
    PERMISSIONS FULL;

-- 是否异常关闭 (断电/崩溃后自动恢复)
DEFINE FIELD OVERWRITE abnormal_close ON shift TYPE bool
    DEFAULT false
    PERMISSIONS FULL;

-- 最后活动时间 (心跳, Unix timestamp millis)
DEFINE FIELD OVERWRITE last_active_at ON shift TYPE option<int>
    PERMISSIONS FULL;

-- 备注
DEFINE FIELD OVERWRITE note ON shift TYPE option<string>
    PERMISSIONS FULL;

-- 时间戳
DEFINE FIELD OVERWRITE created_at ON shift TYPE option<int>
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE updated_at ON shift TYPE option<int>
    PERMISSIONS FULL;

-- 索引
DEFINE INDEX OVERWRITE shift_status_idx ON shift FIELDS status;
DEFINE INDEX OVERWRITE shift_operator_idx ON shift FIELDS operator_id;
DEFINE INDEX OVERWRITE shift_start_time_idx ON shift FIELDS start_time;
