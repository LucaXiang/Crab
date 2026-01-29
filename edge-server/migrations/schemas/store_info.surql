-- Store Info Schema
-- 店铺信息表（单例，每个租户只有一条记录 store_info:main）

DEFINE TABLE OVERWRITE store_info TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

-- 店铺名称
DEFINE FIELD OVERWRITE name ON store_info TYPE string
    DEFAULT ""
    PERMISSIONS FULL;

-- 店铺地址
DEFINE FIELD OVERWRITE address ON store_info TYPE string
    DEFAULT ""
    PERMISSIONS FULL;

-- 税号 / 营业执照 (NIF)
DEFINE FIELD OVERWRITE nif ON store_info TYPE string
    DEFAULT ""
    PERMISSIONS FULL;

-- Logo URL
DEFINE FIELD OVERWRITE logo_url ON store_info TYPE option<string>
    PERMISSIONS FULL;

-- 联系电话
DEFINE FIELD OVERWRITE phone ON store_info TYPE option<string>
    PERMISSIONS FULL;

-- 电子邮箱
DEFINE FIELD OVERWRITE email ON store_info TYPE option<string>
    PERMISSIONS FULL;

-- 官方网站
DEFINE FIELD OVERWRITE website ON store_info TYPE option<string>
    PERMISSIONS FULL;

-- 营业日分界时间 (HH:MM 格式，如 "06:00")
-- 用于跨天班次判断和日结报告计算
-- 默认 "00:00" (午夜)，酒吧/夜店可设置为 "06:00"
DEFINE FIELD OVERWRITE business_day_cutoff ON store_info TYPE string
    DEFAULT "00:00"
    PERMISSIONS FULL;

-- 时间戳
DEFINE FIELD OVERWRITE created_at ON store_info TYPE option<datetime>
    DEFAULT time::now()
    PERMISSIONS FULL;

DEFINE FIELD OVERWRITE updated_at ON store_info TYPE option<datetime>
    DEFAULT time::now()
    PERMISSIONS FULL;
