-- Image Reference Table Schema
-- 图片引用计数表，用于跟踪图片被哪些实体引用

DEFINE TABLE OVERWRITE image_ref TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

-- 图片哈希 (SHA256 内容哈希)
DEFINE FIELD OVERWRITE hash ON image_ref TYPE string
    PERMISSIONS FULL;

-- 引用实体类型: "product", "label_template"
DEFINE FIELD OVERWRITE entity_type ON image_ref TYPE string
    PERMISSIONS FULL;

-- 引用实体 ID (格式: "product:xxx" 或 "label_template:xxx")
DEFINE FIELD OVERWRITE entity_id ON image_ref TYPE string
    PERMISSIONS FULL;

-- 创建时间 (Unix millis)
DEFINE FIELD OVERWRITE created_at ON image_ref TYPE int
    DEFAULT time::unix(time::now()) * 1000
    PERMISSIONS FULL;

-- 唯一索引：同一实体对同一图片只记录一次
DEFINE INDEX OVERWRITE idx_image_ref_unique ON image_ref FIELDS hash, entity_type, entity_id UNIQUE;

-- 哈希索引：快速查找图片的所有引用
DEFINE INDEX OVERWRITE idx_image_ref_hash ON image_ref FIELDS hash;

-- 实体索引：快速查找实体的所有图片引用
DEFINE INDEX OVERWRITE idx_image_ref_entity ON image_ref FIELDS entity_type, entity_id;
