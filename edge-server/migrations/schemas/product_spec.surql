-- Product Spec Index Table
-- 扁平化存储 product specs 的 external_id，用于唯一性约束
-- 当创建/更新产品时同步维护

DEFINE TABLE OVERWRITE product_spec TYPE NORMAL SCHEMAFULL
    PERMISSIONS
        FOR select, create, update, delete
            WHERE $auth.role = role:admin OR $auth.id = employee:admin;

-- 关联的产品
DEFINE FIELD OVERWRITE product ON product_spec TYPE record<product>
    PERMISSIONS FULL;

-- 规格在 specs 数组中的索引
DEFINE FIELD OVERWRITE spec_index ON product_spec TYPE int
    DEFAULT 0
    PERMISSIONS FULL;

-- 外部 ID (用于 POS 系统集成)
DEFINE FIELD OVERWRITE external_id ON product_spec TYPE int
    PERMISSIONS FULL;

-- 唯一性约束：external_id 全局唯一
DEFINE INDEX OVERWRITE unique_external_id ON product_spec FIELDS external_id UNIQUE;

-- 产品索引：快速查找产品的所有 specs
DEFINE INDEX OVERWRITE product_spec_product ON product_spec FIELDS product;
