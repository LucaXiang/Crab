-- Archive Verification Schema
-- 归档哈希链验证记录表（由 VerifyScheduler 自动写入）

DEFINE TABLE OVERWRITE archive_verification TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select WHERE $auth.role = role:admin OR $auth.id = employee:admin
        FOR create FULL
        FOR update, delete NONE;

-- 验证类型
DEFINE FIELD OVERWRITE verification_type ON archive_verification TYPE string
    ASSERT $value IN ["daily", "full"]
    PERMISSIONS FULL;

-- 营业日标签 (daily: "2026-01-29", full: None)
DEFINE FIELD OVERWRITE date ON archive_verification TYPE option<string>
    PERMISSIONS FULL;

-- 订单统计
DEFINE FIELD OVERWRITE total_orders ON archive_verification TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE verified_orders ON archive_verification TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

-- 链路完整性
DEFINE FIELD OVERWRITE chain_intact ON archive_verification TYPE bool
    DEFAULT true
    PERMISSIONS FULL;

-- 异常计数
DEFINE FIELD OVERWRITE chain_resets_count ON archive_verification TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE chain_breaks_count ON archive_verification TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE invalid_orders_count ON archive_verification TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

-- 有异常时存储完整验证结果 JSON
DEFINE FIELD OVERWRITE details ON archive_verification TYPE option<object>
    PERMISSIONS FULL;

-- 时间戳（自动生成，只读）
DEFINE FIELD OVERWRITE created_at ON archive_verification TYPE int READONLY
    DEFAULT 0
    PERMISSIONS FULL;

-- 索引
DEFINE INDEX OVERWRITE av_type_idx ON archive_verification FIELDS verification_type;
DEFINE INDEX OVERWRITE av_date_idx ON archive_verification FIELDS date;
DEFINE INDEX OVERWRITE av_created_idx ON archive_verification FIELDS created_at;
DEFINE INDEX OVERWRITE av_intact_idx ON archive_verification FIELDS chain_intact;
-- Fix #3: 去重约束（同类型+同日期只保留一条）
DEFINE INDEX OVERWRITE av_type_date_unique ON archive_verification FIELDS verification_type, date UNIQUE;
